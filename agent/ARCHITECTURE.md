# AgentActor 模块解耦与 Hook 架构设计

## 背景

`agent/src/agents/agent_actor.rs` 原先同时承担以下职责：

- 对外暴露 `AgentActor`、`Builder`、事件、命令、错误类型
- `run_step` 的执行编排
- `run_loop` 的暂停、继续、取消控制
- 超时与工具执行包装

随着 Hook 体系重新引入，单文件同时承载控制面和执行面会让以下问题逐渐放大：

- 文件过长，难以定位责任边界
- `run_step` 和 `run_loop` 的关注点混在一起
- Hook 运行时细节影响 `AgentActor` 可读性
- 文档和实现容易逐步漂移

本次重构目标是让 `agent_actor` 只保留高层壳层语义，把可独立演化的实现细节拆到子模块。

## 设计目标

1. `agent_actor.rs` 保持足够小，只负责公开 API、核心字段和最小编排。
2. 单步执行、循环控制、类型定义、构建器分别拥有独立文件。
3. 保持对外 API 基本不变，避免影响调用方。
4. Hook 成为执行期扩展点，而不是把副作用重新塞回 `AgentActor`。
5. 让默认行为由 `HookRegistry::default()` 组合出来，而不是散落在 `run_step` 里。

## 模块划分

当前结构：

```text
agent/src/agents/
├── agent_actor.rs
├── agent_actor/
│   ├── builder.rs
│   ├── loop_control.rs
│   ├── runtime.rs
│   └── types.rs
├── hooks/
│   ├── mod.rs
│   ├── registry.rs
│   ├── runtime.rs
│   ├── lifecycle.rs
│   ├── max_iterations.rs
│   ├── metrics.rs
│   ├── streaming_events.rs
│   ├── context_persistence.rs
│   ├── iteration_events.rs
│   ├── error_events.rs
│   └── timeout_policy.rs
├── agent_state.rs
├── agent_utils.rs
├── context.rs
└── tools/
```

各文件职责：

- `agent_actor.rs`
  - `AgentActor<C, E>` 的结构体定义
  - `new` / `with_hooks`
  - `state/context/hooks` 访问器
  - 统一事件发送入口
  - 对子模块类型的 re-export
- `agent_actor/types.rs`
  - `AgentError`
  - `AgentActorEvent`
  - `AgentActorCommand`
  - `AgentActorHandle`
  - `StepResult`
  - `StepResultDraft -> StepResult` 转换
- `agent_actor/runtime.rs`
  - `StepRuntime`
  - `run_step`
  - 模型流式读取与工具执行包装
  - `after_step` 收尾与错误收敛
- `agent_actor/loop_control.rs`
  - `LoopState`
  - `run_loop`
  - Pause / Continue / Cancel 控制
  - 后台循环的终止条件
- `agent_actor/builder.rs`
  - `AgentActorBuilder`
  - 组装 `Context` / `max_iterations` / `user_id`
  - 按需注入 `TimeoutPolicyHook`

## AgentActor 的责任边界

`AgentActor` 现在只持有四类核心资源：

- `state: AgentState`
- `chat: Arc<C>`
- `tool_executor: Arc<E>`
- `hooks: HookRegistry`

也就是说，`AgentActor` 负责“拥有资源”和“暴露入口”，不再直接承载完整执行过程。

这让后续演进更清晰：

- 要改执行顺序，主要看 `agent_actor/runtime.rs`
- 要改循环控制，主要看 `agent_actor/loop_control.rs`
- 要改默认行为，主要看 `hooks/registry.rs`
- 要加扩展能力，优先通过 Hook 落地

## Step 执行流程

`run_step` 的主流程如下：

```text
AgentActor::run_step
├── 从 HookRegistry 生成 ExecutionPolicy
├── 创建 StepRuntime
├── 如配置了 step timeout，则包裹 execute_core
├── StepRuntime::execute_core
│   ├── before_step
│   ├── before_call_model
│   ├── stream_model_output
│   ├── after_call_model
│   ├── before_call_tools
│   ├── execute_tools_with_timeout
│   └── after_call_tools
└── StepRuntime::finish
    ├── after_step
    └── StepResultDraft -> StepResult
```

几个关键点：

- `stream_model_output` 只负责把 `call_model` 产生的流聚合成 `ModelCallOutput`。
- 是否发送事件、是否持久化 context、是否更新 metrics，不由 `runtime.rs` 写死，而由 hooks 决定。
- `StepResultDraft` 是 Hook 运行时内部结果；对外统一暴露 `StepResult`。

## Hook 架构

Hook 的核心协议定义在 `agent/src/agents/hooks/runtime.rs`：

- `AgentHook`
- `HookPhase`
- `HookOutcome`
- `StepHookContext`
- `ExecutionPolicy`
- `StepScratchpad`

### Hook Phase

执行期共有 7 个 phase：

1. `BeforeStep`
2. `BeforeCallModel`
3. `OnModelEvent`
4. `AfterCallModel`
5. `BeforeCallTools`
6. `AfterCallTools`
7. `AfterStep`

### HookRegistry 默认链路

`HookRegistry::default()` 当前按如下顺序注册：

1. `MaxIterationsHook`
2. `LifecycleHook`
3. `MetricsHook`
4. `StreamingEventHook`
5. `ContextPersistenceHook`
6. `IterationEventHook`
7. `ErrorEventHook`

这条默认链路的职责分布很明确：

- `MaxIterationsHook` 负责提前终止
- `LifecycleHook` 负责 `JobState` 与 `iteration`
- `MetricsHook` 负责执行统计
- `StreamingEventHook` 负责流式事件对外发送
- `ContextPersistenceHook` 负责把 Assistant/Tool 结果写回 context
- `IterationEventHook` 负责迭代完成事件
- `ErrorEventHook` 负责错误事件发射

因此，`run_step` 可以专注于“拿到模型输出和工具结果”，而不是在主流程里夹杂状态写回和事件逻辑。

### ExecutionPolicy

Hook 还可以通过 `configure_execution_policy` 注入执行策略，目前主要用于超时：

- `step_timeout`
- `tool_timeout`

`TimeoutPolicyHook` 是这类策略 Hook 的实现，Builder 在设置超时时会自动追加它。

### StepScratchpad

`StepScratchpad` 是单步作用域的临时数据容器，适合放：

- 本步开始时间
- 事件计数器
- Hook 间共享但不需要持久化到 `AgentState` 的上下文

它的意义是避免 Hook 之间为了传递临时数据而污染 `AgentState`。

## Loop 控制流程

`run_loop` 已被独立到 `agent_actor/loop_control.rs`，责任是：

- 管理 `LoopState`
- 消费控制命令
- 在暂停时阻塞等待
- 在取消或 channel 关闭时终止
- 调用 `run_step` 并依据 `StepResult` 决定是否继续

状态机比较简单：

```text
Running <-> Paused
Running -> Cancelled
Paused  -> Cancelled
```

设计原则是让 `run_loop` 只关心控制平面，不重复实现 step 内部的业务逻辑。

## Builder 设计

`AgentActorBuilder` 保留为面向使用者的组装入口，负责：

- 初始化 `Context`
- 配置 `max_iterations`
- 配置 `user_id`
- 替换或追加 Hook
- 在需要时自动注入 `TimeoutPolicyHook`

这让常规使用者不需要了解 `ExecutionPolicy` 和 Hook 链的内部细节。

## 当前架构收益

重构后的直接收益：

- `agent_actor.rs` 从大体量实现文件变为轻量 facade
- 控制面和执行面边界更清晰
- Hook 体系成为明确的一等扩展点
- 事件、状态、context 持久化逻辑可以独立测试
- 新增能力时更容易决定应该落在 `runtime`、`loop_control` 还是某个 hook

## 后续约束

后续继续演进时建议遵守以下边界：

1. 不要把新的副作用直接塞回 `AgentActor::run_step`。
2. 需要修改执行行为时，优先评估是否应通过 Hook 实现。
3. 只有真正属于 actor 资源拥有关系的内容，才放回 `agent_actor.rs`。
4. `loop_control.rs` 不应重复承担 context 写回或事件细节。
5. 默认行为变更时，先检查 `HookRegistry::default()` 顺序是否受影响。

## 结论

现在的 `AgentActor` 架构已经从“一个大文件包办全部逻辑”，收敛为“薄 facade + runtime + loop control + builder + types + hooks”。

这套结构更适合继续扩展：

- 增加新的状态策略
- 定制事件发射
- 插入审计、缓存、压缩、记忆等 hook
- 为不同 agent 类型复用统一的执行骨架

在不破坏外部 API 的前提下，这次拆分把 `AgentActor` 的职责压回到了合理范围。
