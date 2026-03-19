# Agent Actor 架构文档

本文档描述 `agent_actor` 模块当前的设计与实现。

当前的 hook 架构已经收口为两层职责，但对外只暴露一套扩展协议：

- 内部运行时层：`RuntimeHook`
- 对外扩展层：`Hook`

其中：

- `RuntimeHook` 只作为框架内部实现细节存在
- `Hook` 是唯一公开的扩展协议

## 一、模块职责拆分

| 子模块            | 职责                                                   |
| ----------------- | ------------------------------------------------------ |
| `mod.rs`          | 核心结构 `AgentActor` 定义，状态管理，公共 API         |
| `types.rs`        | 对外可见的数据类型：错误、事件、命令、句柄、结果       |
| `builder.rs`      | `AgentActorBuilder` 构建器，初始化参数装配             |
| `lifecycle.rs`    | `StepLifecycle` 单步执行流程，hook 调度，模型/工具调用 |
| `loop_control.rs` | `run_loop` 循环执行，暂停/继续/取消状态机              |

---

## 二、Hook 分层设计

### 1. 内部运行时 Hook：`RuntimeHook`

定义于 `agents/hooks/runtime.rs`。

```rust
#[async_trait]
pub(crate) trait RuntimeHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn configure_execution_policy(&self, _state: &AgentState, _policy: &mut ExecutionPolicy) {}
    fn snapshot(&self) -> Option<Value> { None }

    async fn before_step(&self, ctx: &mut StepHookContext<'_>) -> Result<HookOutcome, HookError>;
    async fn before_call_model(&self, ctx: &mut StepHookContext<'_>, input: &mut BeforeCallModel<'_>) -> Result<HookOutcome, HookError>;
    async fn on_model_event(&self, ctx: &mut StepHookContext<'_>, input: &ModelEventCtx<'_>) -> Result<HookOutcome, HookError>;
    async fn after_call_model(&self, ctx: &mut StepHookContext<'_>, input: &mut AfterCallModel<'_>) -> Result<HookOutcome, HookError>;
    async fn before_call_tools(&self, ctx: &mut StepHookContext<'_>, input: &mut BeforeCallTools<'_>) -> Result<HookOutcome, HookError>;
    async fn after_call_tools(&self, ctx: &mut StepHookContext<'_>, input: &mut AfterCallTools<'_>) -> Result<HookOutcome, HookError>;
    async fn after_step(&self, ctx: &mut StepHookContext<'_>, input: &mut AfterStep<'_>) -> Result<HookOutcome, HookError>;
}
```

特点：

- 基于借用上下文工作，能直接访问 `AgentState`、事件发送器和 step scratchpad
- 覆盖完整运行时生命周期
- 适合框架内部能力，如状态迁移、持久化、流式事件、指标、超时策略
- 仍然保留生命周期参数，因为它本质上是“运行时内部 API”

### 2. 对外 Hook：`Hook`

定义于 `agents/hooks/public.rs`。

```rust
#[async_trait]
pub trait Hook: Send + Sync {
    fn name(&self) -> &'static str;

    async fn before_step(&self, input: BeforeStepInput) -> Result<Vec<HookEffect>, HookError>;
    async fn on_model_event(&self, input: ModelEventInput) -> Result<Vec<HookEffect>, HookError>;
    async fn after_step(&self, input: AfterStepInput) -> Result<Vec<HookEffect>, HookError>;
}
```

特点：

- 公开表面不暴露借用上下文，也不暴露生命周期参数
- 输入是可序列化、可复制、面向稳定协议的快照
- 输出不是直接改内部状态，而是返回受控的 `HookEffect`
- 当前只开放 3 个阶段：
    - `before_step`
    - `on_model_event`
    - `after_step`

这是一种“输入快照 + 返回 effect”的扩展模型，目标不是给外部扩展全部运行时权限，而是给一个稳定、可演进、边界清晰的扩展协议。

### 3. 对外 Hook 的 effect 边界

```rust
pub enum HookEffect {
    EmitEvent(HookEvent),
    ReplaceResult(HookStepUpdate),
    Abort { reason: String },
    SetMetadata { key: String, value: Value },
    RemoveMetadata { key: String },
}
```

约束：

- `EmitEvent`：发送 `AgentActorEvent::HookEvent`
- `ReplaceResult`：只允许在 `after_step` 使用
- `Abort`：立即中止当前 step，并转换为 `AgentError::Hook`
- `SetMetadata` / `RemoveMetadata`：读写当前 step 作用域内的公共 metadata

这里刻意保持 effect 集合很窄，避免公共扩展直接耦合内部运行时结构。

### 4. 为什么要双层 Hook

原因不是“原设计错误”，而是目标不同：

- `RuntimeHook` 追求的是内部实现效率和编排能力
- `Hook` 追求的是公共可扩展性、协议稳定性和低耦合

因此当前结构是有意分层，而不是简单替换：

- 内部层保留 borrow-based runtime API
- 对外层改成 snapshot/effect API

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
    state: AgentState,
    chat: Arc<C>,
    tool_executor: Arc<E>,
    runtime_hooks: RuntimeHookRegistry,
    hooks: HookRegistry,
}
```

职责：

- 持有模型、工具执行器、状态与两类 hook 注册表
- 提供 `run_step()` 和 `run_loop()` 两个执行入口
- 对外只暴露扩展 `HookRegistry`

相关 API：

- `with_hooks(...)`
- `hooks()` / `hooks_mut()`
- `add_hook(...)`
- `with_runtime_hooks(...)` 仅用于 crate 内部

### 2. `StepLifecycle<'a>`

定义于 `agent_actor/lifecycle.rs`，负责单步执行编排。

```rust
struct StepLifecycle<'a> {
    state: &'a mut AgentState,
    event_tx: Option<&'a mpsc::Sender<AgentActorEvent>>,
    hooks: HookPipeline<'a>,
    execution_policy: ExecutionPolicy,
}
```

职责：

- 驱动单步生命周期
- 编排模型流、工具调用与结果组装
- 把 hook 调度委托给统一的 `HookPipeline`

补充：

- `execution_policy` 仍由内部 `HookRegistry` 计算
- `StepLifecycle` 本身不再显式区分 public / internal hook
- 公共 metadata 和 scratchpad 都收敛进 `HookPipeline` 内部

### 3. `AgentActorBuilder<C, E>`

定义于 `agent_actor/builder.rs`。

```rust
pub struct AgentActorBuilder<C, E> {
    chat: C,
    tool_executor: E,
    context: Context,
    max_iterations: usize,
    user_id: String,
    runtime_hooks: RuntimeHookRegistry,
    hooks: HookRegistry,
    step_timeout: Option<Duration>,
    tool_timeout: Option<Duration>,
}
```

职责：

- 组装 Actor 配置
- 对外支持链式添加扩展 `Hook`
- 在需要时自动附加 `TimeoutPolicyHook`

相关 API：

- `hook(...)`
- `hooks(...)`
- `runtime_hook(...)` / `runtime_hooks(...)` 仅用于 crate 内部

### 4. `AgentActorHandle`

定义于 `agent_actor/types.rs`，是外部控制句柄。

```rust
pub struct AgentActorHandle {
    pub cmd_tx: mpsc::Sender<AgentActorCommand>,
    pub event_rx: mpsc::Receiver<AgentActorEvent>,
}
```

方法：

- `pause()`
- `resume()`
- `cancel()`
- `wait()`

### 5. `HookRegistry`

定义于 `agents/hooks/public.rs`。

```rust
pub struct HookRegistry {
    hooks: Vec<Box<dyn Hook>>,
}
```

职责：

- 按顺序执行对外扩展 hook
- 默认为空，不隐式注入框架行为
- 作为唯一公开的 hook 注册入口

### 6. `RuntimeHookRegistry`

定义于 `agents/hooks/registry.rs`。

```rust
pub(crate) struct RuntimeHookRegistry {
    hooks: Vec<Box<dyn RuntimeHook>>,
}
```

职责：

- 按顺序执行内部 runtime hook
- 计算 `ExecutionPolicy`
- 暴露内建 hook snapshot
- `default()` 组装框架默认内部行为

### 7. `HookPipeline<'a>`

定义于 `agents/hooks/pipeline.rs`。

```rust
pub(crate) struct HookPipeline<'a> {
    runtime_hooks: &'a RuntimeHookRegistry,
    hooks: &'a HookRegistry,
    scratchpad: StepScratchpad,
    metadata: HashMap<String, Value>,
}
```

职责：

- 把内部 runtime hook 和对外 hook 收口成一条统一执行流水线
- 在支持的阶段内部决定 `Hook` 与 `RuntimeHook` 的执行顺序
- 承载 step 级 scratchpad 和公共 metadata
- 对 `StepLifecycle` 暴露统一的阶段方法，而不是两套分支 API

---

## 四、关键数据结构

### 1. `StepResult`

单步执行结果，定义于 `agent_actor/types.rs`。

```rust
pub enum StepResult {
    Continue { content, reasoning_content, tool_calls, tool_results },
    Done { content, reasoning_content },
    Error(AgentError),
}
```

### 2. `AgentActorEvent`

运行时事件，定义于 `agent_actor/types.rs`。

```rust
pub enum AgentActorEvent {
    ContentChunk(String),
    ReasoningChunk(String),
    StepCompleted { ... },
    ToolCalls(Vec<ToolCall>),
    ToolResult { call_id, success, output },
    Iteration { iteration, message_count },
    HookEvent { hook, kind, payload },
    MaxIterations { iteration },
    Completed,
    Cancelled,
    Error(AgentError),
}
```

新增的 `HookEvent` 用于承载扩展 hook 发出的自定义事件。

### 3. `HookOutcome`

内部 hook 的返回结果，定义于 `agents/hooks/runtime.rs`。

```rust
pub enum HookOutcome {
    Continue,
    Finish(StepResultDraft),
}
```

### 4. `HookPhase`

hook 执行阶段，用于错误定位。

```rust
pub enum HookPhase {
    BeforeStep,
    BeforeCallModel,
    OnModelEvent,
    AfterCallModel,
    BeforeCallTools,
    AfterCallTools,
    AfterStep,
}
```

### 5. 对外 Hook 输入/输出模型

对外 hook 使用稳定快照类型：

- `BeforeStepInput`
- `ModelEventInput`
- `AfterStepInput`

其中：

- `BeforeStepInput` 包含作业标识、用户标识、迭代信息、`job_state`、上下文长度和 metadata
- `ModelEventInput` 包含 `HookModelEvent`
- `AfterStepInput` 包含 `HookStepResult`

对外结果模型也经过了显式收口：

- `HookStepResult`
- `HookStepUpdate`
- `HookContinueStep`
- `HookDoneStep`

工具调用快照统一为：

```rust
pub struct HookToolCall {
    pub id: String,
    pub call_type: String,
    pub index: Option<u32>,
    pub function: HookToolCallFunction,
}
```

这保证了公开 API 只暴露一种稳定、provider-compatible 的结构。

---

## 五、架构图

```text
┌──────────────────────────────────────────────────────────────────────────────┐
│                               AgentActor<C, E>                              │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  ┌─────────────────┐ │
│  │ AgentState  │  │   Chat<C>    │  │ ToolExecutor  │  │ RuntimeHooks    │ │
│  │   (状态)     │  │   (模型)      │  │   (工具)       │  │   内部 hooks    │ │
│  └─────────────┘  └──────────────┘  └───────────────┘  └─────────────────┘ │
│                                              ┌────────────────────────────┐ │
│                                              │       HookRegistry         │ │
│                                              │       扩展 hooks           │ │
│                                              └────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────────────┘
                                     │
                     ┌───────────────┴────────────────┐
                     ▼                                ▼
         ┌──────────────────────┐         ┌──────────────────────────┐
         │      run_loop()      │         │        run_step()        │
         │   (loop_control.rs)  │         │   StepLifecycle 驱动     │
         └──────────────────────┘         └──────────────────────────┘
                                                      │
                                                      ▼
                                  ┌────────────────────────────────────┐
                                  │          StepLifecycle             │
                                  │                                    │
                                  │  hooks: HookPipeline               │
                                  │                                    │
                                  │  before_step()                     │
                                  │  before_call_model()               │
                                  │  on_model_event()                  │
                                  │  after_call_model()                │
                                  │  before_call_tools()               │
                                  │  after_call_tools()                │
                                  │  after_step()                      │
                                  │                                    │
                                  │  public/internal 顺序细节          │
                                  │  由 HookPipeline 内部处理          │
                                  └────────────────────────────────────┘
```

---

## 六、默认内部 Hook 链

默认内部 hook 链通过 `RuntimeHookRegistry::default()` 组装：

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                       RuntimeHookRegistry::default()                   │
├─────────────────────────────────────────────────────────────────────────┤
│  1. MaxIterationsHook      ── 检查迭代上限                             │
│  2. LifecycleHook          ── 状态转换                                  │
│  3. MetricsHook            ── 执行指标统计                              │
│  4. StreamingEventHook     ── 发送 ContentChunk/ReasoningChunk 事件     │
│  5. ContextPersistenceHook ── 持久化消息到 Context                      │
│  6. IterationEventHook     ── 发送 Iteration 事件                       │
│  7. ErrorEventHook         ── 错误事件处理                              │
│  [可选] TimeoutPolicyHook  ── 步骤/工具超时控制                         │
└─────────────────────────────────────────────────────────────────────────┘
```

补充说明：

- `RuntimeHookRegistry::default()` 只包含内部 hook
- `HookRegistry::default()` 为空
- 对外 hook 不是默认内部流水线的一部分，而是独立扩展层

---

## 七、执行流程

### 1. 单步执行 (`run_step`)

`run_step()` 的关键流程如下：

```text
1. 从 RuntimeHookRegistry 计算 ExecutionPolicy
2. 创建 StepLifecycle
3. 用 step timeout 包裹 StepLifecycle::start()
4. start() 负责执行 step 主流程
5. finish() 负责执行 after_step 收尾
```

`start()` 的细化流程：

```text
before_step()
    └── HookPipeline::before_step()
          ├── Hook::before_step()
          └── RuntimeHook::before_step()
        │
        ▼
before_call_model()
    └── HookPipeline::before_call_model()
          └── RuntimeHook::before_call_model()
        │
        ▼
stream_model_output()
    └── 对每个模型事件:
          └── HookPipeline::on_model_event()
                ├── Hook::on_model_event()
                └── RuntimeHook::on_model_event()
        │
        ▼
after_call_model()
    └── HookPipeline::after_call_model()
          └── RuntimeHook::after_call_model()
        │
        ▼
[有工具调用?]
    ├── 否 ───────────────────────────────► 生成 Done
    └── 是
          │
          ▼
        before_call_tools()
          └── HookPipeline::before_call_tools()
                └── RuntimeHook::before_call_tools()
          │
          ▼
        execute_tools()
          │
          ▼
        after_call_tools()
          └── HookPipeline::after_call_tools()
                └── RuntimeHook::after_call_tools()
          │
          ▼
        生成 Continue
```

`finish(result)` 的细化流程：

```text
after_step()
    └── HookPipeline::after_step()
          ├── Hook::after_step()
          └── RuntimeHook::after_step()
```

注意：

- `ReplaceResult` 只能在 `Hook::after_step()` 生效
- 扩展 hook 在当前支持的三个阶段里都先于内部 hook 执行

### 2. 循环执行 (`run_loop`)

```text
spawn(tokio task)
    │
    ▼
主循环:
    1. drain_commands()
    2. 如果已取消，则发送 Cancelled 并退出
    3. wait_if_paused()
    4. run_step()
    5. 根据 StepResult 判断是否继续
       - Continue: 继续下一轮
       - Done: 发送 Completed 并退出
       - Error: 退出
```

---

## 八、控制流

```text
用户代码                   AgentActor                StepLifecycle            Hooks / Model / Tools
   │                           │                           │                             │
   │  AgentActorBuilder        │                           │                             │
   │  .hook(...)               │                           │                             │
   │  .build()                 │                           │                             │
   │ ─────────────────────────►│                           │                             │
   │                           │                           │                             │
   │  run_loop() / run_step()  │                           │                             │
   │ ─────────────────────────►│                           │                             │
   │                           │  创建 StepLifecycle       │                             │
   │                           │ ─────────────────────────►│                             │
   │                           │                           │  扩展 hook                  │
   │                           │                           │  内部 hook                  │
   │                           │                           │  模型流 / 工具执行          │
   │                           │◄──────────────────────────│                             │
   │  AgentActorEvent          │                           │                             │
   │◄──────────────────────────│                           │                             │
```

---

## 九、设计原则

1. **职责分离**
    - `AgentActor` 只负责组合核心组件与暴露 API
    - `StepLifecycle` 负责单步执行编排
    - `loop_control` 负责后台循环与控制命令
    - `builder` 负责配置装配

2. **双层 Hook**
    - 内部 hook 面向运行时内部编排
    - `Hook` 是唯一公开扩展协议
    - 两者解耦，但在同一条 step 生命周期里协同工作

3. **公共 API 稳定优先**
    - `Hook` 不暴露 `&mut AgentState`
    - `Hook` 不暴露 scratchpad 或内部生命周期借用
    - 对外只开放快照输入和受控 effect

4. **流式优先**
    - 模型调用以 stream 方式处理
    - `on_model_event()` 在流式过程中逐条触发
    - 既支持文本 chunk，也支持 reasoning chunk

5. **可控制的扩展能力**
    - `Hook` 可发事件、改 step 结果、终止 step、维护 step metadata
    - 更深层的运行时改写仍保留在内部 hook 层

6. **Builder 模式**
    - 对外只支持链式装配 `Hook`
    - 超时策略仍通过内部 hook 统一注入

---

## 十、使用示例

### 1. 基本用法

```rust
let actor = AgentActorBuilder::new(model, tool_executor)
    .context(context)
    .max_iterations(10)
    .step_timeout(Duration::from_secs(60))
    .build();

let mut handle = actor.run_loop();

while let Some(event) = handle.event_rx.recv().await {
    match event {
        AgentActorEvent::ContentChunk(text) => print!("{}", text),
        AgentActorEvent::Completed => break,
        _ => {}
    }
}
```

### 2. 自定义 Hook

```rust
struct AuditHook;

#[async_trait]
impl Hook for AuditHook {
    fn name(&self) -> &'static str { "AuditHook" }

    async fn before_step(
        &self,
        input: BeforeStepInput,
    ) -> Result<Vec<HookEffect>, HookError> {
        Ok(vec![HookEffect::SetMetadata {
            key: "started_iteration".to_string(),
            value: serde_json::json!(input.iteration),
        }])
    }

    async fn after_step(
        &self,
        input: AfterStepInput,
    ) -> Result<Vec<HookEffect>, HookError> {
        Ok(vec![HookEffect::EmitEvent(HookEvent {
            kind: "audit.step_finished".to_string(),
            payload: serde_json::json!({
                "iteration": input.iteration,
                "result": input.result,
                "metadata": input.metadata,
            }),
        })])
    }
}

let actor = AgentActorBuilder::new(model, tool_executor)
    .hook(AuditHook)
    .build();
```

如果扩展 hook 需要改写最终结果，应在 `after_step()` 返回 `HookEffect::ReplaceResult(...)`。
