#!/bin/bash
# CC-Switch CLI 快速安装脚本

echo "=== CC-Switch CLI 快速安装 ==="
echo ""

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# 创建日志目录
LOG_DIR="$HOME/.cc-switch/logs"
mkdir -p "$LOG_DIR"

# 清理 /tmp 中的旧日志文件
echo "清理旧日志文件..."
rm -f /tmp/cc-switch*.log 2>/dev/null || true
echo -e "${GREEN}✓${NC} 旧日志文件已清理"
echo ""

# 1. 检查 Rust
echo -e "${YELLOW}[1/8]${NC} 检查 Rust 工具链..."
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}✗${NC} Rust 未安装"
    echo ""
    echo "请运行以下命令安装 Rust:"
    echo "  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "  source ~/.cargo/env"
    echo ""
    echo "安装完成后重新运行本脚本"
    exit 1
fi
echo -e "${GREEN}✓${NC} Rust 已安装: $(rustc --version)"

# 2. 检查 Python 环境
echo -e "${YELLOW}[2/8]${NC} 检查 Python 环境..."
PYTHON_CMD=""
if command -v python3 &> /dev/null; then
    PYTHON_CMD="python3"
elif command -v python &> /dev/null; then
    PYTHON_VERSION=$(python --version 2>&1 | grep -oP '\d+\.\d+')
    if [[ $(echo "$PYTHON_VERSION >= 3.8" | bc -l) -eq 1 ]]; then
        PYTHON_CMD="python"
    fi
fi

if [ -z "$PYTHON_CMD" ]; then
    echo -e "${RED}✗${NC} Python 3.8+ 未安装"
    echo ""
    echo "请安装 Python 3.8 或更高版本:"
    echo "  Ubuntu/Debian: sudo apt-get install python3 python3-venv python3-pip"
    echo "  CentOS/RHEL: sudo yum install python3 python3-pip"
    echo "  macOS: brew install python@3"
    echo ""
    exit 1
fi

PYTHON_VERSION=$($PYTHON_CMD --version 2>&1)
echo -e "${GREEN}✓${NC} Python 已安装: $PYTHON_VERSION"

# 3. 安装 Python 依赖
echo -e "${YELLOW}[3/8]${NC} 安装 Python 依赖..."
SCRIPT_DIR="$(dirname "$0")"
REQUIREMENTS_FILE="$SCRIPT_DIR/claude_proxy/backend/requirements.txt"

if [ ! -f "$REQUIREMENTS_FILE" ]; then
    echo -e "${RED}✗${NC} 依赖文件不存在: $REQUIREMENTS_FILE"
    exit 1
fi

# 检查是否需要安装依赖
if ! $PYTHON_CMD -c "import fastapi" &> /dev/null; then
    $PYTHON_CMD -m pip install -q --upgrade pip --user
    $PYTHON_CMD -m pip install -q -r "$REQUIREMENTS_FILE" --user || {
        echo -e "${RED}✗${NC} Python 依赖安装失败"
        exit 1
    }
else
    $PYTHON_CMD -m pip install -q -r "$REQUIREMENTS_FILE" --upgrade --user 2>/dev/null || true
fi

echo -e "${GREEN}✓${NC} Python 依赖就绪"

# 4. 进入项目目录
echo -e "${YELLOW}[4/8]${NC} 进入项目目录..."
cd "$SCRIPT_DIR/src-tauri" || exit 1
echo -e "${GREEN}✓${NC} 当前目录: $(pwd)"

# 5. 编译 CLI 工具
echo -e "${YELLOW}[5/8]${NC} 编译 CLI 工具..."

# 检查是否需要重新编译
NEED_REBUILD=false
CLI_BINARY="target/release/cc-switch-cli"

if [ ! -f "$CLI_BINARY" ]; then
    NEED_REBUILD=true
else
    # 检查源文件是否比编译文件新
    if [ -n "$(find src -name "*.rs" -newer "$CLI_BINARY" 2>/dev/null)" ]; then
        NEED_REBUILD=true
    elif [ -f "Cargo.toml" ] && [ "Cargo.toml" -nt "$CLI_BINARY" ]; then
        NEED_REBUILD=true
    fi
fi

# 执行编译
if [ "$NEED_REBUILD" = true ]; then
    if cargo build --release --bin cc-switch-cli > "$LOG_DIR/build.log" 2>&1; then
        if grep -q "^error" "$LOG_DIR/build.log"; then
            echo -e "${RED}✗${NC} 编译失败，查看日志: cat $LOG_DIR/build.log"
            exit 1
        fi
    else
        echo -e "${RED}✗${NC} 编译失败，查看日志: cat $LOG_DIR/build.log"
        exit 1
    fi

    if [ ! -f "$CLI_BINARY" ]; then
        echo -e "${RED}✗${NC} 编译失败，未生成二进制文件"
        exit 1
    fi

    echo -e "${GREEN}✓${NC} 编译成功"
else
    echo -e "${GREEN}✓${NC} 使用现有编译文件"
fi

CLI_PATH=$(realpath "$CLI_BINARY")

# 4. 停止旧进程（在安装之前）
echo -e "${YELLOW}[6/8]${NC} 停止旧的代理进程..."

# 检查并停止Python代理服务
if pgrep -f "uvicorn.*backend.app" > /dev/null; then
    pkill -f "uvicorn.*backend.app" 2>/dev/null || true
    sleep 2
    if pgrep -f "uvicorn.*backend.app" > /dev/null; then
        pkill -9 -f "uvicorn.*backend.app" 2>/dev/null || true
        sleep 1
    fi
fi

# 检查并停止CLI代理服务
if pgrep -f "cc-switch-cli proxy" > /dev/null; then
    pkill -f "cc-switch-cli proxy" 2>/dev/null || true
    sleep 2
    if pgrep -f "cc-switch-cli proxy" > /dev/null; then
        pkill -9 -f "cc-switch-cli proxy" 2>/dev/null || true
        sleep 1
    fi
fi

# 清理 PID 文件
PID_FILE="$HOME/.cc-switch/proxy.pid"
rm -f "$PID_FILE" 2>/dev/null

echo -e "${GREEN}✓${NC} 旧进程已停止"
echo ""

# 7. 安装到系统路径
echo -e "${YELLOW}[7/8]${NC} 安装到系统路径..."
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp "$CLI_PATH" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/cc-switch-cli"

# 检查 PATH
if [[ ":$PATH:" != *":$INSTALL_DIR:"* ]]; then
    echo -e "${YELLOW}⚠${NC} ~/.local/bin 不在 PATH 中，添加: export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

echo -e "${GREEN}✓${NC} 安装完成"
echo ""

# 6. 启动Python代理服务器
echo -e "${YELLOW}[8/8]${NC} 启动Python代理服务器..."

# 启动Python代理服务（使用nohup后台运行）
cd ../claude_proxy || exit 1
nohup env HTTP_PROXY="http://127.0.0.1:7890" HTTPS_PROXY="http://127.0.0.1:7890" \
    $PYTHON_CMD -m uvicorn backend.app:app --host 127.0.0.1 --port 15721 \
    > "$LOG_DIR/claude_proxy.log" 2>&1 &
PYTHON_PROXY_PID=$!

# 等待服务启动
sleep 3

# 检查Python代理服务状态
if ps -p $PYTHON_PROXY_PID > /dev/null 2>&1; then
    echo -e "${GREEN}✓${NC} Python代理服务已启动 (PID: $PYTHON_PROXY_PID, 127.0.0.1:15721)"
else
    echo -e "${RED}✗${NC} Python代理服务启动失败，查看日志: cat $LOG_DIR/claude_proxy.log"
fi

echo ""
echo "=== 服务状态 ==="

# 1. Python代理服务器
echo ""
echo "[Python代理服务器]"
if pgrep -f "uvicorn.*backend.app" > /dev/null; then
    echo -e "${GREEN}✓${NC} 运行中 (127.0.0.1:15721)"
else
    echo -e "${RED}✗${NC} 未运行"
fi

# 2. 数据库
echo ""
echo "[数据库]"
if [ -f ~/.cc-switch/cc-switch.db ]; then
    echo -e "${GREEN}✓${NC} 数据库文件存在"
else
    echo -e "${YELLOW}⚠${NC} 未初始化（首次运行时自动创建）"
fi

echo ""
echo "=== 安装完成 ==="
echo "列出服务商: cc-switch-cli list"
echo "查看日志: tail -f $LOG_DIR/claude_proxy.log"
echo ""
