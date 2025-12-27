# Project Structure

## Directory Organization

```
AnyRouter-Transparent-Proxy/
├── backend/                          # 后端服务目录
│   ├── app.py                        # FastAPI 主应用入口，生命周期管理
│   ├── config.py                     # 配置管理，环境变量和自定义请求头加载
│   ├── requirements.txt              # Python 依赖清单
│   ├── services/                     # 业务逻辑层（核心服务）
│   │   ├── proxy.py                  # 代理处理逻辑，请求/响应过滤
│   │   ├── stats.py                  # 统计收集服务，记录请求指标
│   │   └── log_storage.py            # 日志持久化服务，按日期存储
│   ├── routers/                      # 路由层（API 端点定义）
│   │   └── admin.py                  # 管理面板 API 路由
│   └── utils/                        # 工具函数层
│       └── encoding.py               # 编码处理工具
│
├── frontend/                         # 前端项目目录
│   ├── src/                          # 源代码目录
│   │   ├── main.ts                   # Vue 应用入口，初始化插件
│   │   ├── App.vue                   # 根组件，布局和路由容器
│   │   ├── router/                   # 路由配置
│   │   │   └── index.ts              # Vue Router 配置，定义页面路由
│   │   ├── views/                    # 页面组件（功能页面）
│   │   │   ├── Dashboard.vue         # 总览页，统计数据和趋势图表
│   │   │   ├── Monitoring.vue        # 实时监控页，最近请求列表
│   │   │   ├── Logs.vue              # 日志查询页，支持过滤和实时流
│   │   │   └── Config.vue            # 配置管理页，查看和修改建议
│   │   ├── components/               # 复用组件（可选）
│   │   ├── services/                 # API 服务层
│   │   │   └── api.ts                # 封装后端 API 调用，ky HTTP 客户端
│   │   ├── stores/                   # 状态管理（Pinia）
│   │   │   └── index.ts              # 全局状态管理
│   │   └── assets/                   # 静态资源（图片、样式）
│   ├── public/                       # 公共文件（不经过构建）
│   │   ├── manifest.json             # PWA Manifest 配置
│   │   └── icons/                    # PWA 图标
│   ├── dist/                         # 构建输出目录（生产环境静态文件）
│   ├── package.json                  # npm 依赖和脚本配置
│   ├── package-lock.json             # npm 依赖锁定文件
│   ├── vite.config.ts                # Vite 构建配置，包含 PWA 插件
│   ├── tsconfig.json                 # TypeScript 编译配置
│   ├── tsconfig.app.json             # 应用 TypeScript 配置
│   ├── tsconfig.node.json            # Node.js TypeScript 配置
│   └── index.html                    # HTML 入口文件
│
├── data/                             # 数据存储目录（运行时生成）
│   └── logs/                         # 日志持久化存储
│       ├── 2025-12-12.json           # 按日期分文件存储
│       ├── 2025-12-11.json
│       └── ...
│
├── env/                              # 环境配置目录
│   ├── .env.headers.json             # 自定义请求头配置（实际配置）
│   └── .env.headers.example          # 自定义请求头配置模板
│
├── .spec-workflow/                   # 规范工作流目录（项目管理）
│   ├── templates/                    # 规范模板
│   │   ├── product-template.md       # 产品规范模板
│   │   ├── tech-template.md          # 技术规范模板
│   │   └── structure-template.md     # 结构规范模板
│   └── steering/                     # 指导文档
│       ├── product.md                # 产品愿景文档
│       ├── tech.md                   # 技术架构文档
│       └── structure.md              # 项目结构文档（本文件）
│
├── .env.example                      # 环境变量配置模板
├── .env                              # 本地环境变量（不提交到版本控制）
├── Dockerfile                        # Docker 镜像构建配置（多阶段构建）
├── docker-compose.yml                # Docker Compose 编排配置
├── README.md                         # 中文项目文档
├── README_en.md                      # 英文项目文档
├── CLAUDE.md                         # AI 上下文索引（根目录）
├── backend/CLAUDE.md                 # 后端模块详细文档
├── frontend/CLAUDE.md                # 前端模块详细文档
├── .gitignore                        # Git 忽略文件配置
├── .claudeignore                     # Claude Code 忽略文件配置
└── TODO.md                           # 待办事项列表
```

### 结构设计原则

1. **前后端分离**: `backend/` 和 `frontend/` 完全独立，便于独立开发和部署
2. **分层架构**: 后端采用清晰的分层（应用层 → 路由层 → 服务层 → 工具层）
3. **配置外部化**: 环境变量和 JSON 配置文件，支持不同部署环境
4. **模块化设计**: 职责清晰，单一职责原则
5. **文档驱动**: 完整的文档体系（README、CLAUDE.md、模块文档）
6. **容器优先**: Docker 多阶段构建，一次构建包含前后端

### 架构演进历史

- **v1.0**: 单文件架构（`app.py` 403 行）- 简单快速
- **v2.0**: 模块化架构（`backend/` 分层）- 可维护性提升
- **v3.0**: 前后端分离（`frontend/` Vue 3 SPA + PWA）- 完整的管理面板

## Naming Conventions

### Files

#### Backend (Python)
- **Python 模块文件**: `snake_case.py` (例: `app.py`, `log_storage.py`)
- **目录**: `snake_case/` (例: `services/`, `routers/`, `utils/`)
- **配置文件**: 点开头+描述性名称 (例: `.env.example`)
- **依赖文件**: 标准命名 (例: `requirements.txt`)

#### Frontend (TypeScript/Vue)
- **TypeScript 文件**: `camelCase.ts` 或 `PascalCase.ts` (例: `main.ts`, `api.ts`)
- **Vue 组件**: `PascalCase.vue` (例: `Dashboard.vue`, `Logs.vue`)
- **目录**: `kebab-case/` (例: `views/`, `stores/`, `services/`)
- **配置文件**: 标准命名 (例: `vite.config.ts`, `tsconfig.json`)

#### 通用文件
- **文档文件**: `UPPER_CASE.md` 或 `kebab-case.md` (例: `README.md`, `README_en.md`)
- **Docker 文件**: 标准命名 (例: `Dockerfile`, `docker-compose.yml`)
- **环境文件**: 点开头 (例: `.env`, `.env.example`)

### Code

#### Backend (Python)
- **类**: `PascalCase` (例: `LogStorageManager`, `StatisticsCollector`)
- **函数/方法**: `snake_case` (例: `filter_request_headers`, `process_request_body`)
- **常量**: `UPPER_SNAKE_CASE` (例: `API_BASE_URL`, `SYSTEM_PROMPT_REPLACEMENT`)
- **变量**: `snake_case` (例: `http_client`, `forward_headers`)
- **私有成员**: 下划线前缀 (例: `_internal_method`)

#### Frontend (TypeScript/Vue)
- **接口/类型**: `PascalCase` (例: `LogEntry`, `StatsData`)
- **函数/方法**: `camelCase` (例: `fetchLogs`, `updateStats`)
- **常量**: `UPPER_SNAKE_CASE` (例: `API_BASE_URL`, `REFRESH_INTERVAL`)
- **变量**: `camelCase` (例: `logList`, `selectedSpec`)
- **Vue 组件名**: `PascalCase` (例: `Dashboard`, `LogsPage`)
- **组合式函数**: `use` 前缀 + `PascalCase` (例: `useStats`, `useLogs`)

#### 环境变量 (所有)
- **环境变量**: `UPPER_SNAKE_CASE` (例: `DEBUG_MODE`, `PORT`, `LOG_RETENTION_DAYS`)

## Import Patterns

### Backend (Python) Import Order
1. **标准库导入** (按字母排序)
   ```python
   import asyncio
   import json
   import os
   from datetime import datetime
   from pathlib import Path
   from typing import Dict, List, Optional
   ```

2. **第三方库导入** (按字母排序)
   ```python
   from fastapi import FastAPI, Request, Response
   from fastapi.responses import StreamingResponse
   from httpx import AsyncClient
   import uvicorn
   ```

3. **本地模块导入** (相对导入)
   ```python
   from .config import API_BASE_URL, load_env_config
   from .services.proxy import filter_request_headers
   from .services.stats import collect_request_stats
   ```

### Frontend (TypeScript) Import Order
1. **Vue 核心库**
   ```typescript
   import { createApp } from 'vue'
   import { createRouter } from 'vue-router'
   ```

2. **第三方库** (按字母排序)
   ```typescript
   import { Chart } from 'chart.js'
   import ky from 'ky'
   ```

3. **本地模块导入** (使用 `@/` 别名)
   ```typescript
   import { useAppStore } from '@/stores'
   import { fetchLogs } from '@/services/api'
   ```

4. **组件导入**
   ```typescript
   import Dashboard from '@/views/Dashboard.vue'
   ```

5. **类型导入** (单独分组)
   ```typescript
   import type { LogEntry, StatsData } from '@/types'
   ```

6. **样式导入** (最后)
   ```typescript
   import './assets/main.css'
   ```

## Code Structure Patterns

### Backend Module Organization

#### app.py (主应用入口)
```python
# 1. 导入和依赖
#    - 标准库
#    - 第三方库
#    - 本地模块
#    - 类型提示

# 2. 全局变量和配置
#    - 从 config.py 导入配置
#    - 全局 AsyncClient 实例

# 3. 生命周期管理
#    - lifespan() 函数

# 4. FastAPI 应用初始化
#    - app = FastAPI(lifespan=lifespan)
#    - 静态文件挂载
#    - 路由注册

# 5. 主程序入口
#    - if __name__ == "__main__": 启动逻辑
```

#### config.py (配置管理)
```python
# 1. 导入依赖

# 2. 常量定义
#    - 环境变量读取
#    - 默认值设置

# 3. 配置加载函数
#    - load_custom_headers()
#    - filter_request_headers()
#    - filter_response_headers()

# 4. 配置验证
#    - 启动时验证
```

#### services/*.py (业务逻辑层)
```python
# 1. 导入依赖

# 2. 类或函数定义
#    - 主要业务逻辑
#    - 辅助函数

# 3. 错误处理
#    - 异常定义和处理

# 4. 日志记录
#    - 结构化日志
```

#### routers/*.py (路由层)
```python
# 1. 导入依赖

# 2. APIRouter 实例
#    - router = APIRouter()

# 3. 端点定义
#    - @router.get/post/put/delete
#    - 请求/响应模型

# 4. 端点函数实现
#    - 参数验证
#    - 调用服务层
#    - 返回响应
```

### Frontend Module Organization

#### main.ts (应用入口)
```typescript
// 1. 导入核心库
// 2. 导入插件（router, pinia）
// 3. 导入根组件
// 4. 导入样式
// 5. 创建应用实例
// 6. 注册插件
// 7. 挂载应用
```

#### views/*.vue (页面组件)
```vue
<script setup lang="ts">
// 1. 导入依赖
// 2. Props 和 Emits 定义
// 3. 响应式状态
// 4. 计算属性
// 5. 生命周期钩子
// 6. 方法定义
</script>

<template>
  <!-- 1. 页面布局 -->
  <!-- 2. 组件调用 -->
</template>

<style scoped>
/* 1. 组件样式（使用 TailwindCSS） */
</style>
```

#### services/api.ts (API 服务层)
```typescript
// 1. 导入依赖（ky）
// 2. API 客户端配置
// 3. API 函数定义
//    - 命名规范：fetch*, create*, update*, delete*
//    - 类型注解
//    - 错误处理
```

#### stores/*.ts (状态管理)
```typescript
// 1. 导入 Pinia
// 2. 定义 store
//    - state
//    - getters
//    - actions
// 3. 导出 store
```

### Function Organization Pattern

#### Backend (Python)
```python
async def function_name(param1: Type1, param2: Type2) -> ReturnType:
    """
    函数文档字符串 (中文)

    Args:
        param1: 参数描述
        param2: 参数描述

    Returns:
        返回值描述

    Raises:
        Exception: 异常情况描述
    """
    # 1. 日志记录（可选）
    logger.info(f"[前缀] 操作描述: {param1}")

    # 2. 输入验证
    if not param1:
        raise ValueError("参数错误")

    try:
        # 3. 核心逻辑
        result = await process_logic(param1, param2)

        # 4. 结果处理
        return format_result(result)

    except Exception as e:
        # 5. 错误处理和日志
        logger.error(f"[前缀] 操作失败: {e}")
        raise
```

#### Frontend (TypeScript)
```typescript
async function fetchData(id: string): Promise<DataType> {
  try {
    // 1. 参数验证（可选）
    if (!id) throw new Error('ID is required')

    // 2. API 调用
    const response = await api.get(`/endpoint/${id}`).json<DataType>()

    // 3. 数据处理
    return processData(response)

  } catch (error) {
    // 4. 错误处理
    console.error('Failed to fetch data:', error)
    throw error
  }
}
```

## Code Organization Principles

### 1. **单一职责 (Single Responsibility)**
- 每个模块、类、函数只负责一件事
- 后端：`services/` 按功能分模块（proxy、stats、log_storage）
- 前端：`views/` 按页面分组件，`services/` 按功能分 API

### 2. **高内聚低耦合 (High Cohesion, Low Coupling)**
- 相关功能放在同一模块内
- 模块间通过明确的接口通信
- 例：`services/stats.py` 独立负责统计，不依赖 `log_storage.py`

### 3. **分层架构 (Layered Architecture)**
```
Backend:
API 层 (routers/) → 服务层 (services/) → 工具层 (utils/)

Frontend:
视图层 (views/) → 服务层 (services/) → 状态层 (stores/)
```

### 4. **配置驱动 (Configuration-Driven)**
- 环境变量控制行为（`.env`）
- JSON 配置支持复杂配置（`env/.env.headers.json`）
- 默认值合理且安全

### 5. **异步优先 (Async-First)**
- 后端：所有 I/O 操作使用 `async/await`
- 前端：所有 API 调用返回 `Promise`

### 6. **类型安全 (Type Safety)**
- 后端：Python 类型提示 (Type Hints)
- 前端：TypeScript 严格模式 (`strict: true`)

## Module Boundaries

### Backend Module Boundaries

```
┌────────────────────────────────────────────────────────────┐
│                      backend/app.py                        │
│                    (应用入口和协调层)                        │
└────────────────────────────────────────────────────────────┘
              │                       │                  │
              ▼                       ▼                  ▼
┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
│  config.py       │   │  routers/        │   │  services/       │
│  (配置管理)       │   │  (路由层)         │   │  (业务逻辑层)     │
│                  │   │                  │   │                  │
│ • 环境变量        │   │ • admin.py       │   │ • proxy.py       │
│ • JSON 配置      │   │   (管理API)      │   │ • stats.py       │
│ • 头部过滤       │   │                  │   │ • log_storage.py │
└──────────────────┘   └──────────────────┘   └──────────────────┘
                                │                       │
                                └───────────┬───────────┘
                                            ▼
                                ┌──────────────────┐
                                │  utils/          │
                                │  (工具层)         │
                                │                  │
                                │ • encoding.py    │
                                └──────────────────┘
```

### Frontend Module Boundaries

```
┌────────────────────────────────────────────────────────────┐
│                   frontend/src/main.ts                     │
│                    (应用入口和初始化)                        │
└────────────────────────────────────────────────────────────┘
              │                       │                  │
              ▼                       ▼                  ▼
┌──────────────────┐   ┌──────────────────┐   ┌──────────────────┐
│  router/         │   │  views/          │   │  stores/         │
│  (路由配置)       │   │  (页面组件)       │   │  (状态管理)      │
│                  │   │                  │   │                  │
│ • index.ts       │   │ • Dashboard.vue  │   │ • index.ts       │
│                  │   │ • Monitoring.vue │   │   (Pinia Store)  │
│                  │   │ • Logs.vue       │   │                  │
│                  │   │ • Config.vue     │   │                  │
└──────────────────┘   └──────────────────┘   └──────────────────┘
                                │
                                ▼
                      ┌──────────────────┐
                      │  services/       │
                      │  (API 服务层)     │
                      │                  │
                      │ • api.ts         │
                      │   (ky HTTP)      │
                      └──────────────────┘
```

### Interface Boundaries

#### Backend 公共接口
- **HTTP API**: FastAPI 路由端点
  - `/health` - 健康检查
  - `/{proxy_path}` - 代理端点
  - `/admin/*` - 管理面板 API
- **配置接口**: 环境变量和 JSON 文件

#### Frontend 公共接口
- **路由**: Vue Router 页面路由
  - `/` - Dashboard
  - `/monitoring` - Monitoring
  - `/logs` - Logs
  - `/config` - Config
- **API 服务**: `services/api.ts` 封装的函数

### Dependency Direction

#### Backend
```
routers/ → services/ → utils/
   ↓          ↓
config.py  ← ─┘
```

#### Frontend
```
views/ → services/ → stores/
  ↓
router/
```

## Code Size Guidelines

### File Size Limits

#### Backend
- **主应用 (app.py)**: < 200 行 (当前 ~150 行)
- **配置文件 (config.py)**: < 150 行 (当前 ~108 行)
- **服务模块 (services/*.py)**: < 300 行 (proxy.py ~150, stats.py ~250, log_storage.py ~150)
- **路由模块 (routers/*.py)**: < 400 行 (admin.py ~300 行)
- **工具模块 (utils/*.py)**: < 100 行

#### Frontend
- **页面组件 (views/*.vue)**: < 300 行
- **服务模块 (services/*.ts)**: < 200 行 (api.ts ~150 行)
- **状态管理 (stores/*.ts)**: < 200 行
- **配置文件 (*.config.ts)**: < 100 行

### Function Size Limits
- **简单函数**: < 20 行 (如 `filter_request_headers`)
- **中等函数**: < 50 行 (如 `process_request_body`)
- **复杂函数**: < 100 行 (如 `proxy`, 需要拆分)

### Complexity Limits
- **嵌套深度**: 最大 4 层
- **圈复杂度**: 单个函数 < 10
- **参数个数**: 最多 5 个，超出使用对象或关键字参数
- **类方法数**: 单个类 < 15 个方法

### Example: Well-Sized Function

#### Backend (Python)
```python
# ✅ 好的例子 - 简洁明确（12 行）
def filter_request_headers(headers: Dict[str, str]) -> Dict[str, str]:
    """过滤 HTTP 请求头，移除 hop-by-hop 头部"""
    hop_by_hop_headers = {
        "connection", "keep-alive", "proxy-authenticate",
        "proxy-authorization", "te", "trailers",
        "transfer-encoding", "upgrade", "content-length", "content-encoding"
    }

    return {
        k: v for k, v in headers.items()
        if k.lower() not in hop_by_hop_headers
    }
```

#### Frontend (TypeScript)
```typescript
// ✅ 好的例子 - 简洁明确（8 行）
async function fetchStats(): Promise<StatsData> {
  try {
    return await api.get('/admin/stats').json<StatsData>()
  } catch (error) {
    console.error('Failed to fetch stats:', error)
    throw error
  }
}
```

## Configuration Structure

### Environment Variables (.env)
```bash
# 核心配置
API_BASE_URL=https://anyrouter.top
PORT=8088
DEBUG_MODE=false

# System Prompt 配置
SYSTEM_PROMPT_REPLACEMENT=
SYSTEM_PROMPT_BLOCK_INSERT_IF_NOT_EXIST=false

# 管理面板配置
ENABLE_DASHBOARD=true

# 日志配置
LOG_PERSISTENCE_ENABLED=true
LOG_STORAGE_PATH=data/logs
LOG_RETENTION_DAYS=7
LOG_DAILY_LIMIT=1000

# 网络配置
HTTP_PROXY=
HTTPS_PROXY=
```

### Custom Headers JSON (env/.env.headers.json)
```json
{
  "User-Agent": "claude-cli/2.0.8 (external, cli)",
  "__comment": "以 __ 开头的字段会被忽略",
  "__description": "自定义请求头配置文件"
}
```

### Frontend Configuration (vite.config.ts)
```typescript
export default defineConfig({
  server: {
    port: 5173,
    proxy: {
      '/api': 'http://localhost:8088',
      '/admin': 'http://localhost:8088'
    }
  },
  build: {
    outDir: 'dist',
    sourcemap: false
  },
  plugins: [
    vue(),
    VitePWA({
      registerType: 'autoUpdate',
      // PWA 配置
    })
  ]
})
```

## Documentation Standards

### Code Comment Standards

#### Backend (Python - 中文注释)
```python
# 文件头部注释
"""
app.py - FastAPI 主应用入口

负责：
1. FastAPI 应用初始化和生命周期管理
2. 静态文件服务（前端构建产物）
3. 路由注册和 API 端点定义
"""

# 函数文档字符串（Google 风格）
async def proxy(request: Request, proxy_path: str) -> StreamingResponse:
    """
    主代理函数，处理所有转发请求

    Args:
        request: FastAPI Request 对象
        proxy_path: 代理路径（如 /v1/messages）

    Returns:
        StreamingResponse: 流式响应对象

    Raises:
        HTTPException: 上游请求失败时抛出 502 错误
    """
    # 实现...
```

#### Frontend (TypeScript - 英文注释)
```typescript
/**
 * API Service Layer
 *
 * Provides typed API functions for backend communication
 * Uses ky for modern HTTP client with better defaults
 */

/**
 * Fetch statistics data from backend
 *
 * @returns Promise resolving to StatsData
 * @throws Error if request fails
 */
async function fetchStats(): Promise<StatsData> {
  // Implementation...
}
```

### Documentation Files

- **README.md**: 完整的项目说明，包含安装、配置、使用指南
- **CLAUDE.md**: AI 上下文索引，架构图和技术细节
- **backend/CLAUDE.md**: 后端模块详细文档
- **frontend/CLAUDE.md**: 前端模块详细文档

### API Documentation

#### Backend
- **FastAPI 自动生成**: `/docs` 端点提供 Swagger UI
- **类型注解**: 完整的 Pydantic 模型支持文档生成
- **中文描述**: 确保自动文档的中文显示

#### Frontend
- **组件注释**: 每个 Vue 组件包含用途说明
- **类型定义**: 导出的类型都有 TSDoc 注释

## Testing Structure (规划中)

### Backend Testing (pytest)
```
backend/tests/
├── __init__.py
├── conftest.py                # pytest 配置和 fixtures
├── test_app.py                # 主应用测试
├── test_config.py             # 配置管理测试
├── services/
│   ├── test_proxy.py          # 代理逻辑测试
│   ├── test_stats.py          # 统计收集测试
│   └── test_log_storage.py    # 日志持久化测试
└── fixtures/
    ├── sample_requests.json   # 测试数据
    └── sample_responses.json
```

### Frontend Testing (Vitest)
```
frontend/src/
├── __tests__/
│   ├── unit/
│   │   ├── api.test.ts        # API 服务测试
│   │   └── stores.test.ts     # 状态管理测试
│   ├── components/
│   │   └── Dashboard.test.ts  # 组件测试
│   └── e2e/
│       └── app.spec.ts        # 端到端测试
└── test-utils/
    └── setup.ts               # 测试工具和 mock
```

### Test Naming Conventions
- **测试文件**: `test_*.py` (Python), `*.test.ts` (TypeScript)
- **测试类**: `TestClassName` (Python), `describe('ClassName')` (TypeScript)
- **测试方法**: `test_method_name_scenario` (Python), `it('should ...')` (TypeScript)

## Deployment Structure

### Docker Multi-Stage Build
```dockerfile
# Stage 1: Frontend Builder
FROM node:18 AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

# Stage 2: Backend Runtime
FROM python:3.12-slim
WORKDIR /app
COPY backend/requirements.txt ./
RUN pip install --no-cache-dir -r requirements.txt
COPY backend/ ./
COPY --from=frontend-builder /app/frontend/dist /app/frontend/dist
COPY env/ /app/env/

HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD python -c "import urllib.request; urllib.request.urlopen('http://localhost:8088/health')"

EXPOSE 8088
CMD ["uvicorn", "app:app", "--host", "0.0.0.0", "--port", "8088"]
```

### Runtime Environments
- **开发环境**:
  - 后端：`python backend/app.py`
  - 前端：`npm run dev` (独立 Vite dev server)
- **测试环境**: Docker Compose
- **生产环境**: Docker + 负载均衡 (Nginx/Traefik)

### Directory Mapping (Docker)
```
Host                          Container
────────────────────────────  ────────────────────
./backend/                 →  /app/
./frontend/dist/           →  /app/frontend/dist/
./env/                     →  /app/env/
./data/logs/               →  /app/data/logs/ (volume)
```

## Best Practices Summary

### General
- ✅ 保持文件小而专注
- ✅ 使用清晰的命名规范
- ✅ 添加充分的注释和文档
- ✅ 遵循单一职责原则
- ✅ 分层架构，明确模块边界

### Backend Specific
- ✅ 所有 I/O 操作使用 async/await
- ✅ 使用类型提示 (Type Hints)
- ✅ 中文注释，便于团队理解
- ✅ 使用前缀日志系统 (如 `[Proxy]`)
- ✅ 配置驱动，环境变量优先

### Frontend Specific
- ✅ 使用 Composition API (Vue 3)
- ✅ TypeScript 严格模式
- ✅ 组件化设计，复用性强
- ✅ 使用 Pinia 管理全局状态
- ✅ 响应式设计，支持移动端

### Avoid
- ❌ 单文件超过 500 行
- ❌ 函数超过 100 行
- ❌ 嵌套深度超过 4 层
- ❌ 循环依赖
- ❌ 魔法数字（使用常量）
- ❌ 全局状态污染

---

**文档版本**: v3.0 (模块化架构 + 前后端分离)
**最后更新**: 2025-12-12
