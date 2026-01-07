"""
AnyRouter 透明代理 - 主应用模块

基于 FastAPI 的轻量级透明 HTTP 代理服务
"""

from fastapi import FastAPI, Request, Response
from fastapi.responses import StreamingResponse, RedirectResponse
from contextlib import asynccontextmanager
from starlette.background import BackgroundTask
import httpx
import json
import os
import time
import asyncio
import logging

# 导入配置
from .config import (
    TARGET_BASE_URL,
    DEBUG_MODE,
    PORT,
    ENABLE_DASHBOARD,
    DASHBOARD_API_KEY,
    CUSTOM_HEADERS
)

# 导入统计服务
from .services.stats import (
    record_request_start,
    record_request_success,
    record_request_error,
    periodic_stats_update,
    cleanup_stale_requests
)

# 导入代理服务
from .services.proxy import (
    process_request_body,
    filter_response_headers,
    prepare_forward_headers
)

# 导入编码工具
from .utils.encoding import ensure_unicode

# 导入 Admin 路由
from .routers.admin import router as admin_router

# Shared HTTP client for connection pooling and proper lifecycle management
http_client: httpx.AsyncClient = None  # type: ignore

logger = logging.getLogger('claude_proxy')

def _env_flag(name: str, default: str = 'false') -> bool:
    return os.getenv(name, default).lower() in ('true', '1', 'yes', 'y', 'on')

# 默认尽量安静：只在 WARNING/ERROR 时输出；需要时可通过环境变量打开
LOG_REQUESTS = _env_flag('CLAUDE_PROXY_LOG_REQUESTS', 'false')
LOG_BODIES = _env_flag('CLAUDE_PROXY_LOG_BODIES', 'false')
LOG_DYNAMIC_TARGET = _env_flag('CLAUDE_PROXY_LOG_DYNAMIC_TARGET', 'false')


@asynccontextmanager
async def lifespan(_: FastAPI):
    """Manage application lifespan events"""
    global http_client

    # 初始化日志系统：默认 WARNING，DEBUG_MODE 时切到 DEBUG，也允许 LOG_LEVEL 覆盖
    env_level = os.getenv('LOG_LEVEL')
    if env_level:
        level = getattr(logging, env_level.upper(), logging.INFO)
    else:
        level = logging.DEBUG if DEBUG_MODE else logging.WARNING
    logging.basicConfig(
        level=level,
        format='[%(asctime)s %(levelname)s claude_proxy] %(message)s',
        datefmt='%Y-%m-%dT%H:%M:%S',
        force=True,
    )

    # 启动定时统计更新任务
    stats_task = asyncio.create_task(periodic_stats_update())

    # 启动超时请求清理任务
    cleanup_task = asyncio.create_task(cleanup_stale_requests())

    # 输出应用配置信息（只在 worker 进程启动时输出一次）
    logger.info('=' * 60)
    logger.info('Application Configuration:')
    logger.info('  Base URL: %s', TARGET_BASE_URL)
    logger.info('  Server Port: %s', PORT)
    logger.info('  Custom Headers: %s headers loaded', len(CUSTOM_HEADERS))
    if CUSTOM_HEADERS:
        logger.info('  Custom Headers Keys: %s', list(CUSTOM_HEADERS.keys()))
    logger.info('  Debug Mode: %s', DEBUG_MODE)
    logger.info('  Hot Reload: %s', DEBUG_MODE)
    logger.info('  Dashboard Enabled: %s', ENABLE_DASHBOARD)
    if ENABLE_DASHBOARD:
        logger.info('  Dashboard API Key Configured: %s', 'Yes' if DASHBOARD_API_KEY else 'No')
        if DASHBOARD_API_KEY:
            logger.info('  Dashboard Access: http://localhost:%s/admin', PORT)
    logger.info('=' * 60)

    # 读取代理配置
    http_proxy = os.getenv("HTTP_PROXY")
    https_proxy = os.getenv("HTTPS_PROXY")

    # 构建 mounts 配置（httpx 0.28.0+ 的新语法）
    mounts = {}

    if http_proxy:
        # 确保代理 URL 包含协议
        if "://" not in http_proxy:
            http_proxy = f"http://{http_proxy}"
        mounts["http://"] = httpx.AsyncHTTPTransport(proxy=http_proxy)
        logger.info("HTTP Proxy configured: %s", http_proxy)

    if https_proxy:
        # 注意：HTTPS 代理通常也使用 http:// 协议（这不是错误！）
        if "://" not in https_proxy:
            https_proxy = f"http://{https_proxy}"
        mounts["https://"] = httpx.AsyncHTTPTransport(proxy=https_proxy)
        logger.info("HTTPS Proxy configured: %s", https_proxy)

    try:
        # 使用新的 mounts 参数初始化客户端
        if mounts:
            http_client = httpx.AsyncClient(
                follow_redirects=False,
                timeout=60.0,
                mounts=mounts
            )
            logger.info("HTTP client initialized with proxy mounts: %s", list(mounts.keys()))
        else:
            http_client = httpx.AsyncClient(
                follow_redirects=False,
                timeout=60.0
            )
            logger.info("HTTP client initialized without proxy")
    except Exception as e:
        logger.exception("Failed to initialize HTTP client: %s", e)
        raise

    logger.info("=" * 60)

    yield

    # Shutdown: Close HTTP client and stop background tasks
    stats_task.cancel()
    cleanup_task.cancel()

    try:
        await stats_task
    except asyncio.CancelledError:
        pass

    try:
        await cleanup_task
    except asyncio.CancelledError:
        pass

    await http_client.aclose()


app = FastAPI(
    title="Anthropic Transparent Proxy",
    version="1.1",
    lifespan=lifespan
)

# 注册 Admin 路由
app.include_router(admin_router)


# ===== 健康检查端点 =====

@app.get("/health")
async def health_check():
    """
    健康检查端点，用于容器健康检查和服务状态监控
    不依赖上游服务，仅检查代理服务本身是否正常运行
    """
    return {
        "status": "healthy",
        "service": "anthropic-transparent-proxy"
    }


@app.api_route("/", methods=["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"])
async def root_redirect(request: Request):
    """
    根路径：浏览器访问时重定向到 /admin，API 访问保持代理行为
    """
    accept_header = request.headers.get("accept", "")
    wants_html = "text/html" in accept_header or "application/xhtml+xml" in accept_header

    if wants_html:
        return RedirectResponse(url="/admin", status_code=307)

    return await proxy("", request)


# ===== 主代理逻辑 =====

@app.api_route("/{path:path}", methods=["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD"])
async def proxy(path: str, request: Request):
    # 记录请求开始
    start_time = time.time()
    body = await request.body()

    # 跳过 Dashboard 相关路径的统计
    if not path.startswith("api/admin") and not path.startswith("admin"):
        request_id = await record_request_start(path, request.method, len(body))
    else:
        request_id = None

    # 从请求头读取动态目标 URL（用于 cc-switch 集成）
    dynamic_target = request.headers.get("x-target-base-url")  # 修复：使用正确的header名称
    if dynamic_target:
        # cc-switch 模式：使用动态目标
        base_url = dynamic_target.rstrip('/')
        if LOG_DYNAMIC_TARGET:
            logger.info("[Proxy] 使用动态目标: %s", base_url)
    else:
        # 独立模式：使用配置的目标
        base_url = TARGET_BASE_URL

    # 构造目标 URL
    query = request.url.query
    target_url = f"{base_url}/{path}"
    if query:
        target_url += f"?{query}"

    if LOG_REQUESTS:
        logger.info("[Proxy] Request: %s %s -> %s (body=%s bytes)", request.method, path, base_url, len(body))
    if LOG_BODIES and (DEBUG_MODE or LOG_REQUESTS):
        try:
            data = json.loads(body.decode('utf-8'))
            logger.debug("[Proxy] Original body (%s bytes): %s", len(body), json.dumps(data, ensure_ascii=False)[:4000])
        except (json.JSONDecodeError, UnicodeDecodeError) as e:
            logger.debug("[Proxy] Failed to parse JSON: %s", e)

    # 处理请求体（替换 system prompt）
    # 仅在路由为 /v1/messages 时执行处理
    if LOG_REQUESTS:
        logger.debug("[Proxy] Processing request for path: %s", path)
    if path == "v1/messages" or path == "v1/messages/":
        body = process_request_body(body)

    # 准备转发的请求头
    incoming_headers = list(request.headers.items())
    client_host = request.client.host if request.client else None

    # 读取动态 API Key（用于 cc-switch 集成）
    dynamic_api_key = request.headers.get("X-API-Key")

    forward_headers = prepare_forward_headers(incoming_headers, client_host, base_url, dynamic_api_key)

    # 发起上游请求并流式处理响应
    response_time = 0
    bytes_received = 0
    error_response_content = b""  # 新增：缓存错误响应内容（仅当状态码 >= 400 时）
    try:
        # 构建请求但不使用 context manager
        req = http_client.build_request(
            method=request.method,
            url=target_url,
            headers=forward_headers,
            content=body,
        )

        # 发送请求并开启流式模式 (不使用 async with)
        resp = await http_client.send(req, stream=True)

        # 过滤响应头
        response_headers = filter_response_headers(resp.headers.items())

        # 统计响应时间
        response_time = time.time() - start_time

        # 异步生成器:流式读取响应内容并统计字节数
        async def iter_response():
            nonlocal bytes_received
            nonlocal error_response_content
            try:
                async for chunk in resp.aiter_bytes():
                    bytes_received += len(chunk)
                    # 如果是错误响应，缓存内容（限制 50KB）
                    if resp.status_code >= 400 and len(error_response_content) < 50*1024:
                        error_response_content += chunk
                    yield chunk
            except Exception as e:
                # 优雅处理客户端断开连接
                logger.debug("[Stream Error] %s", e)
                # 静默处理,避免日志污染
            finally:
                # 确保资源被释放 (作为备份,主要由 BackgroundTask 处理)
                pass

        # 创建响应完成后的统计任务
        async def close_and_record():
            await resp.aclose()
            if request_id:
                if resp.status_code < 400:
                    await record_request_success(
                        request_id,
                        path,
                        request.method,
                        bytes_received,
                        response_time,
                        resp.status_code
                    )
                else:
                    # 使用缓存的响应内容
                    response_content = ensure_unicode(error_response_content) if error_response_content else None
                    err_content_len = len(error_response_content)
                    short = None
                    if response_content:
                        short = response_content[:200] + ("..." if len(response_content) > 200 else "")
                    logger.warning(
                        "[Proxy] Upstream error: %s %s -> %s status=%s resp_bytes=%s resp=%s",
                        request.method,
                        path,
                        base_url,
                        resp.status_code,
                        err_content_len,
                        short,
                    )

                    # 记录错误到统计服务
                    await record_request_error(
                        request_id,
                        path,
                        request.method,
                        f"HTTP {resp.status_code}: {resp.reason_phrase}",
                        response_time,
                        response_content,  # 新增参数
                        resp.status_code
                    )

        # 使用 BackgroundTask 在响应完成后关闭连接和记录统计
        return StreamingResponse(
            iter_response(),
            status_code=resp.status_code,
            headers=response_headers,
            background=BackgroundTask(close_and_record),
        )

    except httpx.RequestError as e:
        # 记录请求错误
        if request_id:
            await record_request_error(
                request_id,
                path,
                request.method,
                str(e),
                time.time() - start_time,
                None,
                502
            )
        logger.error("[Proxy] Upstream request failed: %s %s -> %s: %s", request.method, path, base_url, e)
        return Response(content=f"Upstream request failed: {e}", status_code=502)


if __name__ == "__main__":
    import uvicorn
    # 开发模式启用热重载，生产模式禁用（通过 DEBUG_MODE 环境变量控制）
    # 注意：使用模块路径而非文件路径，以支持相对导入
    uvicorn.run("backend.app:app", host="0.0.0.0", port=PORT, reload=DEBUG_MODE)
