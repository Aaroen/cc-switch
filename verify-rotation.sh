#!/bin/bash
# 验证CC-Switch Provider轮询功能

echo "==================================================="
echo "       Provider轮询功能验证工具"
echo "==================================================="
echo ""

# 1. 检查当前Provider配置
echo "[1] Provider配置检查"
echo "---------------------------------------------------"
PROVIDER_COUNT=$(cc-switch-cli list claude 2>/dev/null | grep -c "层级:0")
echo "✓ 层级0的Provider数量: $PROVIDER_COUNT"

QUEUE_COUNT=$(cc-switch-cli list claude 2>/dev/null | grep -c "\[队列\]")
echo "✓ 故障转移队列Provider数量: $QUEUE_COUNT"

CURRENT_PROVIDER=$(cc-switch-cli list claude 2>/dev/null | grep "\[当前\]" | awk -F' - ' '{print $1}' | xargs)
echo "✓ 当前Provider: $CURRENT_PROVIDER"
echo ""

# 2. 测试请求并观察Provider切换
echo "[2] 发送测试请求"
echo "---------------------------------------------------"
echo "准备发送3个测试请求，观察Provider使用情况..."
echo ""

# 清空日志
> /tmp/rust_proxy_test.log

# 捕获最近的日志
tail -f ~/.cc-switch/logs/rust_proxy.log > /tmp/rust_proxy_test.log 2>&1 &
TAIL_PID=$!

sleep 1

# 发送测试请求
echo "发送请求 1/3..."
curl -s -X POST http://127.0.0.1:15721/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: test-key" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 10,
    "messages": [{"role": "user", "content": "测试1"}]
  }' > /dev/null 2>&1

sleep 2

echo "发送请求 2/3..."
curl -s -X POST http://127.0.0.1:15721/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: test-key" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 10,
    "messages": [{"role": "user", "content": "测试2"}]
  }' > /dev/null 2>&1

sleep 2

echo "发送请求 3/3..."
curl -s -X POST http://127.0.0.1:15721/v1/messages \
  -H "Content-Type: application/json" \
  -H "x-api-key: test-key" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-sonnet-4-5-20250929",
    "max_tokens": 10,
    "messages": [{"role": "user", "content": "测试3"}]
  }' > /dev/null 2>&1

sleep 2

# 停止日志捕获
kill $TAIL_PID 2>/dev/null

echo ""
echo "[3] 分析请求日志"
echo "---------------------------------------------------"

# 提取使用的Provider
PROVIDERS_USED=$(grep "Provider:" /tmp/rust_proxy_test.log | awk '{print $4}' | sed 's/,$//' | sort | uniq)
PROVIDER_COUNT_USED=$(echo "$PROVIDERS_USED" | grep -c ".")

if [ $PROVIDER_COUNT_USED -eq 0 ]; then
    echo "⚠ 未检测到Provider使用记录"
    echo ""
    echo "可能的原因:"
    echo "  1. Rust代理服务未正常运行"
    echo "  2. 请求被拒绝或超时"
    echo "  3. 日志级别设置问题"
    echo ""
    echo "查看完整日志: tail -50 ~/.cc-switch/logs/rust_proxy.log"
else
    echo "✓ 检测到 $PROVIDER_COUNT_USED 个不同的Provider被使用:"
    echo ""
    echo "$PROVIDERS_USED" | while read provider; do
        COUNT=$(grep "Provider: $provider" /tmp/rust_proxy_test.log | wc -l)
        echo "  • $provider (使用${COUNT}次)"
    done
    echo ""

    if [ $PROVIDER_COUNT_USED -gt 1 ]; then
        echo "✅ Provider轮询功能正常工作"
        echo "   系统在多个Provider之间分配请求"
    else
        echo "⚠ 所有请求使用同一个Provider"
        echo "   可能原因:"
        echo "   - 只有1个Provider健康可用"
        echo "   - 熔断器阻止了其他Provider"
        echo "   - 轮询策略配置为固定Provider"
    fi
fi

echo ""
echo "[4] 故障转移测试建议"
echo "---------------------------------------------------"
echo "手动测试故障转移:"
echo ""
echo "1. 查看当前Provider:"
echo "   cc-switch-cli list claude | grep '\\[当前\\]'"
echo ""
echo "2. 模拟Provider失败（使用无效key）:"
echo "   # 修改当前Provider的API key为无效值"
echo "   # 然后发送请求，观察是否自动切换到下一个Provider"
echo ""
echo "3. 查看熔断器状态:"
echo "   tail -f ~/.cc-switch/logs/rust_proxy.log | grep -E 'Provider|熔断器|故障转移'"
echo ""
echo "4. 查看Provider健康状态:"
echo "   python3 << 'PYEOF'"
echo "   import sqlite3"
echo "   conn = sqlite3.connect('~/.cc-switch/cc-switch.db')"
echo "   cur = conn.cursor()"
echo "   cur.execute(\"SELECT provider_id, is_healthy FROM provider_health WHERE app_type='claude'\")"
echo "   for row in cur.fetchall():"
echo "       print(f'{row[0]}: {\"健康\" if row[1] else \"异常\"}')"
echo "   conn.close()"
echo "   PYEOF"
echo ""

# 清理临时文件
rm -f /tmp/rust_proxy_test.log

echo "==================================================="
echo "[验证完成]"
echo "==================================================="
echo ""
