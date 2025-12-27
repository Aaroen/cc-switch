"""
统计服务模块

负责收集和管理代理服务的统计数据，包括请求统计、性能指标和错误日志
"""

import asyncio
import time
from collections import defaultdict, deque
from datetime import datetime
from typing import Optional, Tuple

# ===== 统计数据收集器 =====
# 全局统计数据（线程安全）
stats_lock = asyncio.Lock()
request_stats = {
    "total_requests": 0,
    "successful_requests": 0,
    "failed_requests": 0,
    "total_bytes_sent": 0,
    "total_bytes_received": 0,
    "start_time": time.time()
}

# 性能指标（最近的请求）
recent_requests = deque(maxlen=1000)  # 保存最近1000个请求的性能数据
error_logs = deque(maxlen=500)  # 保存最近500个错误

# 按路径分组的统计
path_stats = defaultdict(lambda: {
    "count": 0,
    "bytes": 0,
    "errors": 0,
    "avg_response_time": 0
})

# 按时间窗口的统计（用于图表）
time_window_stats = {
    "requests_per_minute": deque(maxlen=1440),  # 24小时的分钟数据
    "errors_per_minute": deque(maxlen=1440),
    "bytes_per_minute": deque(maxlen=1440)
}


async def record_request_start(path: str, method: str, bytes_sent: int) -> str:
    """记录请求开始，返回请求ID，并立即添加到 recent_requests"""
    request_id = f"{int(time.time() * 1000)}-{id(asyncio.current_task())}"
    current_time = time.time()

    async with stats_lock:
        request_stats["total_requests"] += 1
        request_stats["total_bytes_sent"] += bytes_sent
        path_stats[path]["count"] += 1
        path_stats[path]["bytes"] += bytes_sent

        # 立即添加到 recent_requests（状态为 pending）
        recent_requests.append({
            "request_id": request_id,
            "path": path,
            "method": method,
            "status": "pending",  # 标记为进行中
            "status_code": None,  # 尚未完成，无状态码
            "bytes": 0,
            "response_time": 0,
            "timestamp": current_time
        })

    return request_id


async def record_request_success(
    request_id: str,
    path: str,
    method: str,
    bytes_received: int,
    response_time: float,
    status_code: int
):
    """记录成功请求，更新已存在的记录"""
    async with stats_lock:
        existing_req = None
        for req in reversed(recent_requests):
            if req["request_id"] == request_id:
                existing_req = req
                break

        # 如果请求已被标记为超时或错误，直接跳过，避免成功与失败双计数
        if existing_req and (existing_req.get("status_code") == 504 or existing_req.get("error")):
            return

        request_stats["successful_requests"] += 1
        request_stats["total_bytes_received"] += bytes_received

        # 更新路径统计
        current_avg = path_stats[path]["avg_response_time"]
        count = path_stats[path]["count"]
        path_stats[path]["avg_response_time"] = (current_avg * (count - 1) + response_time) / count

        # 查找并更新 recent_requests 中的记录
        found = False
        if existing_req:
            existing_req["status"] = "completed"
            existing_req["status_code"] = status_code
            existing_req["bytes"] = bytes_received
            existing_req["response_time"] = response_time
            found = True

        # 如果没找到（可能被 deque 清除了），则添加新记录
        if not found:
            recent_requests.append({
                "request_id": request_id,
                "path": path,
                "method": method,
                "status": "completed",
                "status_code": status_code,
                "bytes": bytes_received,
                "response_time": response_time,
                "timestamp": time.time()
            })


async def record_request_error(
    request_id: str,
    path: str,
    method: str,
    error_msg: str,
    response_time: float = 0,
    response_content: str = None,
    status_code: int | None = None
):
    """记录请求错误，更新已存在的记录"""
    async with stats_lock:
        existing_req = None
        for req in reversed(recent_requests):
            if req["request_id"] == request_id:
                existing_req = req
                break

        # 如果请求已被标记为成功，直接跳过，避免成功与失败双计数
        if existing_req and existing_req.get("status_code") is not None and existing_req.get("status_code", 0) < 400 and not existing_req.get("error"):
            return

        request_stats["failed_requests"] += 1
        path_stats[path]["errors"] += 1

        # 记录错误日志
        error_logs.append({
            "request_id": request_id,
            "path": path,
            "error": error_msg,
            "status_code": status_code,
            "response_content": response_content,
            "timestamp": time.time(),
            "response_time": response_time
        })

        # 查找并更新 recent_requests 中的记录
        found = False
        if existing_req:
            existing_req["status"] = "completed"
            existing_req["status_code"] = status_code
            existing_req["error"] = error_msg
            existing_req["response_content"] = response_content
            existing_req["response_time"] = response_time
            found = True

        # 如果没找到，则添加新记录
        if not found:
            recent_requests.append({
                "request_id": request_id,
                "path": path,
                "method": method,
                "status": "completed",
                "status_code": status_code,
                "error": error_msg,
                "response_content": response_content,
                "response_time": response_time,
                "timestamp": time.time()
            })


async def update_time_window_stats():
    """更新时间窗口统计（每分钟调用一次）"""
    current_time = time.time()
    # 将当前时间戳向下取整到分钟级别（秒级时间戳）
    current_minute_timestamp = int(current_time // 60) * 60

    async with stats_lock:
        # 计算本分钟的请求数
        minute_requests = sum(1 for req in recent_requests
                            if req["timestamp"] > current_time - 60)
        minute_errors = sum(1 for req in recent_requests
                          if (
                              (req.get("status_code") is not None and req.get("status_code", 0) >= 400) or
                              req.get("status") == "error"
                          ) and req["timestamp"] > current_time - 60)
        minute_bytes = sum(req.get("bytes", 0) for req in recent_requests
                         if req["timestamp"] > current_time - 60)

        time_window_stats["requests_per_minute"].append({
            "time": current_minute_timestamp,  # 使用 Unix 时间戳（秒级）
            "count": minute_requests
        })
        time_window_stats["errors_per_minute"].append({
            "time": current_minute_timestamp,  # 使用 Unix 时间戳（秒级）
            "count": minute_errors
        })
        time_window_stats["bytes_per_minute"].append({
            "time": current_minute_timestamp,  # 使用 Unix 时间戳（秒级）
            "count": minute_bytes
        })


async def periodic_stats_update():
    """定期更新统计数据"""
    while True:
        try:
            await update_time_window_stats()
        except Exception as e:
            print(f"[Stats] Failed to update time window stats: {e}")

        # 每分钟更新一次
        await asyncio.sleep(60)


# ===== 工具函数 =====

def format_bytes(bytes_count: int) -> str:
    """格式化字节数为友好显示"""
    for unit in ['B', 'KB', 'MB', 'GB', 'TB']:
        if bytes_count < 1024.0:
            return f"{bytes_count:.1f} {unit}"
        bytes_count /= 1024.0
    return f"{bytes_count:.1f} PB"


def calculate_percentiles(values: list, percentiles: list = [50, 95, 99]) -> dict:
    """计算百分位数"""
    if not values:
        return {p: 0 for p in percentiles}

    sorted_values = sorted(values)
    n = len(sorted_values)
    return {p: sorted_values[int(p * n / 100)] for p in percentiles}


async def get_time_filtered_data(start_time: float = None, end_time: float = None) -> Tuple:
    """获取时间过滤的数据"""
    current_time = time.time()

    # 默认时间范围：最近1小时
    if not start_time:
        start_time = current_time - 3600
    if not end_time:
        end_time = current_time

    async with stats_lock:
        # 过滤最近的请求
        filtered_requests = [
            req for req in recent_requests
            if start_time <= req["timestamp"] <= end_time
        ]

        # 过滤错误日志
        filtered_errors = [
            error for error in error_logs
            if start_time <= error["timestamp"] <= end_time
        ]

        # 过滤时间窗口统计（time 字段现在是 Unix 时间戳）
        filtered_time_series = {
            "requests_per_minute": [
                data for data in time_window_stats["requests_per_minute"]
                if start_time <= data["time"] <= end_time
            ],
            "errors_per_minute": [
                data for data in time_window_stats["errors_per_minute"]
                if start_time <= data["time"] <= end_time
            ],
            "bytes_per_minute": [
                data for data in time_window_stats["bytes_per_minute"]
                if start_time <= data["time"] <= end_time
            ]
        }

    return filtered_requests, filtered_errors, filtered_time_series


async def cleanup_stale_requests():
    """清理超时的进行中请求（后台任务）"""
    TIMEOUT_SECONDS = 300  # 5 分钟超时

    while True:
        try:
            await asyncio.sleep(60)  # 每 60 秒检查一次

            current_time = time.time()
            stale_requests = []  # 收集超时请求，统一记录错误

            async with stats_lock:
                # 遍历并标记超时的 pending 请求
                for req in list(recent_requests):
                    if req.get("status") == "pending":
                        age = current_time - req["timestamp"]
                        if age > TIMEOUT_SECONDS:
                            stale_requests.append({
                                "request_id": req["request_id"],
                                "path": req["path"],
                                "method": req["method"],
                                "response_time": age
                            })

            for req in stale_requests:
                await record_request_error(
                    req["request_id"],
                    req["path"],
                    req["method"],
                    f"请求超时（{req['response_time']:.0f}秒）",
                    response_time=req["response_time"],
                    status_code=504
                )
                print(f"[Stats] Request {req['request_id']} timed out after {req['response_time']:.0f}s")

        except Exception as e:
            print(f"[Stats] Failed to cleanup stale requests: {e}")
