# CC-Switch（Linux 终端增强版 / Terminal Edition）

本仓库是对上游 `farion1231/cc-switch` 的二次开发版本：**专注纯 Linux 终端/CLI 场景**，目标是让 Claude Code / Codex CLI / Gemini CLI 在 Linux 下能“更稳、更快、更好排障”地使用多供应商代理与测速能力。

- 原上游 README 备份：`README.upstream.md`
- 一键安装/部署脚本：`install-ccs.sh`
- 终端使用指南：`docs/USER-GUIDE.md`

---

## 一键部署（推荐）

```bash
git clone https://github.com/Aaroen/cc-switch.git
cd cc-switch
chmod +x ./install-ccs.sh
./install-ccs.sh
```

脚本会自动：
- 编译并安装 `csc` CLI
- 启动 Python 代理层（默认端口 `15722`）与 Rust 代理层（默认端口 `15721`）
- 接管/写入 Claude CLI / Codex CLI / Gemini CLI 的本地代理配置
- 初始化 `~/.cc-switch/cc-switch.db` 并输出服务状态与日志位置

---

## 终端特性（相对上游的重点增强）

> 具体差异可对比：本仓库 vs 上游仓库/参考仓库（见下方链接）。

- **真实链路测速**：`csc t claude|codex|gemini` 通过“真实启动请求”验证可用性并择优固定。
- **智能模型名解析 + 写回**：
  - Claude：按 `family(haiku/sonnet/opus) → major/minor(4/5) → thinking` 优先级匹配供应商真实模型名，成功后写回，避免重复匹配与启动报错。
  - Codex(OpenAI)：基于同家族模型策略（`gpt/o1/o3` 等），结合供应商 `/v1/models` 与历史写回别名，选择最贴近的真实模型名并写回。
- **家族守护（Family Affinity）**：优先保证“请求家族不跨家族映射”，避免 `claude-*` 被映射到其他家族造成体验断崖。
- **GPT 模型名深度净化**：对 `gpt-*` 强制去除日期/版本尾巴（例如 `-2024-08-06`、`-0613`），内部只处理基础名，杜绝“幽灵模型名”导致的匹配失败。
- **指纹/请求摘要兜底**：内置 `src-tauri/defaults/last_request_summaries.json`，即使首次没有真实请求，也能提供一套可用默认摘要用于启动测速复用（提升安装后可用性）。
- **日志降噪 + 可视化对齐**：北京时间（UTC+8）、单行输出、追加上游实际模型（含映射箭头），方便在纯终端环境快速定位问题。

---

## 常用命令

```bash
# 列出供应商
csc ls

# 代理状态
csc p st

# 真实测速（可选指定 supplier）
csc t claude
csc t codex
csc t codex <supplier>

# 导入/导出配置
csc ex my.json
csc im my.json
```

日志查看：

```bash
tail -f ~/.cc-switch/logs/rust_proxy.log
tail -f ~/.cc-switch/logs/claude_proxy.log
```

---

## 协议与致谢（MIT）

本仓库遵循 `LICENSE`（MIT）。

二次开发来源与参考（均为 MIT）：
- 上游：`https://github.com/farion1231/cc-switch`
- 参考透明代理实现思路：`https://github.com/RebornQ/AnyRouter-Transparent-Proxy`

本仓库保留并遵循其许可证要求；如需查看上游的原始说明与截图，请见 `README.upstream.md`。

---

## 安全提示

- 请勿把供应商 `API Key`、本地 `~/.cc-switch` 配置与日志直接公开到互联网。
- 如需分享排障信息，建议先脱敏 `Authorization`、`api-key` 与任何包含 key 的字段。

