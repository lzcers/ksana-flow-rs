# 模块文件结构

flow/src/flow/runner/
├── mod.rs # 模块导出
├── runner.rs # Runner 主协调器
├── scheduler.rs # 任务调度器
├── executor.rs # 任务执行器
├── exec_context.rs # 执行上下文
└── task_guard.rs # 任务守卫 (RAII)

## 核心调用链

Runner::new()
├── Scheduler::new(graph) ──► 初始化调度器
├── Executor::new(...) ─────► 初始化执行器
└── ExecutionContext::new() ─► 初始化执行上下文

Runner::run() [主循环]
├── Scheduler::pop_initial_starts() ──► 获取初始任务
│
├── Runner::start_node() ──► 启动节点
│ ├── Executor::exec() ──► 执行任务（异步）
│ │ └── Node::run() ──► 节点实际逻辑
│ └── TaskGuard ────────► 跟踪任务生命周期
│
└── 接收 TaskEvent ────────► 处理执行结果
├── ExecutionContext::set_output()
├── Scheduler::schedule_from_output() ──► 调度下游
└── Runner::start_by_specs() ─────────► 启动下游节点

## 模块职责对照表

┌─────────────────┬──────────┬────────────────────────┬─────────────────────────────────────────────┐
│ 模块 │ 职责 │ 核心结构 │ 关键方法 │
├─────────────────┼──────────┼────────────────────────┼─────────────────────────────────────────────┤
│ runner.rs │ 主协调器 │ Runner, RunnerHandle │ new(), run(), start_node() │
├─────────────────┼──────────┼────────────────────────┼─────────────────────────────────────────────┤
│ scheduler.rs │ 任务调度 │ Scheduler │ materialize_nodes(), schedule_from_output() │
├─────────────────┼──────────┼────────────────────────┼─────────────────────────────────────────────┤
│ executor.rs │ 任务执行 │ Executor │ exec() │
├─────────────────┼──────────┼────────────────────────┼─────────────────────────────────────────────┤
│ exec_context.rs │ 状态管理 │ ExecutionContext │ set_output(), set_state() │
├─────────────────┼──────────┼────────────────────────┼─────────────────────────────────────────────┤
│ task_guard.rs │ 生命周期 │ TaskGuard, TaskTracker │ 自动增减计数 │
└─────────────────┴──────────┴────────────────────────┴─────────────────────────────────────────────┘
