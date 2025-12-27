# CC-Switch 用户指南

## 项目概述

**CC-Switch** 是一个跨平台桌面应用程序，为 Claude Code、Codex 和 Gemini CLI 提供统一的配置管理和智能代理服务。该应用采用 Tauri 2.8 架构，前端基于 React 18 + TypeScript，后端基于 Rust，支持 Windows、macOS 和 Linux 平台。

**核心价值**：
- 统一管理多个 AI CLI 工具的配置
- 提供智能代理服务，支持自动故障转移和容错
- 降低 API 调用成本，优化性能
- 提供图形界面和命令行工具双接口

**技术架构**：
- 后端：Rust + Tauri 2.8 + Tokio + Axum
- 前端：React 18 + TypeScript + Vite 5 + TailwindCSS
- 数据库：SQLite 3.31 + JSON 双层架构
- 总代码量：约 57,000 行

---

## 系统要求

### 操作系统
- **Windows**：Windows 10 及以上版本
- **macOS**：macOS 10.15 (Catalina) 及以上版本
- **Linux**：Ubuntu 22.04+ / Debian 11+ / Fedora 34+

### 硬件要求
- 内存：最低 4GB RAM
- 存储：至少 100MB 可用空间
- 网络：需要互联网连接以访问 API 服务

---

## 安装方法

### Windows 平台
提供两种安装方式：
1. MSI 安装包：双击运行，按向导完成安装
2. Portable ZIP：解压后直接运行 `cc-switch.exe`

### macOS 平台
通过 Homebrew 安装：
```bash
brew tap farion1231/ccswitch
brew install --cask cc-switch
```

### Linux 平台
根据发行版选择：
- Debian/Ubuntu：下载 `.deb` 包，执行 `sudo dpkg -i cc-switch_*.deb`
- Arch Linux：通过 AUR 安装
- 其他发行版：使用 AppImage 格式

### CLI 工具安装
运行项目根目录下的安装脚本：
```bash
./install-cli.sh
```
该脚本将 `cc-switch-cli` 安装到 `~/.local/bin/` 目录。

---

## 核心功能

### 1. 供应商管理

供应商（Provider）是指提供 API 服务的第三方平台或自建服务。应用支持对供应商进行全生命周期管理。

#### 1.1 添加供应商

**通过图形界面**：
1. 点击界面顶部的"添加供应商"按钮
2. 填写供应商信息：
   - 名称：供应商的显示名称
   - API 端点：服务的 URL 地址
   - API 密钥：认证密钥
   - 类别：官方/国产/聚合/第三方/自定义
3. 点击"保存"完成添加

**通过命令行**：
```bash
cc-switch-cli add \
  --name "MyProvider" \
  --url "https://api.example.com" \
  --key "sk-xxxx"
```

#### 1.2 批量添加供应商

支持通过 URL 列表与密钥列表的笛卡尔积批量创建供应商：
```bash
cc-switch-cli add-group \
  --name "ServiceGroup" \
  --urls "url1.com,url2.com,url3.com" \
  --keys "key1,key2" \
  --cooldown-hours 72
```
上述命令将创建 3 × 2 = 6 个供应商实例。

#### 1.3 供应商切换

**通过图形界面**：
- 点击供应商卡片即可切换为当前活动供应商
- 或通过系统托盘菜单快速切换

**通过命令行**：
```bash
cc-switch-cli enable <provider-name>
```

#### 1.4 供应商管理操作

- **编辑**：修改供应商的配置信息
- **克隆**：快速复制现有供应商配置
- **删除**：移除不再使用的供应商
- **排序**：通过拖拽调整供应商显示顺序
- **速度测试**：测量 API 端点的网络延迟

#### 1.5 供应商导入导出

**导出配置**：
```bash
cc-switch-cli export ~/backup/providers.json
```

**导入配置**：
```bash
cc-switch-cli import ~/backup/providers.json
```

### 2. MCP 服务器管理

MCP (Model Context Protocol) 服务器为 AI 工具提供扩展功能。应用支持统一管理 Claude、Codex 和 Gemini 的 MCP 服务器配置。

#### 2.1 支持的传输类型
- **stdio**：标准输入输出通信
- **http**：HTTP 协议通信
- **sse**：服务器发送事件（Server-Sent Events）

#### 2.2 MCP 服务器操作

**添加 MCP 服务器**：
1. 进入"MCP"标签页
2. 点击"添加 MCP 服务器"
3. 选择传输类型并填写配置信息
4. 选择启用的应用（Claude/Codex/Gemini）

**使用内置模板**：
应用内置 10+ 预配置 MCP 服务器模板，包括：
- 文件系统访问
- Git 集成
- 数据库连接
- Web 搜索
- 等

**配置同步**：
- GUI 修改会自动同步到 live 配置文件
- 可从 live 配置文件导入修改内容

**导入导出**：
- 支持跨应用导入导出 MCP 配置
- 支持 JSON 和 TOML 格式自动解析

### 3. 提示词管理

提示词（Prompts）是用户自定义的系统指令，可应用于不同的 AI 工具。

#### 3.1 提示词预设

**创建预设**：
1. 进入"提示词"标签页
2. 点击"创建预设"
3. 输入预设名称和描述
4. 使用 Markdown 编辑器编写提示词内容
5. 选择应用范围（Claude/Codex/Gemini）

**激活预设**：
- 在预设列表中点击"激活"按钮
- 激活后提示词将写入对应的配置文件：
  - Claude：`~/.claude/CLAUDE.md`
  - Codex：`~/.claude/AGENTS.md`
  - Gemini：`~/.gemini/GEMINI.md`

#### 3.2 编辑器功能

- 基于 CodeMirror 6 的 Markdown 编辑器
- 实时预览
- 语法高亮
- 智能回填：保留用户在配置文件中的手动修改

### 4. 技能管理

技能（Skills）是可复用的功能模块，可从 GitHub 仓库安装到 AI 工具中。

#### 4.1 技能仓库

**内置仓库**：
- 应用预配置了官方和常用的技能仓库

**添加自定义仓库**：
1. 进入"技能"标签页
2. 点击"添加仓库"
3. 输入 GitHub 仓库 URL

#### 4.2 技能安装

**扫描技能**：
- 应用会递归扫描仓库中的所有技能目录
- 自动识别符合规范的技能定义

**安装技能**：
1. 在技能列表中选择要安装的技能
2. 点击"安装"按钮
3. 技能将被安装到 `~/.claude/skills/` 目录

**管理技能**：
- 更新：拉取最新版本
- 卸载：从系统中移除

### 5. 智能代理系统

智能代理是应用的核心功能，提供高可用的 API 转发和容错机制。

#### 5.1 代理服务控制

**启动代理**：
- 在图形界面中点击"启动代理"开关
- 代理服务器将在 `127.0.0.1` 上监听随机端口
- 界面会显示代理地址，例如 `http://127.0.0.1:8080`

**配置 AI 工具使用代理**：
将生成的代理地址配置到对应工具的环境变量或配置文件中。

**停止代理**：
- 关闭开关或退出应用时自动停止

#### 5.2 多级排序算法

代理服务在选择供应商时采用多级排序算法，确保最优选择：

1. **同组优先**：优先选择与当前供应商同名的其他实例
2. **组间排序**：不同组按名称字母顺序排列
3. **轮询层级**：按 `rotation_tier` 排序（数字越小越优先，用于主力/保底分层）
4. **URL 优先级**：按 `url_priority` 排序（同组内不同 URL 的优先级）
5. **URL 延迟**：按 `url_latency` 排序（相同优先级时选择延迟更低的）
6. **组内优先级**：按 `group_priority` 排序（手动调整的组内顺序）
7. **使用均衡**：按 `usage_count` 排序（使用次数少的优先，实现负载均衡）
8. **时间分散**：按 `last_used_at` 排序（最久未使用的优先）
9. **手动排序**：按 `sort_index` 排序（最终排序依据）

#### 5.3 冷静期机制

冷静期机制采用 URL 优先策略，智能判断故障原因并避免误判。

**核心逻辑（两阶段判断）**：

1. **阶段一：失败时记录**
   - 当 API 请求失败且熔断器打开时，记录该 URL + Key 组合

2. **阶段二：成功时判断**
   - 当同一 Key 在其他 URL 成功时，检查是否在其他 URL 失败过
   - 若存在失败记录，则对失败的 URL 触发冷静期
   - 逻辑：同一 Key 在 URL-A 失败但在 URL-B 成功，说明 URL-A 有问题

**组级智能判断**：
- 若同一服务商的所有 Key×URL 组合均失败，仅输出警告而不触发冷静期
- 原因：可能是服务商整体故障或临时波动，应继续尝试其他服务商

**熔断器协同**：
- 熔断器用于短期保护（秒级到分钟级）
- 冷静期用于长期规避（小时级到天级）
- 仅在熔断器打开时才触发冷静期判断

**冷静期过滤**：
- 在供应商选择时自动过滤处于冷静期内的供应商
- 判断条件：当前时间 < `cooldownUntil` 时间戳

**配置参数**：
- `cooldownDuration`：冷静期时长（默认 72 小时 = 259,200 秒）
- `cooldownUntil`：冷静期结束的 Unix 时间戳
- 可通过批量添加时的 `--cooldown-hours` 参数自定义时长

**CLI 命令**：

查看冷静期列表：
```bash
cc-switch-cli cooldown list
```

手动设置冷静期：
```bash
cc-switch-cli cooldown set <provider-name> --hours 24
```

清除冷静期：
```bash
cc-switch-cli cooldown clear <provider-name>
```

**机制优势**：
- URL 优先：避免因 Key 失效误判而冷却所有供应商
- 智能判断：通过成功案例验证故障归因
- 避免误判：组级失败仅警告，不影响故障转移

#### 5.4 WAF 智能绕过

应用能够自动检测和绕过常见的 Web 应用防火墙（WAF）。

**支持的 WAF**：
- 阿里云 WAF（HTTP 529 XOR Challenge）
- Cloudflare WAF（预留接口）

**工作流程**：
1. 检测到 HTTP 529 响应
2. 提取 XOR mask 参数
3. 计算正确的 Cookie 值
4. 自动重试请求
5. 若失败则触发冷静期

#### 5.5 智能探测重试

为降低 token 消耗和成本，代理服务在首次失败后使用轻量级探测请求验证供应商可用性。

**工作原理**：
1. 向供应商发送完整请求
2. 若失败，生成探测请求（约 10 tokens）
3. 发送探测请求到下一个供应商
4. 若探测成功，发送完整请求
5. 若探测失败，快速跳过该供应商

**性能指标**：
- Token 节省率：70-80%
- 响应延迟减少：50%
- 探测响应时间：< 500ms
- 缓存命中率：90%+

**成本节约估算**：
- 日节约：约 $200
- 月节约：约 $6,000
- 年节约：约 $72,000

**配置选项**：
- 默认启用
- 探测结果缓存 60 秒

#### 5.6 透明代理模式

代理服务支持透明代理，无需显式指定应用类型。

**路径检测规则**：
- `/claude/*` 或 `/v1/messages` → Claude
- `/codex/*` 或 `/v1/chat/completions` → Codex
- `/gemini/*` 或 `/v1beta/*` → Gemini

**System Prompt 替换**：
- Claude：`system` 字段（字符串或数组）
- OpenAI：`messages[].role="system"`
- Gemini：`systemInstruction`

**自定义 Headers**：
支持注入自定义 HTTP 头部信息。

---

## 系统集成功能

### 1. 深度链接协议

应用支持通过 `ccswitch://` 协议进行配置分享和导入。

**协议格式**：
```
ccswitch://provider?name=<name>&url=<url>&key=<key>
```

**使用场景**：
- 一键导入供应商配置
- 团队配置分享
- 快速部署

### 2. 系统托盘

**功能**：
- 显示应用状态
- 快速切换供应商
- 访问常用功能
- 最小化到托盘

**平台支持**：
- Windows：任务栏托盘图标
- macOS：菜单栏图标（自动适配深浅色主题）
- Linux：系统托盘图标

### 3. 开机自启

**启用自启**：
1. 进入"设置"标签页
2. 勾选"开机自动启动"
3. 重启后验证

**禁用自启**：
取消勾选即可。

### 4. 自动更新

应用支持自动检查和安装更新。

**更新检查**：
- 启动时自动检查
- 手动检查：设置 → 检查更新

**更新流程**：
1. 检测到新版本
2. 提示用户下载
3. 下载完成后重启应用
4. 自动安装更新

### 5. 单实例守护

应用仅允许同时运行一个实例，重复启动时将激活已运行的窗口。

---

## 数据持久化

### 1. SQLite 数据库

**位置**：`~/.cc-switch/cc-switch.db`

**存储内容**：
- 供应商配置
- MCP 服务器配置
- 技能配置
- 提示词预设
- 用量统计
- 模型定价

### 2. JSON 配置文件

**位置**：`~/.cc-switch/settings.json`

**存储内容**：
- 窗口状态
- 本地路径
- 设备级设置

### 3. Live 配置文件

**Claude**：`~/.claude/config.json`、`~/.claude/CLAUDE.md`
**Codex**：`~/.codex/config.json`、`~/.claude/AGENTS.md`
**Gemini**：`~/.gemini/config.json`、`~/.gemini/GEMINI.md`

### 4. 备份和恢复

**数据库备份**：
- 自动备份：应用在迁移前自动创建备份
- 手动备份：复制 `~/.cc-switch/` 目录

**恢复数据**：
1. 停止应用
2. 替换数据库文件
3. 重启应用

---

## 命令行工具

### 1. 基础命令

**列出所有供应商**：
```bash
cc-switch-cli list
```

**查看当前激活的供应商**：
```bash
cc-switch-cli current
```

**切换供应商**：
```bash
cc-switch-cli enable <provider-name>
```

**添加供应商**：
```bash
cc-switch-cli add --name <name> --url <url> --key <key>
```

**删除供应商**：
```bash
cc-switch-cli remove <provider-name>
```

### 2. 批量操作

**批量添加供应商组**：
```bash
cc-switch-cli add-group \
  --name "GroupName" \
  --urls "url1,url2,url3" \
  --keys "key1,key2" \
  --cooldown-hours 72
```

### 3. 冷静期管理

**列出冷静期供应商**：
```bash
cc-switch-cli cooldown list
```

**设置冷静期**：
```bash
cc-switch-cli cooldown set <provider-name> --hours <hours>
```

**清除冷静期**：
```bash
cc-switch-cli cooldown clear <provider-name>
```

### 4. 配置管理

**导出配置**：
```bash
cc-switch-cli export <path>
```

**导入配置**：
```bash
cc-switch-cli import <path>
```

### 5. 性能测试

**测试供应商延迟**：
```bash
cc-switch-cli test <provider-name>
```

---

