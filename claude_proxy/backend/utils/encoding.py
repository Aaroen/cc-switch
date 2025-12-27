"""
编码处理工具模块

提供文本编码转换和长度限制功能
"""

import codecs


def ensure_unicode(text, max_length=50*1024):
    """
    确保文本是 Unicode 字符串，处理字节串和转义序列

    参数:
        text: 输入文本（可以是 str 或 bytes）
        max_length: 最大长度限制（默认 50KB）

    返回:
        转换后的 Unicode 字符串，或 None（如果输入为 None）
    """
    if text is None:
        return None

    # 如果是字节对象，直接解码
    if isinstance(text, bytes):
        try:
            # 使用 UTF-8 解码，错误的字节用替换字符代替
            decoded = text.decode('utf-8', errors='replace')
        except Exception:
            # 如果 UTF-8 解码失败，尝试使用 latin-1（单字节编码，不会出错）
            decoded = text.decode('latin-1', errors='replace')
    else:
        # 如果已经是字符串，直接转换为 str（确保不是其他类型）
        decoded = str(text)

    # 限制长度
    if len(decoded) > max_length:
        # 截断并添加提示
        truncated_length = max_length - 100  # 留出空间给提示信息
        if truncated_length > 0:
            decoded = decoded[:truncated_length] + f"\n\n[响应内容已截断，显示前 {truncated_length} 字符，总计约 {len(decoded)} 字符]"
        else:
            decoded = f"[响应内容长度 {len(decoded)} 超过限制 {max_length}]"

    return decoded
