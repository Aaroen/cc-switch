#!/bin/bash
# 诊断CC-Switch环境变量配置

echo "==================================================="
echo "       CC-Switch 环境变量诊断工具"
echo "==================================================="
echo ""

# 1. 当前Shell类型
echo "[1] Shell 环境"
echo "---------------------------------------------------"
echo "当前Shell: $SHELL"
echo "Shell PID: $$"
if [ -n "$TMUX" ]; then
    echo "运行环境: tmux 会话"
    echo "TMUX版本: $(tmux -V)"
else
    echo "运行环境: 普通终端"
fi
echo ""

# 2. 当前环境变量
echo "[2] 当前ANTHROPIC环境变量"
echo "---------------------------------------------------"
ANTHROPIC_VARS=$(env | grep ANTHROPIC | sort)
if [ -z "$ANTHROPIC_VARS" ]; then
    echo "  (未设置任何ANTHROPIC环境变量)"
else
    echo "$ANTHROPIC_VARS" | while read line; do
        echo "  $line"
    done
fi
echo ""

# 3. 检查冲突
echo "[3] 冲突检查"
echo "---------------------------------------------------"
HAS_AUTH_TOKEN=false
HAS_API_KEY=false

if [ -n "$ANTHROPIC_AUTH_TOKEN" ]; then
    echo "  ⚠ ANTHROPIC_AUTH_TOKEN 已设置: ${ANTHROPIC_AUTH_TOKEN:0:10}..."
    HAS_AUTH_TOKEN=true
fi

if [ -n "$ANTHROPIC_API_KEY" ]; then
    echo "  ✓ ANTHROPIC_API_KEY 已设置: ${ANTHROPIC_API_KEY:0:20}..."
    HAS_API_KEY=true
fi

if [ "$HAS_AUTH_TOKEN" = true ] && [ "$HAS_API_KEY" = true ]; then
    echo ""
    echo "  ❌ 检测到冲突: AUTH_TOKEN 和 API_KEY 同时存在"
    echo "     Claude CLI 会优先使用 AUTH_TOKEN 导致行为异常"
elif [ "$HAS_AUTH_TOKEN" = true ]; then
    echo ""
    echo "  ⚠ 只有 AUTH_TOKEN，缺少 API_KEY"
    echo "    双层架构需要 API_KEY（占位符即可）"
elif [ "$HAS_API_KEY" = true ]; then
    echo ""
    echo "  ✓ 配置正常: 只有 API_KEY"
else
    echo ""
    echo "  ❌ 缺少必要的环境变量"
fi
echo ""

# 4. 配置文件检查
echo "[4] 配置文件检查"
echo "---------------------------------------------------"
for file in ~/.bashrc ~/.bash_profile ~/.profile; do
    if [ -f "$file" ]; then
        echo "  检查: $file"

        # 检查未注释的AUTH_TOKEN
        AUTH_TOKEN_LINES=$(grep "ANTHROPIC_AUTH_TOKEN" "$file" | grep -v "^#" | grep -v "^[[:space:]]*#")
        if [ -n "$AUTH_TOKEN_LINES" ]; then
            echo "    ❌ 发现未注释的 AUTH_TOKEN:"
            echo "$AUTH_TOKEN_LINES" | sed 's/^/       /'
        fi

        # 检查未注释的API_KEY
        API_KEY_LINES=$(grep "ANTHROPIC_API_KEY" "$file" | grep -v "^#" | grep -v "^[[:space:]]*#")
        if [ -n "$API_KEY_LINES" ]; then
            echo "    ✓ API_KEY 配置:"
            echo "$API_KEY_LINES" | sed 's/^/       /'
        fi

        # 检查unset命令
        UNSET_LINES=$(grep "unset ANTHROPIC" "$file" | grep -v "^#" | grep -v "^[[:space:]]*#")
        if [ -n "$UNSET_LINES" ]; then
            echo "    ✓ 清理命令:"
            echo "$UNSET_LINES" | sed 's/^/       /'
        fi
        echo ""
    fi
done

# 5. tmux环境检查
if [ -n "$TMUX" ]; then
    echo "[5] tmux 全局环境"
    echo "---------------------------------------------------"
    TMUX_ANTHROPIC=$(tmux showenv -g 2>/dev/null | grep ANTHROPIC)
    if [ -z "$TMUX_ANTHROPIC" ]; then
        echo "  (tmux全局环境未设置ANTHROPIC变量)"
    else
        echo "$TMUX_ANTHROPIC" | while read line; do
            if [[ $line == -* ]]; then
                echo "  [已清除] ${line:1}"
            else
                echo "  $line"
            fi
        done
    fi
    echo ""
fi

# 6. 代理服务状态
echo "[6] 代理服务状态"
echo "---------------------------------------------------"
# Rust代理
if [ -f ~/.cc-switch/proxy.pid ]; then
    RUST_PID=$(cat ~/.cc-switch/proxy.pid)
    if ps -p "$RUST_PID" > /dev/null 2>&1; then
        echo "  ✓ Rust代理运行中 (PID: $RUST_PID, 127.0.0.1:15721)"
    else
        echo "  ✗ Rust代理未运行（PID文件存在但进程不存在）"
    fi
else
    echo "  ✗ Rust代理未运行"
fi

# Python代理
if pgrep -f "uvicorn.*backend.app.*--port 15722" > /dev/null; then
    PYTHON_PID=$(pgrep -f "uvicorn.*backend.app.*--port 15722")
    echo "  ✓ Python代理运行中 (PID: $PYTHON_PID, 127.0.0.1:15722)"
else
    echo "  ✗ Python代理未运行"
fi
echo ""

# 7. 诊断结论
echo "==================================================="
echo "[诊断结论]"
echo "==================================================="

if [ "$HAS_AUTH_TOKEN" = true ] && [ "$HAS_API_KEY" = true ]; then
    echo "❌ 环境变量冲突"
    echo ""
    echo "问题: ANTHROPIC_AUTH_TOKEN 和 ANTHROPIC_API_KEY 同时存在"
    echo ""
    echo "可能原因:"
    echo "  1. tmux会话继承了旧的环境变量"
    echo "  2. 配置文件中有未注释的export语句"
    echo "  3. 系统级环境变量设置"
    echo ""
    echo "解决方案:"
    if [ -n "$TMUX" ]; then
        echo "  在当前tmux窗口执行:"
        echo "    unset ANTHROPIC_AUTH_TOKEN"
        echo "    source ~/.bashrc"
        echo ""
        echo "  或在tmux中新建窗口（会加载新环境）:"
        echo "    Ctrl+b c"
    else
        echo "  在当前终端执行:"
        echo "    unset ANTHROPIC_AUTH_TOKEN"
        echo "    source ~/.bashrc"
    fi
elif [ "$HAS_API_KEY" = true ] && [ "$HAS_AUTH_TOKEN" = false ]; then
    echo "✓ 环境变量配置正确"
    echo ""
    echo "ANTHROPIC_API_KEY 已设置为占位符，双层架构正常工作"
    echo ""
    echo "请求链路:"
    echo "  Claude CLI → Rust(15721) → Python(15722) → anyrouter.top"
    echo "             ↑ Provider轮询/熔断器/故障转移"
else
    echo "⚠ 环境变量未正确配置"
    echo ""
    echo "缺少必要的 ANTHROPIC_API_KEY 环境变量"
    echo ""
    echo "解决方案:"
    echo "  执行: source ~/.bashrc"
fi

echo ""
echo "==================================================="
