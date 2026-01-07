"""
代理服务模块

负责处理 HTTP 请求和响应，包括请求头过滤、请求体处理和 System Prompt 替换
"""

import json
import logging
import os
from typing import Iterable
from urllib.parse import urlparse

from ..config import (
    HOP_BY_HOP_HEADERS,
    PRESERVE_HOST,
    SYSTEM_PROMPT_REPLACEMENT,
    SYSTEM_PROMPT_BLOCK_INSERT_IF_NOT_EXIST,
    CLAUDE_CODE_KEYWORD,
    CUSTOM_HEADERS,
    TARGET_BASE_URL,
    DEBUG_MODE,
)

logger = logging.getLogger('claude_proxy')

def _env_flag(name: str, default: str = 'false') -> bool:
    return os.getenv(name, default).lower() in ('true', '1', 'yes', 'y', 'on')

# 仅在需要时输出 System Replacement 细节，避免污染 claude_proxy.log
LOG_SYSTEM_REPLACEMENT = _env_flag('CLAUDE_PROXY_LOG_SYSTEM_REPLACEMENT', 'false') or DEBUG_MODE


def filter_request_headers(headers: Iterable[tuple]) -> dict:
    """
    过滤请求头，移除 hop-by-hop 头部和 Content-Length

    Args:
        headers: 原始请求头（可迭代的元组列表）

    Returns:
        dict: 过滤后的请求头字典
    """
    out = {}
    for k, v in headers:
        lk = k.lower()
        if lk in HOP_BY_HOP_HEADERS:
            continue
        if lk == "host" and not PRESERVE_HOST:
            continue
        # 移除 Content-Length，让 httpx 根据实际内容自动计算
        # 因为我们可能会修改请求体，导致长度改变
        if lk == "content-length":
            continue
        out[k] = v
    return out


def filter_response_headers(headers: Iterable[tuple]) -> dict:
    """
    过滤响应头，移除 hop-by-hop 头部和 Content-Length

    Args:
        headers: 原始响应头（可迭代的元组列表）

    Returns:
        dict: 过滤后的响应头字典
    """
    out = {}
    for k, v in headers:
        lk = k.lower()
        if lk in HOP_BY_HOP_HEADERS:
            continue
        # 移除 Content-Length，避免流式响应时长度不匹配
        # StreamingResponse 会自动处理传输编码
        if lk == "content-length":
            continue
        # httpx 会自动解压 gzip/deflate，去掉 Content-Encoding 避免客户端重复解压导致 ZlibError
        if lk == "content-encoding":
            continue
        out[k] = v
    return out


def process_request_body(body: bytes) -> bytes:
    """
    处理请求体,替换 system 数组中第一个元素的 text 内容

    注意：此函数仅在 proxy() 中处理 /v1/messages 路由时被调用
    其他路由（如 /v1/completions, /v1/models 等）跳过此处理

    Args:
        body: 原始请求体（bytes）

    Returns:
        处理后的请求体（bytes），如果无法处理则返回原始 body
    """
    # 如果未配置替换文本，直接返回原始 body
    if SYSTEM_PROMPT_REPLACEMENT is None:
        if LOG_SYSTEM_REPLACEMENT:
            logger.debug("[System Replacement] Not configured, keeping original body")
        # try:
        #     print(f"[System Replacement None] Original system[0].text: {json.loads(body.decode('utf-8'))['system'][0]['text']}")
        # except (json.JSONDecodeError, UnicodeDecodeError, KeyError, IndexError, TypeError) as e:
        #     print(f"[System Replacement None] Failed to parse or access system prompt: {e}")
        return body

    # 尝试解析 JSON
    try:
        data = json.loads(body.decode('utf-8'))
        if LOG_SYSTEM_REPLACEMENT:
            logger.debug("[System Replacement] Successfully parsed JSON body")
    except (json.JSONDecodeError, UnicodeDecodeError) as e:
        if LOG_SYSTEM_REPLACEMENT:
            logger.debug("[System Replacement] Failed to parse JSON: %s, keeping original body", e)
        return body

    # 检查 system 字段是否存在且为列表
    if "system" not in data:
        if LOG_SYSTEM_REPLACEMENT:
            logger.debug("[System Replacement] No 'system' field found, keeping original body")
        return body

    if not isinstance(data["system"], list):
        if LOG_SYSTEM_REPLACEMENT:
            logger.debug(
                "[System Replacement] 'system' field is not a list (type: %s), keeping original body",
                type(data["system"]),
            )
        return body

    if len(data["system"]) == 0:
        if LOG_SYSTEM_REPLACEMENT:
            logger.debug("[System Replacement] 'system' array is empty, keeping original body")
        return body

    # 获取第一个元素
    first_element = data["system"][0]

    # 检查第一个元素是否有 'text' 字段
    if not isinstance(first_element, dict) or "text" not in first_element:
        if LOG_SYSTEM_REPLACEMENT:
            logger.debug("[System Replacement] First element doesn't have 'text' field, keeping original body")
        return body

    # 记录原始内容
    original_text = first_element["text"]
    if LOG_SYSTEM_REPLACEMENT:
        preview = original_text[:100] + ("..." if len(original_text) > 100 else "")
        logger.debug("[System Replacement] Original system[0].text: %s", preview)

    # 判断是否启用插入模式
    if SYSTEM_PROMPT_BLOCK_INSERT_IF_NOT_EXIST:
        # 插入模式：检查是否包含关键字（忽略大小写）
        if CLAUDE_CODE_KEYWORD.lower() in original_text.lower():
            # 包含关键字：执行替换
            first_element["text"] = SYSTEM_PROMPT_REPLACEMENT
            if LOG_SYSTEM_REPLACEMENT:
                preview = SYSTEM_PROMPT_REPLACEMENT[:100] + ("..." if len(SYSTEM_PROMPT_REPLACEMENT) > 100 else "")
                logger.debug("[System Replacement] Found '%s', replacing with: %s", CLAUDE_CODE_KEYWORD, preview)
        else:
            # 不包含关键字：执行插入
            new_element = {
                "type": "text",
                "text": SYSTEM_PROMPT_REPLACEMENT,
                "cache_control": {
                    "type": "ephemeral"
                }
            }
            data["system"].insert(0, new_element)
            if LOG_SYSTEM_REPLACEMENT:
                preview = SYSTEM_PROMPT_REPLACEMENT[:100] + ("..." if len(SYSTEM_PROMPT_REPLACEMENT) > 100 else "")
                logger.debug("[System Replacement] '%s' not found, inserting at position 0: %s", CLAUDE_CODE_KEYWORD, preview)
                logger.debug("[System Replacement] Array length changed: %s -> %s", len(data["system"]) - 1, len(data["system"]))
    else:
        # 原始模式：直接替换
        first_element["text"] = SYSTEM_PROMPT_REPLACEMENT
        if LOG_SYSTEM_REPLACEMENT:
            preview = SYSTEM_PROMPT_REPLACEMENT[:100] + ("..." if len(SYSTEM_PROMPT_REPLACEMENT) > 100 else "")
            logger.debug("[System Replacement] Replaced with: %s", preview)

    if LOG_SYSTEM_REPLACEMENT:
        logger.debug(
            "[System Replacement] original_text == SYSTEM_PROMPT_REPLACEMENT: %s",
            SYSTEM_PROMPT_REPLACEMENT == original_text,
        )

    # 转换回 JSON bytes
    try:
        # 这里必须加 separators 压缩空格，我也不知道为什么有空格不行。。。
        modified_body = json.dumps(data, ensure_ascii=False, separators=(',', ':')).encode('utf-8')
        if LOG_SYSTEM_REPLACEMENT:
            logger.debug(
                "[System Replacement] Successfully modified body (original size: %s bytes, new size: %s bytes)",
                len(body),
                len(modified_body),
            )
        return modified_body
    except Exception as e:
        if LOG_SYSTEM_REPLACEMENT:
            logger.debug("[System Replacement] Failed to serialize modified JSON: %s, keeping original body", e)
        return body


def prepare_forward_headers(incoming_headers: Iterable[tuple], client_host: str = None, target_url: str = None, api_key: str = None) -> dict:
    """
    准备转发的请求头

    Args:
        incoming_headers: 原始请求头
        client_host: 客户端 IP 地址
        target_url: 目标 URL（用于设置 Host）
        api_key: API Key（用于认证）

    Returns:
        dict: 准备好的转发请求头
    """
    # 复制并过滤请求头
    forward_headers = filter_request_headers(incoming_headers)

    # 设置 Host
    if not PRESERVE_HOST:
        # 优先使用动态目标 URL，否则使用配置的 TARGET_BASE_URL
        base_url = target_url if target_url else TARGET_BASE_URL
        parsed = urlparse(base_url)
        forward_headers["Host"] = parsed.netloc

    # 注入 API Key（如果提供）
    if api_key:
        # 检查 API Key 格式，决定使用哪个头部
        if api_key.startswith("sk-ant-"):
            # Anthropic API Key
            forward_headers["x-api-key"] = api_key
        elif api_key.startswith("Bearer "):
            # Bearer Token
            forward_headers["authorization"] = api_key
        else:
            # 默认当作 x-api-key
            forward_headers["x-api-key"] = api_key

    # 注入自定义 Header
    for k, v in CUSTOM_HEADERS.items():
        forward_headers[k] = v

    # 添加 X-Forwarded-For
    if client_host:
        existing = forward_headers.get("X-Forwarded-For")
        forward_headers["X-Forwarded-For"] = f"{existing}, {client_host}" if existing else client_host

    return forward_headers
