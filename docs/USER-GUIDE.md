# CC-Switch 终端命令行使用指南

## 快速开始

```bash
# 查看所有供应商
csc ls

# 查看当前供应商
csc c

# 查看帮助
csc --help
```

## 供应商管理

### 查看供应商

```bash
# 列出所有应用的供应商
csc ls

# 只列出 claude 的供应商
csc ls claude

# 查看当前供应商
csc c
csc c claude
```

### 添加供应商

```bash
csc add <应用类型> <ID> --name <名称> --api-key <密钥> --base-url <URL> --priority <层级>

# 示例：添加 claude 供应商
csc add claude x666 \
  --name x666 \
  --api-key sk-xxxxx \
  --base-url https://example.com \
  --priority 1

# 应用类型支持：claude, codex, gemini
# 优先级层级：0 最高，数字越大优先级越低
```

### 删除供应商

```bash
csc rm <应用类型> <ID>

# 示例
csc rm claude x666
```

### 启用供应商（指定使用）

```bash
csc en <应用类型> <ID>

# 示例：指定使用 x666 供应商
csc en claude x666

# 重要：指定后该供应商将被优先使用（优先于故障转移队列）
# 需要重启代理服务器生效：csc p r
```

### 取消指定供应商（回到层级轮询）

```bash
csc dis <应用类型>

# 示例：取消 claude 的当前指定供应商
csc dis claude

# 说明：取消后系统将自动使用故障转移队列中最优先层级的供应商
# 需要重启代理服务器生效：csc p r
```

### 设置优先级层级

```bash
csc sp <应用类型> <ID> <层级>

# 示例：将 x666 设置为层级 1
csc sp claude x666 1

# 层级说明：
# - 层级 0：最高优先级（主要供应商）
# - 层级 1-N：备用供应商，只有更高层级全部失败后才使用
```

## 故障转移队列管理

### 添加到队列

```bash
csc qa <应用类型> <ID>

# 示例
csc qa claude x666
```

### 从队列移除

```bash
csc qr <应用类型> <ID>

# 示例
csc qr claude x666
```

## 延迟测试

```bash
# 测试所有 claude 供应商
csc t claude

# 测试指定供应商
csc t claude x666

# 测试说明：
# - 发送真实 API 请求 "1+1=?"
# - 测量完整响应时间（包括网络、服务器处理、响应传输）
# - 自动按延迟排序显示结果
```

## 配置导入导出

### 导出配置

```bash
csc ex <文件路径>

# 示例：导出到 .benv 目录
csc ex ~/.benv/cc-switch-backup-$(date +%Y%m%d).sql
```

### 导入配置

```bash
csc im <文件路径>

# 示例
csc im ~/.benv/cc-switch-backup-20260106.sql

# 注意：导入前会自动备份现有配置
```

## 代理服务器管理

### 启动服务器

```bash
# 前台启动（按 Ctrl+C 停止）
csc p s
csc proxy start
```

### 停止服务器

```bash
csc p x
csc proxy stop
```

### 重启服务器

```bash
csc p r
csc proxy restart

# 提示：修改供应商配置后需要重启服务器生效
```

### 查看状态

```bash
csc p st
csc proxy status
```

## 命令别名速查表

| 完整命令 | 简短别名 | 说明 |
|---------|---------|------|
| `csc list` | `csc ls` | 列出供应商 |
| `csc current` | `csc c` | 查看当前供应商 |
| `csc add` | `csc a` | 添加供应商 |
| `csc remove` | `csc rm` | 删除供应商 |
| `csc enable` | `csc en` | 启用供应商 |
| `csc disable` | `csc dis` | 取消指定供应商 |
| `csc set-priority` | `csc sp` | 设置优先级 |
| `csc add-to-queue` | `csc qa` | 添加到队列 |
| `csc remove-from-queue` | `csc qr` | 从队列移除 |
| `csc test-latency` | `csc t` | 测试延迟 |
| `csc export` | `csc ex` | 导出配置 |
| `csc import` | `csc im` | 导入配置 |
| `csc proxy start` | `csc p s` | 启动代理 |
| `csc proxy stop` | `csc p x` | 停止代理 |
| `csc proxy restart` | `csc p r` | 重启代理 |
| `csc proxy status` | `csc p st` | 查看状态 |

## 工作流示例

### 场景 1：添加新供应商并测试

```bash
# 1. 添加供应商
csc add claude x666 \
  --name x666 \
  --api-key sk-xxxxx \
  --base-url https://example.com \
  --priority 1

# 2. 添加到故障转移队列
csc qa claude x666

# 3. 测试延迟
csc t claude x666

# 4. 查看配置
csc ls claude
```

### 场景 2：指定使用某个供应商

```bash
# 1. 指定供应商
csc en claude x666

# 2. 重启代理服务器
csc p r

# 3. 查看当前供应商
csc c claude

# 重要：指定的供应商将优先使用，不受层级限制
```

### 场景 3：配置备份与恢复

```bash
# 1. 导出当前配置
csc ex ~/.benv/cc-switch-backup.sql

# 2. 做一些修改...

# 3. 如需恢复，导入之前的备份
csc im ~/.benv/cc-switch-backup.sql

# 4. 重启服务器应用配置
csc p r
```

### 场景 4：批量测试并调整优先级

```bash
# 1. 测试所有供应商
csc t claude

# 2. 根据测试结果调整优先级
csc sp claude fast-provider 0
csc sp claude slow-provider 2

# 3. 重启服务器
csc p r
```

## 故障转移机制说明

### 优先级层级

- **层级 0**：最高优先级，优先使用
- **层级 1-N**：备用层级，按顺序故障转移
- 只有当前层级所有供应商都失败后，才切换到下一层级

### 用户指定供应商

- 使用 `csc en` 指定的供应商将**优先于故障转移队列**
- 即使该供应商不在层级 0，也会优先使用
- 只有指定的供应商失败（熔断器打开）时，才会回退到故障转移队列

### 层级内轮询

- 同一层级内的供应商采用 round-robin 轮询
- 同一 URL 的多个 API key 会自动分组轮询
- URL 延迟测试会在层级切换时自动执行，结果缓存用于排序

## 常见问题

### Q: 修改配置后不生效？

A: 修改供应商配置后需要重启代理服务器：
```bash
csc p r
```

### Q: 如何临时使用某个供应商测试？

A: 指定该供应商为当前供应商，测试完成后再改回：
```bash
# 测试前
csc en claude test-provider
csc p r

# 测试...

# 测试后恢复
csc en claude original-provider
csc p r
```

### Q: 如何查看代理日志？

A:
```bash
# Rust 代理日志
tail -f ~/.cc-switch/logs/rust_proxy.log

# Python 代理日志
tail -f ~/.cc-switch/logs/claude_proxy.log
```

### Q: 数据库文件在哪里？

A: `~/.cc-switch/cc-switch.db`

自动备份在：`~/.cc-switch/backups/`

---

更多信息请参考项目 GitHub: https://github.com/farion1231/cc-switch
