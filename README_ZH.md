<div align="center">

# Claude Code & Codex 供应商管理器

[![Version](https://img.shields.io/badge/version-3.6.1-blue.svg)](https://github.com/farion1231/cc-switch/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/farion1231/cc-switch/releases)
[![Built with Tauri](https://img.shields.io/badge/built%20with-Tauri%202-orange.svg)](https://tauri.app/)

[English](README.md) | 中文 | [更新日志](CHANGELOG.md)

一个用于管理和切换 Claude Code 与 Codex 不同供应商配置、MCP的桌面应用。

</div>

## ❤️赞助商

![智谱 GLM](assets/partners/banners/glm-zh.jpg)

感谢智谱AI的 GLM CODING PLAN 赞助了本项目！

GLM CODING PLAN 是专为AI编码打造的订阅套餐,每月最低仅需20元，即可在十余款主流AI编码工具如 Claude Code、Cline 中畅享智谱旗舰模型 GLM-4.6，为开发者提供顶尖、高速、稳定的编码体验。

CC Switch 已经预设了智谱GLM，只需要填写 key 即可一键导入编程工具。智谱AI为本软件的用户提供了特别优惠，使用[此链接](https://www.bigmodel.cn/claude-code?ic=RRVJPB5SII)购买可以享受九折优惠。

---

<table>
<tr>
<td width="180"><img src="assets/partners/logos/packycode.png" alt="PackyCode" width="150"></td>
<td>感谢 PackyCode 赞助了本项目！PackyCode 是一家稳定、高效的API中转服务商，提供 Claude Code、Codex、Gemini 等多种中转服务。PackyCode 为本软件的用户提供了特别优惠，使用<a href="https://www.packyapi.com/register?aff=cc-switch">此链接</a>注册并在充值时填写"cc-switch"优惠码，可以享受9折优惠。</td>
</tr>
</table>

## 界面预览

|                  主界面                   |                  添加供应商                  |
| :---------------------------------------: | :------------------------------------------: |
| ![主界面](assets/screenshots/main-zh.png) | ![添加供应商](assets/screenshots/add-zh.png) |

## 功能特性

### 当前版本：v3.6.1 | [完整更新日志](CHANGELOG.md)

**核心功能**

- **供应商管理**：一键切换 Claude Code 与 Codex 的 API 配置
- **MCP 集成**：集中管理 MCP 服务器，支持 stdio/http 类型和实时同步
- **速度测试**：测量 API 端点延迟，可视化连接质量指示器
- **导入导出**：备份和恢复配置，自动轮换（保留最近 10 个）
- **国际化支持**：完整的中英文本地化（UI、错误、托盘）
- **Claude 插件同步**：一键应用或恢复 Claude 插件配置

**v3.6 亮点**

- 供应商复制 & 拖拽排序
- 多端点管理 & 自定义配置目录（支持云同步）
- 细粒度模型配置（四层：Haiku/Sonnet/Opus/自定义）
- WSL 环境支持，配置目录切换自动同步
- 100% hooks 测试覆盖 & 完整架构重构
- 新增预设：DMXAPI、Azure Codex、AnyRouter、AiHubMix、MiniMax

**系统功能**

- 系统托盘快速切换
- 单实例守护
- 内置自动更新器
- 原子写入与回滚保护

## 下载安装

### 系统要求

- **Windows**: Windows 10 及以上
- **macOS**: macOS 10.15 (Catalina) 及以上
- **Linux**: Ubuntu 22.04+ / Debian 11+ / Fedora 34+ 等主流发行版

### Windows 用户

从 [Releases](../../releases) 页面下载最新版本的 `CC-Switch-v{版本号}-Windows.msi` 安装包或者 `CC-Switch-v{版本号}-Windows-Portable.zip` 绿色版。

### macOS 用户

**方式一：通过 Homebrew 安装（推荐）**

```bash
brew tap farion1231/ccswitch
brew install --cask cc-switch
```

更新：

```bash
brew upgrade --cask cc-switch
```

**方式二：手动下载**

从 [Releases](../../releases) 页面下载 `CC-Switch-v{版本号}-macOS.zip` 解压使用。

> **注意**：由于作者没有苹果开发者账号，首次打开可能出现"未知开发者"警告，请先关闭，然后前往"系统设置" → "隐私与安全性" → 点击"仍要打开"，之后便可以正常打开

### Linux 用户

从 [Releases](../../releases) 页面下载最新版本的 `CC-Switch-v{版本号}-Linux.deb` 包或者 `CC-Switch-v{版本号}-Linux.AppImage` 安装包。

## 快速开始

### 基本使用

1. **添加供应商**：点击"添加供应商" → 选择预设或创建自定义配置
2. **切换供应商**：
   - 主界面：选择供应商 → 点击"启用"
   - 系统托盘：直接点击供应商名称（立即生效）
3. **生效方式**：重启终端或 Claude Code/Codex 以应用更改
4. **恢复官方登录**：选择"官方登录"预设，重启终端后使用 `/login`（Claude）或官方登录流程（Codex）

### MCP 管理

- **位置**：点击右上角"MCP"按钮
- **添加服务器**：使用内置模板（mcp-fetch、mcp-filesystem）或自定义配置
- **启用/禁用**：切换开关以控制哪些服务器同步到 live 配置
- **同步**：启用的服务器自动同步到 `~/.claude.json`（Claude）或 `~/.codex/config.toml`（Codex）

### 配置文件

**Claude Code**

- Live 配置：`~/.claude/settings.json`（或 `claude.json`）
- API key 字段：`env.ANTHROPIC_AUTH_TOKEN` 或 `env.ANTHROPIC_API_KEY`
- MCP 服务器：`~/.claude.json` → `mcpServers`

**Codex**

- Live 配置：`~/.codex/auth.json`（必需）+ `config.toml`（可选）
- API key 字段：`auth.json` 中的 `OPENAI_API_KEY`
- MCP 服务器：`~/.codex/config.toml` → `[mcp.servers]`

**CC Switch 存储**

- 主配置（SSOT）：`~/.cc-switch/config.json`
- 设置：`~/.cc-switch/settings.json`
- 备份：`~/.cc-switch/backups/`（自动轮换，保留 10 个）

### 云同步设置

1. 前往设置 → "自定义配置目录"
2. 选择您的云同步文件夹（Dropbox、OneDrive、iCloud、坚果云等）
3. 重启应用以应用
4. 在其他设备上重复操作以启用跨设备同步

> **注意**：首次启动会自动导入现有 Claude/Codex 配置作为默认供应商。

## 架构总览

### 设计原则

```
┌─────────────────────────────────────────────────────────────┐
│                    前端 (React + TS)                        │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │ Components  │  │    Hooks     │  │  TanStack Query  │  │
│  │   (UI)      │──│ (业务逻辑)   │──│   (缓存/同步)    │  │
│  └─────────────┘  └──────────────┘  └──────────────────┘  │
└────────────────────────┬────────────────────────────────────┘
                         │ Tauri IPC
┌────────────────────────▼────────────────────────────────────┐
│                  后端 (Tauri + Rust)                        │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐  │
│  │  Commands   │  │   Services   │  │  Models/Config   │  │
│  │  (API 层)   │──│  (业务层)    │──│     (数据)       │  │
│  └─────────────┘  └──────────────┘  └──────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

**核心设计模式**

- **SSOT**（单一事实源）：所有供应商配置存储在 `~/.cc-switch/config.json`
- **双向同步**：切换时写入 live 文件，编辑当前供应商时从 live 回填
- **原子写入**：临时文件 + 重命名模式防止配置损坏
- **并发安全**：RwLock 与作用域守卫避免死锁
- **分层架构**：清晰分离（Commands → Services → Models）

**核心组件**

- **ProviderService**：供应商增删改查、切换、回填、排序
- **McpService**：MCP 服务器管理、导入导出、live 文件同步
- **ConfigService**：配置导入导出、备份轮换
- **SpeedtestService**：API 端点延迟测量

**v3.6 重构**

- 后端：5 阶段重构（错误处理 → 命令拆分 → 测试 → 服务 → 并发）
- 前端：4 阶段重构（测试基础 → hooks → 组件 → 清理）
- 测试：100% hooks 覆盖 + 集成测试（vitest + MSW）

## 开发

### 环境要求

- Node.js 18+
- pnpm 8+
- Rust 1.85+
- Tauri CLI 2.8+

### 开发命令

```bash
# 安装依赖
pnpm install

# 开发模式（热重载）
pnpm dev

# 类型检查
pnpm typecheck

# 代码格式化
pnpm format

# 检查代码格式
pnpm format:check

# 运行前端单元测试
pnpm test:unit

# 监听模式运行测试（推荐开发时使用）
pnpm test:unit:watch

# 构建应用
pnpm build

# 构建调试版本
pnpm tauri build --debug
```

### Rust 后端开发

```bash
cd src-tauri

# 格式化 Rust 代码
cargo fmt

# 运行 clippy 检查
cargo clippy

# 运行后端测试
cargo test

# 运行特定测试
cargo test test_name

# 运行带测试 hooks 的测试
cargo test --features test-hooks
```

### 测试说明（v3.6 新增）

**前端测试**：

- 使用 **vitest** 作为测试框架
- 使用 **MSW (Mock Service Worker)** 模拟 Tauri API 调用
- 使用 **@testing-library/react** 进行组件测试

**测试覆盖**：

- Hooks 单元测试（100% 覆盖）
  - `useProviderActions` - 供应商操作
  - `useMcpActions` - MCP 管理
  - `useSettings` 系列 - 设置管理
  - `useImportExport` - 导入导出
- 集成测试
  - App 主应用流程
  - SettingsDialog 完整交互
  - MCP 面板功能

**运行测试**：

```bash
# 运行所有测试
pnpm test:unit

# 监听模式（自动重跑）
pnpm test:unit:watch

# 带覆盖率报告
pnpm test:unit --coverage
```

## 技术栈

**前端**：React 18 · TypeScript · Vite · TailwindCSS 4 · TanStack Query v5 · react-i18next · react-hook-form · zod · shadcn/ui · @dnd-kit

**后端**：Tauri 2.8 · Rust · serde · tokio · thiserror · tauri-plugin-updater/process/dialog/store/log

**测试**：vitest · MSW · @testing-library/react

## 项目结构

```
├── src/                      # 前端 (React + TypeScript)
│   ├── components/           # UI 组件 (providers/settings/mcp/ui)
│   ├── hooks/                # 自定义 hooks (业务逻辑)
│   ├── lib/
│   │   ├── api/              # Tauri API 封装（类型安全）
│   │   └── query/            # TanStack Query 配置
│   ├── i18n/locales/         # 翻译 (zh/en)
│   ├── config/               # 预设 (providers/mcp)
│   └── types/                # TypeScript 类型定义
├── src-tauri/                # 后端 (Rust)
│   └── src/
│       ├── commands/         # Tauri 命令层（按领域）
│       ├── services/         # 业务逻辑层
│       ├── app_config.rs     # 配置数据模型
│       ├── provider.rs       # 供应商领域模型
│       ├── mcp.rs            # MCP 同步与校验
│       └── lib.rs            # 应用入口 & 托盘菜单
├── tests/                    # 前端测试
│   ├── hooks/                # 单元测试
│   └── components/           # 集成测试
└── assets/                   # 截图 & 合作商资源
```

## 更新日志

查看 [CHANGELOG.md](CHANGELOG.md) 了解版本更新详情。

## Electron 旧版

[Releases](../../releases) 里保留 v2.0.3 Electron 旧版

如果需要旧版 Electron 代码，可以拉取 electron-legacy 分支

## 贡献

欢迎提交 Issue 反馈问题和建议！

提交 PR 前请确保：

- 通过类型检查：`pnpm typecheck`
- 通过格式检查：`pnpm format:check`
- 通过单元测试：`pnpm test:unit`
- 功能性 PR 请先经过 issue 区讨论

## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=farion1231/cc-switch&type=Date)](https://www.star-history.com/#farion1231/cc-switch&Date)

## License

MIT © Jason Young
