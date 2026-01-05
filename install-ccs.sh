#!/bin/bash
# CC-Switch CLI 快速安装脚本（完善版）

# 颜色定义
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 进度条函数
show_progress() {
    local current=$1
    local total=$2
    local step_name=$3
    local status=$4  # "running" 或 "done" 或 "error"

    local percent=$((current * 100 / total))
    local filled=$((current * 40 / total))
    local empty=$((40 - filled))

    # 移动到行首并清除行
    printf "\r\033[K"

    # 显示进度条
    printf "["
    printf "%${filled}s" | tr ' ' '='
    printf "%${empty}s" | tr ' ' '-'
    printf "] %3d%% " "$percent"

    # 显示状态
    case "$status" in
        "running")
            printf "${CYAN}◷${NC} [%d/%d] %s" "$current" "$total" "$step_name"
            ;;
        "done")
            printf "${GREEN}✓${NC} [%d/%d] %s" "$current" "$total" "$step_name"
            ;;
        "error")
            printf "${RED}✗${NC} [%d/%d] %s" "$current" "$total" "$step_name"
            ;;
    esac
}

# 步骤完成函数
step_done() {
    local current=$1
    local total=$2
    local step_name=$3
    show_progress "$current" "$total" "$step_name" "done"
    # 不换行，保持在同一行等待下一个步骤覆盖
}

# 步骤运行中函数
step_running() {
    local current=$1
    local total=$2
    local step_name=$3
    show_progress "$current" "$total" "$step_name" "running"
}

# 步骤错误函数
step_error() {
    local current=$1
    local total=$2
    local step_name=$3
    show_progress "$current" "$total" "$step_name" "error"
    echo ""  # 错误时换行，因为后续会有错误信息
}

# 清屏并显示标题
clear
echo -e "${CYAN}     CC-Switch CLI 快速安装与部署工具          ${NC}"
echo ""

# 总步骤数
TOTAL_STEPS=10

# 创建日志目录
LOG_DIR="$HOME/.cc-switch/logs"
mkdir -p "$LOG_DIR"

# 清理旧日志文件
rm -f /tmp/cc-switch*.log 2>/dev/null || true
rm -f "$LOG_DIR/rust_proxy.log" 2>/dev/null || true
rm -f "$LOG_DIR/claude_proxy.log" 2>/dev/null || true

# 检测操作系统类型
detect_os() {
    if [ -f /etc/os-release ]; then
        . /etc/os-release
        OS=$ID
        OS_VERSION=$VERSION_ID
    elif type lsb_release >/dev/null 2>&1; then
        OS=$(lsb_release -si | tr '[:upper:]' '[:lower:]')
        OS_VERSION=$(lsb_release -sr)
    elif [ -f /etc/lsb-release ]; then
        . /etc/lsb-release
        OS=$(echo $DISTRIB_ID | tr '[:upper:]' '[:lower:]')
        OS_VERSION=$DISTRIB_RELEASE
    elif [ "$(uname)" = "Darwin" ]; then
        OS="macos"
        OS_VERSION=$(sw_vers -productVersion)
    else
        OS="unknown"
        OS_VERSION="unknown"
    fi
}

detect_os

# 检查并安装基础工具
check_and_install_tools() {
    local tools_missing=false

    # 检查curl
    if ! command -v curl &> /dev/null; then
        echo -e "${YELLOW}curl 未安装，正在安装...${NC}"
        case "$OS" in
            ubuntu|debian)
                sudo apt-get install -y curl > /dev/null 2>&1 || tools_missing=true
                ;;
            centos|rhel|fedora)
                sudo yum install -y curl > /dev/null 2>&1 || sudo dnf install -y curl > /dev/null 2>&1 || tools_missing=true
                ;;
            macos)
                # macOS默认自带curl
                :
                ;;
        esac
    fi

    # 检查git（可选，但建议安装）
    if ! command -v git &> /dev/null; then
        echo -e "${YELLOW}git 未安装，正在安装...${NC}"
        case "$OS" in
            ubuntu|debian)
                sudo apt-get install -y git > /dev/null 2>&1
                ;;
            centos|rhel|fedora)
                sudo yum install -y git > /dev/null 2>&1 || sudo dnf install -y git > /dev/null 2>&1
                ;;
            macos)
                if command -v brew &> /dev/null; then
                    brew install git > /dev/null 2>&1
                fi
                ;;
        esac
    fi

    if [ "$tools_missing" = true ]; then
        echo -e "${RED}错误: 部分基础工具安装失败，请手动安装curl${NC}"
        exit 1
    fi
}

check_and_install_tools

# ============================================================================
# 步骤 1: 检查并安装 Rust 工具链
# ============================================================================
CURRENT_STEP=1
step_running $CURRENT_STEP $TOTAL_STEPS "检查 Rust 工具链"

if ! command -v cargo &> /dev/null; then
    echo ""
    echo -e "${YELLOW}Rust 未安装，正在自动安装...${NC}"

    # 下载并安装 Rust
    if curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable > /dev/null 2>&1; then
        # 加载 Rust 环境变量
        if [ -f "$HOME/.cargo/env" ]; then
            source "$HOME/.cargo/env"
        fi

        # 验证安装
        if command -v cargo &> /dev/null; then
            RUST_VERSION=$(rustc --version)
            step_done $CURRENT_STEP $TOTAL_STEPS "安装 Rust 工具链 ($RUST_VERSION)"
        else
            step_error $CURRENT_STEP $TOTAL_STEPS "Rust 安装失败"
            echo ""
            echo -e "${RED}错误: Rust 自动安装失败${NC}"
            echo "请手动运行: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
            echo ""
            exit 1
        fi
    else
        step_error $CURRENT_STEP $TOTAL_STEPS "Rust 安装失败"
        echo ""
        echo -e "${RED}错误: Rust 自动安装失败，请检查网络连接${NC}"
        echo ""
        exit 1
    fi
else
    RUST_VERSION=$(rustc --version)
    step_done $CURRENT_STEP $TOTAL_STEPS "检查 Rust 工具链 ($RUST_VERSION)"
fi

# ============================================================================
# 步骤 2: 检查并安装 Python 环境
# ============================================================================
CURRENT_STEP=2
step_running $CURRENT_STEP $TOTAL_STEPS "检查 Python 环境"

PYTHON_CMD=""
if command -v python3 &> /dev/null; then
    PYTHON_CMD="python3"
elif command -v python &> /dev/null; then
    PYTHON_VERSION=$(python --version 2>&1 | grep -oP '\d+\.\d+')
    if [[ $(echo "$PYTHON_VERSION >= 3.8" | bc -l 2>/dev/null || echo 0) -eq 1 ]]; then
        PYTHON_CMD="python"
    fi
fi

if [ -z "$PYTHON_CMD" ]; then
    echo ""
    echo -e "${YELLOW}Python 3.8+ 未安装，正在自动安装...${NC}"

    # 根据操作系统自动安装Python
    case "$OS" in
        ubuntu|debian)
            if command -v sudo &> /dev/null; then
                sudo apt-get update -qq > /dev/null 2>&1
                if sudo apt-get install -y python3 python3-venv python3-pip > /dev/null 2>&1; then
                    PYTHON_CMD="python3"
                fi
            fi
            ;;
        centos|rhel|fedora)
            if command -v sudo &> /dev/null; then
                if sudo yum install -y python3 python3-pip > /dev/null 2>&1; then
                    PYTHON_CMD="python3"
                elif sudo dnf install -y python3 python3-pip > /dev/null 2>&1; then
                    PYTHON_CMD="python3"
                fi
            fi
            ;;
        macos)
            if command -v brew &> /dev/null; then
                if brew install python@3 > /dev/null 2>&1; then
                    PYTHON_CMD="python3"
                fi
            else
                echo -e "${YELLOW}提示: macOS需要先安装Homebrew${NC}"
                echo "请运行: /bin/bash -c \"\$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\""
            fi
            ;;
    esac

    # 验证Python安装
    if [ -z "$PYTHON_CMD" ] || ! command -v $PYTHON_CMD &> /dev/null; then
        step_error $CURRENT_STEP $TOTAL_STEPS "Python 自动安装失败"
        echo ""
        echo -e "${RED}错误: Python 自动安装失败${NC}"
        echo "请手动安装 Python 3.8+:"
        echo "  Ubuntu/Debian: sudo apt-get install python3 python3-venv python3-pip"
        echo "  CentOS/RHEL: sudo yum install python3 python3-pip"
        echo "  macOS: brew install python@3"
        echo ""
        exit 1
    fi

    PYTHON_VERSION=$($PYTHON_CMD --version 2>&1)
    step_done $CURRENT_STEP $TOTAL_STEPS "安装 Python 环境 ($PYTHON_VERSION)"
else
    PYTHON_VERSION=$($PYTHON_CMD --version 2>&1)
    step_done $CURRENT_STEP $TOTAL_STEPS "检查 Python 环境 ($PYTHON_VERSION)"
fi

# ============================================================================
# 步骤 3: 安装 Python 依赖
# ============================================================================
CURRENT_STEP=3
step_running $CURRENT_STEP $TOTAL_STEPS "安装 Python 依赖"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REQUIREMENTS_FILE="$SCRIPT_DIR/claude_proxy/backend/requirements.txt"

if [ ! -f "$REQUIREMENTS_FILE" ]; then
    step_error $CURRENT_STEP $TOTAL_STEPS "依赖文件不存在"
    echo ""
    echo -e "${RED}错误: 依赖文件不存在: $REQUIREMENTS_FILE${NC}"
    exit 1
fi

# 确保pip可用
if ! $PYTHON_CMD -m pip --version &> /dev/null; then
    echo ""
    echo -e "${YELLOW}pip 未安装，正在安装...${NC}"

    # 尝试使用ensurepip安装pip
    if $PYTHON_CMD -m ensurepip --upgrade &> /dev/null; then
        echo -e "${GREEN}pip 安装成功${NC}"
    else
        # 使用get-pip.py安装
        if curl -sS https://bootstrap.pypa.io/get-pip.py | $PYTHON_CMD - --user &> /dev/null; then
            echo -e "${GREEN}pip 安装成功${NC}"
        else
            step_error $CURRENT_STEP $TOTAL_STEPS "pip 安装失败"
            echo ""
            echo -e "${RED}错误: pip 安装失败${NC}"
            exit 1
        fi
    fi
fi

# 安装Python依赖
if ! $PYTHON_CMD -c "import fastapi" &> /dev/null; then
    $PYTHON_CMD -m pip install -q --upgrade pip --user 2>&1 | grep -v "WARNING" || true
    if ! $PYTHON_CMD -m pip install -q -r "$REQUIREMENTS_FILE" --user 2>&1 | grep -v "WARNING"; then
        step_error $CURRENT_STEP $TOTAL_STEPS "Python 依赖安装失败"
        echo ""
        exit 1
    fi
else
    $PYTHON_CMD -m pip install -q -r "$REQUIREMENTS_FILE" --upgrade --user 2>/dev/null || true
fi

step_done $CURRENT_STEP $TOTAL_STEPS "安装 Python 依赖"

# ============================================================================
# 步骤 4: 编译 CLI 工具
# ============================================================================
CURRENT_STEP=4
step_running $CURRENT_STEP $TOTAL_STEPS "编译 CLI 工具"

cd "$SCRIPT_DIR/src-tauri" || exit 1

NEED_REBUILD=false
CLI_BINARY="target/release/cc-switch-cli"

if [ ! -f "$CLI_BINARY" ]; then
    NEED_REBUILD=true
else
    if [ -n "$(find src -name "*.rs" -newer "$CLI_BINARY" 2>/dev/null)" ]; then
        NEED_REBUILD=true
    elif [ -f "Cargo.toml" ] && [ "Cargo.toml" -nt "$CLI_BINARY" ]; then
        NEED_REBUILD=true
    fi
fi

if [ "$NEED_REBUILD" = true ]; then
    if cargo build --release --bin cc-switch-cli > "$LOG_DIR/build.log" 2>&1; then
        if grep -q "^error" "$LOG_DIR/build.log"; then
            step_error $CURRENT_STEP $TOTAL_STEPS "编译失败"
            echo ""
            echo -e "${RED}编译失败，查看日志: cat $LOG_DIR/build.log${NC}"
            exit 1
        fi
    else
        step_error $CURRENT_STEP $TOTAL_STEPS "编译失败"
        echo ""
        echo -e "${RED}编译失败，查看日志: cat $LOG_DIR/build.log${NC}"
        exit 1
    fi

    if [ ! -f "$CLI_BINARY" ]; then
        step_error $CURRENT_STEP $TOTAL_STEPS "编译失败，未生成二进制文件"
        echo ""
        exit 1
    fi
    step_done $CURRENT_STEP $TOTAL_STEPS "编译 CLI 工具 (新编译)"
else
    step_done $CURRENT_STEP $TOTAL_STEPS "编译 CLI 工具 (使用缓存)"
fi

CLI_PATH=$(realpath "$CLI_BINARY")

# ============================================================================
# 步骤 5: 停止旧服务
# ============================================================================
CURRENT_STEP=5
step_running $CURRENT_STEP $TOTAL_STEPS "停止旧服务"

# 停止Python代理
if pgrep -f "uvicorn.*backend.app" > /dev/null; then
    pkill -f "uvicorn.*backend.app" 2>/dev/null || true
    sleep 1
    if pgrep -f "uvicorn.*backend.app" > /dev/null; then
        pkill -9 -f "uvicorn.*backend.app" 2>/dev/null || true
    fi
fi

# 停止CLI代理
if pgrep -f "cc-switch-cli proxy" > /dev/null; then
    pkill -f "cc-switch-cli proxy" 2>/dev/null || true
    sleep 1
    if pgrep -f "cc-switch-cli proxy" > /dev/null; then
        pkill -9 -f "cc-switch-cli proxy" 2>/dev/null || true
    fi
fi

# 清理PID文件
rm -f "$HOME/.cc-switch/proxy.pid" 2>/dev/null
rm -f "$HOME/.cc-switch/python_proxy.pid" 2>/dev/null

step_done $CURRENT_STEP $TOTAL_STEPS "停止旧服务"

# ============================================================================
# 步骤 6: 安装到系统路径
# ============================================================================
CURRENT_STEP=6
step_running $CURRENT_STEP $TOTAL_STEPS "安装到系统路径"

INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"
cp "$CLI_PATH" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/cc-switch-cli"

step_done $CURRENT_STEP $TOTAL_STEPS "安装到系统路径 ($INSTALL_DIR)"

# ============================================================================
# 步骤 7: 检测和修复环境变量
# ============================================================================
CURRENT_STEP=7
step_running $CURRENT_STEP $TOTAL_STEPS "检测环境变量配置"

# 检测Shell配置文件
SHELL_CONFIG=""
if [ -n "$BASH_VERSION" ] && [ -f "$HOME/.bashrc" ]; then
    SHELL_CONFIG="$HOME/.bashrc"
elif [ -n "$ZSH_VERSION" ] && [ -f "$HOME/.zshrc" ]; then
    SHELL_CONFIG="$HOME/.zshrc"
elif [ -f "$HOME/.bash_profile" ]; then
    SHELL_CONFIG="$HOME/.bash_profile"
elif [ -f "$HOME/.profile" ]; then
    SHELL_CONFIG="$HOME/.profile"
fi

# 检测环境变量函数
check_env_config() {
    local config_file="$1"
    local var_name="$2"
    if [ -f "$config_file" ]; then
        grep -q "export $var_name=" "$config_file" && return 0
    fi
    return 1
}

# 检查是否需要修复
ENV_NEEDS_FIX=false

if ! check_env_config "$SHELL_CONFIG" "ANTHROPIC_BASE_URL" && [ -z "$ANTHROPIC_BASE_URL" ]; then
    ENV_NEEDS_FIX=true
fi

if ! check_env_config "$SHELL_CONFIG" "OPENAI_BASE_URL" && [ -z "$OPENAI_BASE_URL" ]; then
    ENV_NEEDS_FIX=true
fi

# 自动修复环境变量
if [ "$ENV_NEEDS_FIX" = true ] && [ -n "$SHELL_CONFIG" ]; then
    cp "$SHELL_CONFIG" "${SHELL_CONFIG}.backup.$(date +%Y%m%d_%H%M%S)" 2>/dev/null || true

    if ! check_env_config "$SHELL_CONFIG" "ANTHROPIC_BASE_URL"; then
        echo "" >> "$SHELL_CONFIG"
        echo "# CC-Switch 环境变量配置 (Claude CLI)" >> "$SHELL_CONFIG"
        echo 'export ANTHROPIC_BASE_URL="http://127.0.0.1:15721"' >> "$SHELL_CONFIG"
        echo 'export ANTHROPIC_API_KEY="sk-placeholder-managed-by-rust-backend"' >> "$SHELL_CONFIG"
    fi

    if ! check_env_config "$SHELL_CONFIG" "OPENAI_BASE_URL"; then
        echo "" >> "$SHELL_CONFIG"
        echo "# CC-Switch 环境变量配置 (Codex CLI)" >> "$SHELL_CONFIG"
        echo 'export OPENAI_BASE_URL="http://127.0.0.1:15721"' >> "$SHELL_CONFIG"
        echo 'export OPENAI_API_KEY="sk-placeholder-managed-by-rust-backend"' >> "$SHELL_CONFIG"
    fi
    step_done $CURRENT_STEP $TOTAL_STEPS "检测环境变量配置 (已修复)"
else
    step_done $CURRENT_STEP $TOTAL_STEPS "检测环境变量配置 (正常)"
fi

# 日志轮转函数
rotate_log_if_needed() {
    local log_file="$1"
    local max_size_mb=10
    local max_backups=5

    if [ ! -f "$log_file" ]; then
        return 0
    fi

    local file_size_mb=$(du -m "$log_file" 2>/dev/null | cut -f1)

    if [ "$file_size_mb" -ge "$max_size_mb" ]; then
        if [ -f "${log_file}.${max_backups}" ]; then
            rm -f "${log_file}.${max_backups}"
        fi
        for i in $(seq $((max_backups-1)) -1 1); do
            if [ -f "${log_file}.${i}" ]; then
                mv "${log_file}.${i}" "${log_file}.$((i+1))"
            fi
        done
        mv "$log_file" "${log_file}.1"
        touch "$log_file"
    fi
}

rotate_log_if_needed "$LOG_DIR/rust_proxy.log"
rotate_log_if_needed "$LOG_DIR/claude_proxy.log"

# ============================================================================
# 步骤 8: 启动 Python 代理服务
# ============================================================================
CURRENT_STEP=8
step_running $CURRENT_STEP $TOTAL_STEPS "启动 Python 代理服务"

cd "$SCRIPT_DIR/claude_proxy" || exit 1
nohup env HTTP_PROXY="http://127.0.0.1:7890" HTTPS_PROXY="http://127.0.0.1:7890" \
    $PYTHON_CMD -m uvicorn backend.app:app --host 127.0.0.1 --port 15722 --log-level warning \
    > "$LOG_DIR/claude_proxy.log" 2>&1 &
PYTHON_PROXY_PID=$!

echo "$PYTHON_PROXY_PID" > "$HOME/.cc-switch/python_proxy.pid"

sleep 2

if ps -p $PYTHON_PROXY_PID > /dev/null 2>&1; then
    step_done $CURRENT_STEP $TOTAL_STEPS "启动 Python 代理服务 (PID: $PYTHON_PROXY_PID)"
else
    step_error $CURRENT_STEP $TOTAL_STEPS "Python 代理服务启动失败"
    echo ""
    echo -e "${RED}启动失败，查看日志: tail -f $LOG_DIR/claude_proxy.log${NC}"
    exit 1
fi

# ============================================================================
# 步骤 9: 启动 Rust 代理服务
# ============================================================================
CURRENT_STEP=9
step_running $CURRENT_STEP $TOTAL_STEPS "启动 Rust 代理服务"

cd "$SCRIPT_DIR" || exit 1
nohup "$INSTALL_DIR/cc-switch-cli" proxy start > "$LOG_DIR/rust_proxy.log" 2>&1 &
RUST_PROXY_PID=$!

sleep 2

if ps -p $RUST_PROXY_PID > /dev/null 2>&1; then
    step_done $CURRENT_STEP $TOTAL_STEPS "启动 Rust 代理服务 (PID: $RUST_PROXY_PID)"
else
    step_error $CURRENT_STEP $TOTAL_STEPS "Rust 代理服务启动失败"
    echo ""
    echo -e "${RED}启动失败，查看日志: tail -f $LOG_DIR/rust_proxy.log${NC}"
    kill "$PYTHON_PROXY_PID" 2>/dev/null || true
    exit 1
fi

# ============================================================================
# 步骤 10: 验证部署状态
# ============================================================================
CURRENT_STEP=10
step_running $CURRENT_STEP $TOTAL_STEPS "验证部署状态"

sleep 1

# 验证服务状态
PYTHON_OK=false
RUST_OK=false
DB_OK=false

if pgrep -f "uvicorn.*backend.app.*--port 15722" > /dev/null; then
    PYTHON_OK=true
fi

RUST_PID_FILE="$HOME/.cc-switch/proxy.pid"
if [ -f "$RUST_PID_FILE" ]; then
    PID=$(cat "$RUST_PID_FILE")
    if ps -p "$PID" > /dev/null 2>&1; then
        RUST_OK=true
    fi
fi

if [ -f "$HOME/.cc-switch/cc-switch.db" ]; then
    DB_OK=true
fi

if [ "$PYTHON_OK" = true ] && [ "$RUST_OK" = true ]; then
    step_done $CURRENT_STEP $TOTAL_STEPS "验证部署状态 (成功)"
    echo ""  # 最后一步完成，换行
else
    step_error $CURRENT_STEP $TOTAL_STEPS "验证部署状态 (部分失败)"
    # step_error已经换行
fi

echo ""
echo ""
echo -e "${GREEN}          部署完成 - 服务状态报告              ${NC}"
echo ""

# 服务状态详情
echo -e "${BLUE}┌─ Python 代理层 (端口 15722)${NC}"
if [ "$PYTHON_OK" = true ]; then
    echo -e "│  状态: ${GREEN}✓ 运行中${NC}"
    echo -e "│  地址: 127.0.0.1:15722"
    echo -e "│  日志: tail -f $LOG_DIR/claude_proxy.log"
else
    echo -e "│  状态: ${RED}✗ 未运行${NC}"
fi
echo ""

echo -e "${BLUE}┌─ Rust 代理层 (端口 15721)${NC}"
if [ "$RUST_OK" = true ]; then
    echo -e "│  状态: ${GREEN}✓ 运行中${NC}"
    echo -e "│  地址: 127.0.0.1:15721"
    echo -e "│  日志: tail -f $LOG_DIR/rust_proxy.log"
else
    echo -e "│  状态: ${RED}✗ 未运行${NC}"
fi
echo ""

echo -e "${BLUE}┌─ 数据库${NC}"
if [ "$DB_OK" = true ]; then
    echo -e "│  状态: ${GREEN}✓ 已初始化${NC}"
    echo -e "│  位置: ~/.cc-switch/cc-switch.db"
else
    echo -e "│  状态: ${YELLOW}⚠ 未初始化${NC} (首次运行时自动创建)"
fi
echo ""

echo -e "${CYAN}┌─ 快速命令${NC}"
echo -e "│  ${YELLOW}cc-switch-cli list${NC}          # 列出所有供应商"
echo -e "│  ${YELLOW}cc-switch-cli proxy status${NC}  # 查看代理状态"
echo -e "│  ${YELLOW}cc-switch-cli --help${NC}        # 查看帮助信息"
echo ""


echo ""
echo -e "│  ${BLUE}完整文档: ${CYAN}$SCRIPT_DIR/docs/USER-GUIDE.md${NC}"
echo ""
