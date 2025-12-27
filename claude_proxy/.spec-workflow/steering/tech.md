# Technology Stack

## Project Type
**高性能异步 HTTP 代理服务 + 现代化 Web 管理面板** - 基于 FastAPI 的轻量级透明代理服务器，专为解决 Anthropic API 兼容性问题而设计，支持流式响应、企业级部署和实时监控管理。

## Core Technologies

### Backend Stack

#### Primary Language(s)
- **Language**: Python 3.12 (现代化异步编程支持)
- **Runtime**: CPython 官方解释器
- **Language-specific tools**:
  - pip (包管理器)
  - venv (虚拟环境)
  - Python async/await (异步编程范式)

#### Key Backend Dependencies
- **FastAPI 0.115.5**: 现代高性能 Web 框架，支持自动 API 文档生成和异步处理
- **Uvicorn 0.32.1**: ASGI 服务器，支持 HTTP/1.1 和 WebSocket，生产级性能
- **httpx 0.28.1**: 现代异步 HTTP 客户端，支持 HTTP/2 和连接池复用
- **python-dotenv 1.0.1**: 环境变量管理，支持 .env 文件配置
- **sse-starlette 2.2.1**: Server-Sent Events (SSE) 支持，用于实时日志流

### Frontend Stack

#### Primary Language(s)
- **Language**: TypeScript 5.9.3 (静态类型支持)
- **Runtime**: Node.js (构建时)
- **Package Manager**: npm

#### Key Frontend Dependencies
- **Vue 3.5.25**: 渐进式 JavaScript 框架，Composition API
- **Vite 7.2.4**: 现代化前端构建工具，HMR 热更新
- **Pinia 3.0.4**: Vue 3 官方状态管理库
- **TailwindCSS 4.0.0**: 实用优先的 CSS 框架
- **ky 1.14.1**: 基于 Fetch API 的现代 HTTP 客户端
- **Chart.js 4.5.1**: 数据可视化图表库
- **vue-chartjs**: Chart.js 的 Vue 3 集成
- **vite-plugin-pwa 0.21.1**: PWA 支持插件（离线访问、桌面安装）

### Application Architecture

#### Backend Architecture
**模块化异步代理架构**
- **设计模式**: 模块化分层架构
  - `app.py`: 主应用入口，FastAPI 应用定义和生命周期管理
  - `config.py`: 配置管理，环境变量和自定义请求头加载
  - `services/proxy.py`: 代理处理逻辑，请求/响应过滤
  - `services/stats.py`: 统计收集服务，记录请求指标
  - `services/log_storage.py`: 日志持久化服务
  - `routers/admin.py`: 管理面板 API 路由
  - `utils/encoding.py`: 编码处理工具
- **异步处理**: 基于 FastAPI + Uvicorn 的 ASGI 异步架构
- **代理模式**: 透明 HTTP 代理，完全转发请求/响应
- **生命周期**: FastAPI lifespan 事件管理 HTTP 客户端初始化和清理

#### Frontend Architecture
**现代化单页应用（SPA）**
- **设计模式**:
  - Composition API (Vue 3)
  - 组件化设计（页面组件 + 复用组件）
  - 状态管理（Pinia stores）
  - 路由管理（Vue Router）
- **页面组件**:
  - `Dashboard.vue`: 总览页，统计数据和趋势图表
  - `Monitoring.vue`: 实时监控页，最近请求列表
  - `Logs.vue`: 日志查询页，支持过滤和实时流
  - `Config.vue`: 配置管理页，查看和修改建议
- **服务层**: `services/api.ts` - 封装后端 API 调用
- **状态管理**: `stores/index.ts` - 全局状态管理
- **PWA 支持**: Service Worker + Manifest，支持离线访问和桌面安装

### Data Storage
- **Primary storage**:
  - 日志持久化：JSON 文件，按日期分文件存储（`data/logs/YYYY-MM-DD.json`）
  - 配置：环境变量 + JSON 配置文件
- **Caching**:
  - 连接池复用（httpx 内置）
  - 内存缓存（统计数据）
- **Data formats**: JSON (API 请求/响应、日志、配置)

### External Integrations
- **APIs**: AnyRouter API (https://anyrouter.top) - Anthropic API 代理服务
- **Protocols**:
  - HTTP/1.1 (主要协议)
  - HTTPS (加密传输)
  - Server-Sent Events (流式响应、实时日志流)
  - WebSocket (未来支持)
- **Authentication**:
  - API Key 转发
  - 自定义请求头注入
  - 透明身份验证

### Monitoring & Dashboard Technologies
- **Dashboard Framework**: Vue 3 + TypeScript + TailwindCSS 4
- **Real-time Communication**:
  - Server-Sent Events (SSE) - 实时日志流
  - 定时轮询 - 统计数据每 5 秒刷新
- **Visualization Libraries**:
  - Chart.js 4.5.1 - 时间趋势图表
  - vue-chartjs - Vue 3 集成
- **State Management**: Pinia 3.0.4 - Vue 3 官方状态管理
- **PWA Support**:
  - vite-plugin-pwa - 离线访问、桌面安装
  - Service Worker - 缓存策略和离线支持
  - Web App Manifest - 桌面应用体验
- **Health Monitoring**: `/health` 端点用于 Docker 容器健康检查
- **Logging**:
  - 前缀日志系统: `[Proxy]`, `[System Replacement]`, `[Custom Headers]`, `[Stream Error]`, `[Log Storage]`
  - 日志持久化: 按日期分文件，可配置保留天数
  - 可选调试模式: 详细请求/响应信息
  - 生产安全: 自动过滤敏感信息

## Development Environment

### Backend Development Tools
- **Build System**: 无需构建，Python 直接执行
- **Package Management**: pip + requirements.txt
- **Development workflow**:
  - 开发模式: `python backend/app.py`
  - 生产模式: `uvicorn backend.app:app --host 0.0.0.0 --port 8088`
  - Docker 模式: `docker-compose up -d`

### Frontend Development Tools
- **Build System**: Vite 7.2.4 (快速构建和 HMR)
- **Package Management**: npm + package.json
- **Development workflow**:
  - 开发模式: `npm run dev` (Vite dev server + HMR)
  - 构建模式: `npm run build` (生成静态文件到 `dist/`)
  - 预览模式: `npm run preview` (预览生产构建)

### Code Quality Tools
#### Backend
- **静态分析**: 可选的 pyflakes 和 mypy
- **代码格式化**: black, isort (推荐但不强制)
- **测试框架**: pytest + httpx TestClient (基础测试覆盖)
- **文档生成**: FastAPI 自动生成 `/docs` Swagger UI

#### Frontend
- **静态分析**: TypeScript 编译器（strict mode）
- **代码格式化**: Prettier (可选)
- **Linting**: ESLint (可选)
- **测试框架**: Vitest (推荐，与 Vite 集成)

### Version Control & Collaboration
- **VCS**: Git (主分支为 main)
- **Branching Strategy**: 简单的主分支开发，功能分支可选
- **代码审查**: Pull Request 流程 (可选)

### Dashboard Development
- **Live Reload**:
  - 后端：Uvicorn 自动重载
  - 前端：Vite HMR 热模块替换
- **Port Management**:
  - 后端：可配置端口 (默认 8088)
  - 前端开发服务器：默认 5173
- **Multi-Instance Support**: 支持多实例部署（通过端口配置）

## Deployment & Distribution
- **Target Platform(s)**:
  - Linux (主要生产环境)
  - Docker 容器 (推荐部署方式)
  - macOS (开发环境)
  - Windows (通过 WSL2)
- **Distribution Method**:
  - GitHub 源码
  - Docker 镜像构建（多阶段构建：前端构建 → 后端镜像）
- **Installation Requirements**:
  - **本地部署**: Python 3.12+, Node.js (仅构建前端需要)
  - **Docker 部署**: Docker + Docker Compose
  - 网络访问 AnyRouter API
- **Deployment**:
  - **Docker Compose**: 一键部署
  - **健康检查**: 30秒间隔检查 `/health`
  - **自动重启**: `unless-stopped` 策略
  - **多阶段构建**:
    1. Stage 1 (frontend-builder): Node.js 构建前端
    2. Stage 2 (backend): Python 运行时 + 前端静态文件

## Technical Requirements & Constraints

### Performance Requirements
- **启动时间**: < 5 秒
- **代理延迟**: < 200ms (不包括上游 API 延迟)
- **并发处理**: 100+ 并发连接 (单实例)
- **内存使用**: < 256MB 基础运行（含前端静态文件服务）
- **流式响应**: 支持 60+ 秒长连接
- **前端性能**:
  - 首次内容绘制（FCP）: < 1.5s
  - 可交互时间（TTI）: < 3s
  - PWA 安装包: < 2MB

### Compatibility Requirements
- **Platform Support**:
  - Linux: Ubuntu 20.04+, CentOS 8+, Alpine 3.15+
  - macOS: 11.0+ (Big Sur)
  - Windows: 10+ (通过 WSL2 推荐)
- **Browser Support** (前端):
  - Chrome/Edge 90+
  - Firefox 88+
  - Safari 14+
  - 移动端浏览器（iOS Safari 14+, Chrome Android 90+）
- **Dependency Versions**:
  - Python: 3.12+ (利用最新异步特性)
  - Node.js: 18+ (仅构建时需要)
  - FastAPI: 0.115+
  - Vue: 3.5+
  - Vite: 7.2+
- **Standards Compliance**:
  - RFC 7230 (HTTP/1.1 消息语法和路由)
  - RFC 7231 (HTTP 语义和内容)
  - SSE 规范 (流式响应)
  - PWA 标准 (Web App Manifest, Service Worker)

### Security & Compliance
- **Security Requirements**:
  - 防重定向攻击 (follow_redirects=False)
  - 请求超时保护 (60秒)
  - 敏感信息过滤 (生产环境日志)
  - HTTP 头部安全过滤
  - 前端 CSP (Content Security Policy)
- **Compliance Standards**:
  - 数据保护合规 (不记录敏感请求内容)
  - 企业安全标准 (可配置的安全策略)
  - 日志保留策略 (可配置，默认 7 天)
- **Threat Model**:
  - DoS 攻击防护 (超时和连接限制)
  - 中间人攻击防护 (HTTPS 传输)
  - 资源耗尽防护 (连接池管理、日志自动清理)
  - XSS 防护 (Vue 3 自动转义)

### Scalability & Reliability
- **Expected Load**:
  - 并发连接: 100+ (单实例)
  - 请求速率: 1,000+ req/min
  - 数据传输: 100MB+ per minute
- **Availability Requirements**:
  - 正常运行时间: 99.9%
  - 故障恢复: < 30 秒
  - 优雅关闭: 确保连接正确关闭
- **Growth Projections**:
  - 水平扩展: 支持多实例负载均衡
  - 垂直扩展: 支持 8GB+ 内存使用
  - 地理分布: 支持多地域部署

## Technical Decisions & Rationale

### Decision Log

1. **模块化架构选择**:
   - **理由**: 功能增长后需要更好的代码组织，职责分离更清晰
   - **权衡**: 适度复杂度 vs 可维护性提升
   - **演进**: 从单文件架构演进到模块化架构

2. **FastAPI + httpx 技术栈**:
   - **理由**: 现代异步框架，原生支持流式响应，生态完善
   - **权衡**: 学习成本 vs 开发效率
   - **替代方案**: Flask + aiohttp (较重), Node.js (技术栈不一致)

3. **Vue 3 + TypeScript 前端栈**:
   - **理由**:
     - Vue 3: 轻量级、易学习、Composition API 更灵活
     - TypeScript: 静态类型检查，减少运行时错误
     - Vite: 快速构建和 HMR，开发体验极佳
   - **权衡**: React 生态更大，但 Vue 3 更轻量级，适合中小型项目
   - **替代方案**: React + Next.js (过重), Svelte (生态较小)

4. **TailwindCSS 4 选择**:
   - **理由**: 实用优先的 CSS 框架，快速开发，样式一致性好
   - **权衡**: 类名冗长 vs 开发速度
   - **替代方案**: Bootstrap (较重), 原生 CSS (开发慢)

5. **PWA 支持决策**:
   - **理由**: 提供离线访问和桌面应用体验，提升用户体验
   - **实现**: vite-plugin-pwa + Service Worker
   - **权衡**: 缓存管理复杂度 vs 用户体验提升

6. **RFC 7230 严格遵循**:
   - **理由**: 确保与各种 HTTP 客户端的最大兼容性
   - **实现**: 自动过滤 hop-by-hop 头部，正确处理 Content-Length 和 Content-Encoding
   - **权衡**: 标准遵循 vs 实现复杂度

7. **System Prompt 路由限制**:
   - **理由**: 仅对 `/v1/messages` 处理，避免影响其他 API 路由
   - **实现**: 路由匹配 + 条件处理逻辑
   - **设计哲学**: 最小化干预原则

8. **BackgroundTask 资源管理**:
   - **理由**: 解决流式响应连接过早关闭问题
   - **实现**: `StreamingResponse` + `BackgroundTask(resp.aclose)`
   - **权衡**: 复杂度增加 vs 连接安全

9. **日志持久化设计**:
   - **理由**: 支持历史日志查询和审计
   - **实现**: 按日期分文件存储，JSON 格式
   - **权衡**: 磁盘空间占用 vs 数据持久化需求
   - **优化**: 自动清理过期日志，可配置保留天数

10. **SSE 实时日志流**:
    - **理由**: 无需 WebSocket 即可实现实时推送，更轻量级
    - **实现**: sse-starlette + EventSource API
    - **权衡**: 单向通信 vs 实现简单性

## Known Limitations

### 当前技术限制
- **单进程模型**:
  - **影响**: CPU 利用率受限，无法充分利用多核
  - **未来解决方案**: 多进程部署 + 负载均衡

- **内存状态**:
  - **影响**: 统计数据和日志缓存无法跨实例共享
  - **未来解决方案**: Redis 外部缓存

### 功能限制
- **协议支持**:
  - **影响**: 暂不支持 WebSocket 和 HTTP/2 代理
  - **计划时间**: 6 个月内实现 WebSocket 支持

- **实时通信**:
  - **影响**: 前端使用 SSE 单向通信，未来需要 WebSocket 双向通信
  - **计划时间**: 3 个月内添加 WebSocket 支持

- **监控能力**:
  - **影响**: 缺乏详细的性能指标（P95/P99 延迟）
  - **计划时间**: 3 个月内添加 Prometheus metrics

### 扩展性限制
- **配置管理**:
  - **影响**: 缺乏动态配置更新能力
  - **解决方案**: 实现配置热重载机制

- **安全功能**:
  - **影响**: 缺乏访问控制和用户认证
  - **解决方案**: 添加 API Key 验证和用户权限管理

### 部署限制
- **多阶段构建**:
  - **影响**: 前端变更需要重新构建整个 Docker 镜像
  - **解决方案**: 考虑前端独立部署或使用 CDN

- **网络配置**:
  - **影响**: 在复杂网络环境中的配置可能较为复杂
  - **解决方案**: 提供网络诊断工具和配置向导

### 性能优化空间
- **前端性能**:
  - **影响**: 未实现虚拟滚动、懒加载等优化
  - **解决方案**: 日志列表使用虚拟滚动，图表按需加载

- **后端缓存**:
  - **影响**: 统计数据每次请求都重新计算
  - **解决方案**: 实现智能缓存策略，定期更新统计数据
