#!/usr/bin/env python3
"""
测试 encoding 模块的功能
"""

import sys
import os

# 将 backend 目录添加到路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from backend.utils.encoding import ensure_unicode

def test_encoding_conversion():
    """测试编码转换功能"""
    print("=" * 60)
    print("测试 encoding 模块...")
    print("=" * 60)

    # 测试 1: 正常字符串
    print("\n测试 1: 正常字符串")
    result = ensure_unicode("Hello World")
    print(f"✓ 输入: 'Hello World'")
    print(f"  输出: {result}")
    assert result == "Hello World"

    # 测试 2: 字节串（UTF-8 编码的中文字符）
    print("\n测试 2: UTF-8 字节串（中文字符）")
    original = "当前模型"
    byte_content = original.encode('utf-8')
    print(f"✓ 输入: {byte_content}")
    result = ensure_unicode(byte_content)
    print(f"  输出: {result}")
    assert result == original

    # 测试 3: None 值
    print("\n测试 3: None 值")
    result = ensure_unicode(None)
    print(f"✓ 输入: None")
    print(f"  输出: {result}")
    assert result is None

    # 测试 4: 长度限制
    print("\n测试 4: 长度限制（50KB）")
    long_text = "A" * 100000  # 100KB
    result = ensure_unicode(long_text, max_length=50*1024)  # 50KB
    print(f"✓ 输入: 100KB 文本")
    print(f"  输出长度: {len(result)} 字符")
    assert "截断" in result
    assert len(result) < 100000  # 应该被截断

    # 测试 5: 包含特殊字符的字节串
    print("\n测试 5: 包含特殊字符的字节串")
    mixed_content = "Error: 当前模型不可用\nModel: claude-3-5-sonnet"
    byte_content = mixed_content.encode('utf-8')
    result = ensure_unicode(byte_content)
    print(f"✓ 输入: {byte_content}")
    print(f"  输出: {result}")
    assert "当前模型不可用" in result

    # 测试 6: JSON 格式的错误响应
    print("\n测试 6: JSON 格式的错误响应")
    json_error = '{"error": {"type": "invalid_request", "message": "当前模型无法访问"}}'
    byte_content = json_error.encode('utf-8')
    result = ensure_unicode(byte_content)
    print(f"✓ 输入: {json_error}")
    print(f"  输出: {result}")
    assert "当前模型无法访问" in result

    print("\n" + "=" * 60)
    print("✓ 所有 encoding 测试通过！")
    print("=" * 60)
    return 0

if __name__ == "__main__":
    try:
        sys.exit(test_encoding_conversion())
    except Exception as e:
        print(f"\n✗ 测试失败: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
