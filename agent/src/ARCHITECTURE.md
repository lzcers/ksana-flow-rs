# Agent Actor 架构文档

本文档详细描述 `agent_actor` 模块的设计与实现。

## 一、模块职责拆分

| 子模块 | 职责 |
|--------|------|
| `mod.rs` | 核心结构 `AgentActor` 定义，状态管理，公共 API |
| `types.rs` | 对外可见的数据类型：错误、事件、命令、句柄、结果 |
| `builder.rs` | `AgentActorBuilder` 构建器，初始化参数装配 |
| `runtime.rs` | `StepRuntime` 单步执行流程，hook 调度，模型/工具调用 |
| `loop_control.rs` | `run_loop` 循环执行，暂停/继续/取消状态机 |

---

## 二、关键 Trait

### `AgentHook`

定义于 `agents/hooks/runtime.rs`，是插件化扩展的核心机制。

```rust
#[async_trait]
pub trait AgentHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn configure_execution_policy(&self, _state: &AgentState, _policy: &mut ExecutionPolicy) {}
    fn snapshot(&self) -> Option<Value> { None }

    // 7 个生命周期钩子方法
    async fn before_step(&self, ctx: &mut StepHookContext<'_>) -> Result<HookOutcome, HookError>;
    async fn before_call_model(&self, ctx, input: &mut BeforeCallModel<'_>) -> Result<HookOutcome, HookError>;
    async fn on_model_event(&self, ctx, input: &ModelEventCtx<'_>) -> Result<HookOutcome, HookError>;
    async fn after_call_model(&self, ctx, input: &mut AfterCallModel<'_>) -> Result<HookOutcome, HookError>;
    async fn before_call_tools(&self, ctx, input: &mut BeforeCallTools<'_>) -> Result<HookOutcome, HookError>;
    async fn after_call_tools(&self, ctx, input: &mut AfterCallTools<'_>) -> Result<HookOutcome, HookError>;
    async fn after_step(&self, ctx, input: &mut AfterStep<'_>) -> Result<HookOutcome, HookError>;
}
```

**作用**：所有执行行为都通过 hook 组合实现，支持自定义扩展。

---

## 三、关键 Struct

### 1. `AgentActor<C, E>`

定义于 `agent_actor/mod.rs`，是 Agent 的核心门面结构。

```rust
pub struct AgentActor<C, E>
where
    C: ChatCapability + Send + Sync + 'static,
    E: ToolExecutor + Send + 'static,
{
    state: AgentState,           // Agent 状态（控制面）
    chat: Arc<C>,                // Chat 模型
    tool_executor: Arc<E>,       // 工具执行器
    hooks: HookRegistry,         // 生命周期 hooks
}
```

**职责**：
- 持有核心组件引用
- 提供 `run_step()` 和 `run_loop()` 两个执行入口
- 暴露状态访问 API

### 2. `StepRuntime<'a>`

定义于 `agent_actor/runtime.rs`，负责单步执行的编排。

```rust
struct StepRuntime<'a> {
    state: &'a mut AgentState,
    hooks: &'a HookRegistry,
    event_tx: Option<&'a mpsc::Sender<AgentActorEvent>>,
    scratchpad: StepScratchpad,      // 步骤间数据暂存
    execution_policy: ExecutionPolicy, // 超时策略
}
```

**职责**：
- 驱动 hook 流水线
- 编排模型调用和工具执行
- 处理流式输出

### 3. `AgentActorBuilder<C, E>`

定义于 `agent_actor/builder.rs`，实现构建器模式。

```rust
pub struct AgentActorBuilder<C, E> {
    chat: C,
    tool_executor: E,
    context: Context,
    max_iterations: usize,
    user_id: String,
    hooks: HookRegistry,
    step_timeout: Option<Duration>,
    tool_timeout: Option<Duration>,
}
```

**职责**：
- 组装 Actor 配置
- 支持链式调用
- 自动添加 `TimeoutPolicyHook`

### 4. `AgentActorHandle`

定义于 `agent_actor/types.rs`，外部控制句柄。

```rust
pub struct AgentActorHandle {
    pub cmd_tx: mpsc::Sender<AgentActorCommand>,
    pub event_rx: mpsc::Receiver<AgentActorEvent>,
}
```

**方法**：
- `pause()` - 暂停执行
- `resume()` - 恢复执行
- `cancel()` - 取消执行
- `wait()` - 等待完成并收集所有事件

### 5. `HookRegistry`

定义于 `agents/hooks/registry.rs`，hook 容器。

```rust
pub struct HookRegistry {
    hooks: Vec<Box<dyn AgentHook>>,
}
```

**职责**：
- 按顺序执行所有注册的 hook
- 提供 `default()` 组装默认 hook 链

---

## 四、关键 Enum

### `StepResult`

单步执行结果，定义于 `agent_actor/types.rs`。

```rust
pub enum StepResult {
    Continue { content, reasoning_content, tool_calls, tool_results },  // 有工具，继续
    Done { content, reasoning_content },                                 // 无工具，完成
    Error(AgentError),                                                   // 出错
}
```

### `AgentActorEvent`

运行时事件，定义于 `agent_actor/types.rs`。

```rust
pub enum AgentActorEvent {
    ContentChunk(String),           // LLM 文本流
    ReasoningChunk(String),         // 推理内容流（DeepSeek reasoner）
    StepCompleted { ... },          // 单步完成
    ToolCalls(Vec<ToolCall>),       // 请求工具
    ToolResult { call_id, success, output },  // 工具结果
    Iteration { iteration, message_count },   // 迭代完成
    MaxIterations { iteration },              // 达到上限
    Completed,                      // Agent 完成
    Cancelled,                      // 用户取消
    Error(AgentError),              // 错误
}
```

### `HookOutcome`

Hook 返回结果，定义于 `agents/hooks/runtime.rs`。

```rust
pub enum HookOutcome {
    Continue,                 // 继续执行
    Finish(StepResultDraft),  // 提前终止当前步骤
}
```

### `HookPhase`

Hook 执行阶段，用于错误报告。

```rust
pub enum HookPhase {
    BeforeStep, BeforeCallModel, OnModelEvent, AfterCallModel,
    BeforeCallTools, AfterCallTools, AfterStep,
}
```

---

## 五、架构图

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              AgentActor<C, E>                                │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  ┌─────────────────┐  │
│  │ AgentState  │  │    Chat<C>   │  │ ToolExecutor  │  │  HookRegistry   │  │
│  │  (状态)      │  │   (模型)     │  │   (工具)      │  │    (钩子链)     │  │
│  └─────────────┘  └──────────────┘  └───────────────┘  └─────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
                    ┌───────────────┴───────────────┐
                    ▼                               ▼
        ┌──────────────────────┐      ┌──────────────────────────┐
        │   run_loop()         │      │      run_step()          │
        │   (loop_control.rs)  │      │      (runtime.rs)        │
        └──────────────────────┘      └──────────────────────────┘
                    │                               │
                    ▼                               ▼
        ┌──────────────────────┐      ┌──────────────────────────┐
        │   AgentActorHandle   │      │      StepRuntime         │
        │   - cmd_tx           │      │  ┌─────────────────────┐ │
        │   - event_rx         │      │  │  StepScratchpad     │ │
        │                      │      │  │  ExecutionPolicy    │ │
        │   方法:              │      │  └─────────────────────┘ │
        │   - pause()          │      │                          │
        │   - resume()         │      │  执行流程:               │
        │   - cancel()         │      │  before_step()           │
        │   - wait()           │      │      ▼                   │
        └──────────────────────┘      │  before_call_model()     │
                                      │      ▼                   │
                                      │  stream_model_output()   │
                                      │      │                   │
                                      │      ├── on_model_event()│
                                      │      ▼                   │
                                      │  after_call_model()      │
                                      │      ▼                   │
                                      │  [有工具调用?]            │
                                      │      ├── 是 ──┐          │
                                      │      └── 否 ──┴─► Done   │
                                      │                ▼          │
                                      │         before_call_tools()│
                                      │                ▼          │
                                      │         execute_tools()   │
                                      │                ▼          │
                                      │         after_call_tools()│
                                      │                ▼          │
                                      │             Continue      │
                                      │                          │
                                      │  after_step() ◄──────────┤
                                      └──────────────────────────┘
```

---

## 六、Hook 流水线

默认 hook 链通过 `HookRegistry::default()` 组装：

```
┌─────────────────────────────────────────────────────────────────────────┐
│                           HookRegistry::default()                        │
├─────────────────────────────────────────────────────────────────────────┤
│  1. MaxIterationsHook      ── 检查迭代上限                               │
│  2. LifecycleHook          ── 状态转换 (Pending→Running 等)              │
│  3. MetricsHook            ── 执行指标统计                               │
│  4. StreamingEventHook     ── 发送 ContentChunk/ReasoningChunk 事件      │
│  5. ContextPersistenceHook ── 持久化消息到 Context                       │
│  6. IterationEventHook     ── 发送 Iteration 事件                        │
│  7. ErrorEventHook         ── 错误事件处理                               │
│  [可选] TimeoutPolicyHook  ── 步骤/工具超时控制                          │
└─────────────────────────────────────────────────────────────────────────┘
```

各 Hook 职责详见 `agents/hooks/` 目录下对应文件。

---

## 七、执行流程

### 单步执行 (`run_step`)

```
before_step()
    │
    ▼
before_call_model()
    │
    ▼
stream_model_output()  ◄─── call_model() ◄─── 模型 API
    │
    ├── on_model_event() (每个 chunk)
    │
    ▼
after_call_model()
    │
    ▼
[有工具调用?]
    │
    ├── 否 ──────────────────────────► Done
    │
    ├── 是
    │     │
    │     ▼
    │   before_call_tools()
    │     │
    │     ▼
    │   execute_tools()  ◄─── call_tools() ◄─── 工具执行
    │     │
    │     ▼
    │   after_call_tools()
    │     │
    │     ▼
    │   Continue
    │
    ▼
after_step()
```

### 循环执行 (`run_loop`)

```
spawn(tokio task)
    │
    ▼
┌─────────────────────────────┐
│         主循环               │
│  ┌───────────────────────┐  │
│  │  drain_commands()     │  │  ◄── 处理暂停/继续/取消命令
│  └───────────────────────┘  │
│              │              │
│              ▼              │
│  ┌───────────────────────┐  │
│  │  is_terminal()?       │  │  ◄── 检查是否已取消
│  └───────────────────────┘  │
│              │              │
│              ▼              │
│  ┌───────────────────────┐  │
│  │  wait_if_paused()     │  │  ◄── 暂停时阻塞等待
│  └───────────────────────┘  │
│              │              │
│              ▼              │
│  ┌───────────────────────┐  │
│  │  run_step()           │  │  ◄── 执行单步
│  └───────────────────────┘  │
│              │              │
│              ▼              │
│  ┌───────────────────────┐  │
│  │  handle_step_result() │  │  ◄── 处理结果，决定是否继续
│  └───────────────────────┘  │
│              │              │
│              └──────────────┤
│                             │
└─────────────────────────────┘
```

---

## 八、控制流

```
用户代码                     AgentActor                      Hooks                     模型/工具
   │                           │                              │                          │
   │  ActorBuilder::new()      │                              │                          │
   │  .context()               │                              │                          │
   │  .max_iterations()        │                              │                          │
   │  .build()                 │                              │                          │
   │ ─────────────────────────►│                              │                          │
   │                           │                              │                          │
   │  run_loop()               │                              │                          │
   │ ─────────────────────────►│                              │                          │
   │                           │  spawn(tokio task)           │                          │
   │  ◄────────────────────────│                              │                          │
   │  AgentActorHandle         │                              │                          │
   │                           │                              │                          │
   │  pause()                  │                              │                          │
   │ ─────────────────────────►│  LoopState::Paused           │                          │
   │                           │                              │                          │
   │  resume()                 │                              │                          │
   │ ─────────────────────────►│  LoopState::Running          │                          │
   │                           │                              │                          │
   │  cancel()                 │                              │                          │
   │ ─────────────────────────►│  LoopState::Cancelled        │                          │
   │  ◄── Cancelled ───────────│                              │                          │
   │                           │                              │                          │
   │  wait()                   │                              │                          │
   │ ─────────────────────────►│  收集所有事件                 │                          │
   │  ◄── Vec<AgentActorEvent> │                              │                          │
```

---

## 九、设计原则

1. **职责分离**
   - `AgentActor` 只做组合，不包含执行逻辑
   - `StepRuntime` 负责单步编排
   - `loop_control` 负责循环控制
   - `builder` 负责配置组装

2. **Hook 驱动**
   - 所有扩展行为通过 hook 实现
   - 默认行为通过 `HookRegistry::default()` 组装
   - 自定义行为通过 `AgentActorBuilder::hook()` 添加

3. **流式优先**
   - `call_model()` 返回 Stream
   - `on_model_event()` hook 实时处理每个 chunk
   - 支持 DeepSeek reasoner 模式的 `reasoning_content`

4. **异步控制**
   - 通过 channel 实现 pause/resume/cancel
   - 控制逻辑与执行逻辑解耦
   - 支持实时事件推送

5. **Builder 模式**
   - 灵活组装 Actor 配置
   - 支持自定义 hook 链
   - 自动处理超时策略

---

## 十、使用示例

### 基本用法

```rust
let actor = AgentActorBuilder::new(model, tool_executor)
    .context(context)
    .max_iterations(10)
    .step_timeout(Duration::from_secs(60))
    .build();

let handle = actor.run_loop();

// 实时处理事件
while let Some(event) = handle.event_rx.recv().await {
    match event {
        AgentActorEvent::ContentChunk(text) => print!("{}", text),
        AgentActorEvent::Completed => break,
        _ => {}
    }
}
```

### 手动控制

```rust
let mut actor = AgentActorBuilder::new(model, tool_executor)
    .context(context)
    .build();

loop {
    let result = actor.run_step(None).await;
    match result {
        StepResult::Continue { .. } => continue,
        StepResult::Done { .. } => break,
        StepResult::Error(e) => return Err(e.into()),
    }
}
```

### 自定义 Hook

```rust
struct MyHook;

#[async_trait]
impl AgentHook for MyHook {
    fn name(&self) -> &'static str { "MyHook" }

    async fn before_call_model(
        &self,
        ctx: &mut StepHookContext<'_>,
        _input: &mut BeforeCallModel<'_>,
    ) -> Result<HookOutcome, HookError> {
        // 自定义逻辑
        Ok(HookOutcome::Continue)
    }
}

let actor = AgentActorBuilder::new(model, tool_executor)
    .hook(MyHook)
    .build();
```