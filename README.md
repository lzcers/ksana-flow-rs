# ksana-flow-rs

> 「我所不能造者，我亦不能解。」  
> "What I cannot create, I do not understand." —— Richard Feynman

**ksana-flow** 是一个基于 DAG（有向无环图）的可视化 LLM 工作流构建与执行平台。你可以在画布上拖拽节点、连线、配置参数，构建复杂的 AI 应用流水线——从简单的 LLM 调用到多步骤的数据处理、量化交易回测、图文生成，一切皆可编排。

---

## 核心特性

- **可视化工作流编辑器** — 基于 ReactFlow 的拖拽式画布，支持节点创建、连线、分组与子图嵌套
- **强大的 DAG 执行引擎** — 用 Rust 编写的高性能异步运行时，支持条件分支、子图嵌套、流式输出
- **丰富的内置节点** — LLM 调用、文本处理、图片生成、量化交易、定时器、通知等开箱即用
- **实时运行反馈** — WebSocket 推送执行状态、流式输出、错误信息，所有节点在画布上实时更新
- **前后端分离** — Rust 后端 (Axum) + React 前端 (Vite)，API 与 UI 完全解耦
- **可扩展节点系统** — 前后端一致的类型注册机制，新增节点只需按约定实现接口即可

---

## 项目架构

```
ksana-flow-rs/
├── flow/           # 核心工作流引擎 (Rust 库)
├── nodes/          # 预定义节点实现 (Rust 库)
├── server/         # HTTP + WebSocket 服务端 (Axum)
├── web/            # 可视化前端 (React + Vite + ReactFlow)
└── ksana.db        # SQLite 数据库 (工作流持久化)
```

### flow — 核心引擎

一个通用的基于图的异步工作流执行库，提供：

| 模块 | 职责 |
|------|------|
| `graph` | DAG 图定义、编译、子图折叠；提供 `Node` trait、 `GraphBuilder` fluent API 与 `build_flow!` 宏 |
| `runner` | 异步运行时：调度器 (`Scheduler`)、执行器 (`Executor`)、执行上下文、事件系统 |
| `controller` | 控制面：Runner 注册表、命令广播、事件聚合、`task_local` 作用域传播 |
| `reactive_stream` | 流式输出桥接：`futures::Stream` → `TaskEvent`，支持逐条调度与聚合 |

**设计亮点：**

- **控制面与执行面分离** — `Controller` 通过 `broadcast` 发命令、`mpsc` 收事件，Runner 间通过注册表建立父子关系
- **`task_local` 上下文传播** — 节点代码在任何嵌套深度都能通过 `try_controller()` / `try_runner_id()` 获取当前执行上下文
- **子图复用** — `SubgraphExecutor` 通过同一个 `Controller` 创建子 Runner，实现嵌套工作流，支持超时控制与上下文继承
- **容错设计** — `catch_unwind` 包裹节点执行，panic 不会击穿 Runner
- **流式与一次性统一** — `Output` 同时承载 `value` 与 `stream`，下游调度对两者一致处理

详细架构设计见 [flow/ARCHITECTURE.md](flow/ARCHITECTURE.md)。

### nodes — 内置节点库

预定义的 LLM 工作流节点，涵盖：

| 分类 | 节点 | 说明 |
|------|------|------|
| **LLM** | `LLMNode` | 大语言模型调用，支持系统/用户/助手多轮对话，流式输出 |
| **文本** | `TextNode` / `TextSplitNode` / `TextMergeNode` / `TextFileNode` | 文本输入、分割、合并、文件读写 |
| **图片** | `ImgGenNode` | AI 图片生成 |
| **交易** | `TradeSourceNode` / `KNode` / `StrategyNode` / `BacktesterNode` | 行情数据源接入、K 线分析、策略执行、回测引擎 |
| **控制流** | `MapNode` / `ReduceNode` | Map-Reduce 并行处理范式 |
| **工具** | `TimerNode` / `NotifyNode` / `VarNode` / `PromptNode` | 定时触发、通知、变量存储、提示词管理 |

节点系统采用前后端双重注册机制，确保类型安全与运行时一致性。自定义节点开发指南见 [web/NODE_SYSTEM.md](web/NODE_SYSTEM.md)。

### server — 服务端

基于 Axum 的 HTTP 服务端，提供：

```
POST   /api/workflows          # 创建工作流
GET    /api/workflows          # 列出所有工作流
GET    /api/workflows/:id      # 获取工作流详情 (Blueprint)
PUT    /api/workflows/:id      # 更新工作流
DELETE /api/workflows/:id      # 删除工作流
POST   /api/workflow/run       # 运行工作流
POST   /api/workflow/run_node  # 单节点运行
GET    /api/workflow/:id/status  # 查询运行状态
POST   /api/workflow/:id/pause   # 暂停
POST   /api/workflow/:id/resume  # 恢复
POST   /api/workflow/:id/stop    # 停止
GET    /api/nodes              # 获取可用节点类型列表
POST   /api/upload             # 文件上传
GET    /api/files/:id          # 文件下载
GET    /api/ai_media/:id       # AI 生成媒体
WS     /ws                     # WebSocket 连接 (运行事件推送)
```

运行状态通过 WebSocket 实时推送到前端，每个节点在画布上即时反映其执行状态、输出与错误信息。

### web — 前端

基于 React 19 + Vite + ReactFlow 的现代化前端：

- **画布编辑器** — 拖拽创建节点、连线构建 DAG、右键菜单、分组折叠
- **节点系统** — 可复用的 `NodeWrapper` / `FormNodeView` 基础组件，统一的端口系统与连线校验
- **配置面板** — 每个节点的参数配置表单，支持 IME 输入、草稿状态管理
- **运行监控** — WebSocket 驱动的实时状态回写，支持暂停/恢复/停止
- **工作流管理** — 多工作流切换、保存、导入导出

技术栈：React 19 · TypeScript 6 · ReactFlow 12 · Zustand 5 · Tailwind CSS 4 · Lucide React · Immer · RxJS

---

## 快速开始

### 环境要求

- Rust 1.85+ (edition 2024)
- Node.js 22+ / pnpm
- SQLite 3

### 启动后端

```bash
# 在项目根目录
cargo run -p server

# 服务启动在 http://localhost:3000
```

### 启动前端

```bash
cd web
pnpm install
pnpm dev

# 开发服务器启动在 http://localhost:5173
```

### 构建前端

```bash
cd web
pnpm build

# 产物输出到 web/dist
```

---

## 开发指南

### 新增一个节点

节点系统遵循「后端注册 → 前端 metadata → 前端 manifest」三段式闭环。以最简单的表单型节点为例：

1. **后端** — 在 `server/src/registry.rs` 注册节点类型名、默认配置与运行时构造器
2. **前端 metadata** — 在 `web/src/components/nodes/MyNode/metadata.ts` 定义端口、图标、分类、默认配置
3. **前端 manifest** — 在 `web/src/components/nodes/MyNode/manifest.ts` 绑定 metadata 与 React 组件
4. **接入注册表** — 修改 `web/src/components/nodes/manifests.ts` 与 `metadata.ts`

完整步骤与容器节点（如子图）的开发细节见 [web/NODE_SYSTEM.md](web/NODE_SYSTEM.md)。

### 引擎设计细节

工作流引擎的完整架构文档（C4 Model）：[flow/ARCHITECTURE.md](flow/ARCHITECTURE.md)，包含：

- System Context 系统上下文图
- Container 容器视图
- Component 组件视图（图定义与编译、运行时执行、子图执行流、流式输出）
- 关键运行时序列图
- 设计要点与约束

---

## 项目灵感

ksana（剎那）是佛教中表示极短时间单位的梵语词汇。这个项目的初衷是探索 AI 应用开发的基本范式——让复杂的 LLM 工作流像流水一样自然流动，在剎那间完成编排与执行。

作为一个 side project，它承载着对 AI 编程能力极限的好奇与实验。"just for fun, but serious about engineering."
