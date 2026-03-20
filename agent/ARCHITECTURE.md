# Agent Actor 架构文档

本文档描述 `agent_actor` 当前的执行架构。

当前版本的核心设计目标是：

- hook 不再直接改写 `AgentState` / `Context` / 事件发送器
- 单步执行只保留一个最终事实源：`StepFrame.final_result`
- 所有内核提交逻辑收口到 `commit reducer + committer`

---

## 一、核心设计

当前执行链路分为三层：

1. **Hook 输入层**
   - public hook：稳定快照输入 + `HookEffect`
   - runtime hook：只读 runtime view + `Effect`

2. **Step 运行层**
   - `StepLifecycle` 驱动模型流、工具调用和阶段切换
   - `HookPipeline` 统一执行 public/runtime hook
   - `EffectHandle` 把 effect 应用到 `StepFrame`

3. **提交层**
   - `CommitReducer` 根据 `StepFrame.final_result` 生成 `CommitPlan`
   - `StepCommitter` 统一落地状态、上下文和提交事件

这意味着：

- hook 只能“提议 effect”
- `AgentActor` 内核才负责最终提交
- `after_step` 改写不会再绕开 finalize

---

## 二、模块职责

| 子模块 | 职责 |
|--------|------|
| `agent_actor/mod.rs` | `AgentActor` 定义、共享状态、公共 API |
| `agent_actor/builder.rs` | `AgentActorBuilder` 配置装配 |
| `agent_actor/lifecycle.rs` | 单步执行驱动，维护 `StepFrame` |
| `agent_actor/commit.rs` | `CommitReducer` / `StepCommitter` |
| `agent_actor/loop_control.rs` | `run_step` / `run_loop` 控制逻辑 |
| `agent_actor/types.rs` | 对外事件、错误、句柄、结果类型 |
| `hooks/public.rs` | public hook 协议和稳定快照类型 |
| `hooks/runtime.rs` | runtime hook 协议和只读 runtime view |
| `hooks/effects.rs` | 内部统一 `Effect` / `EffectHandle` |
| `hooks/frame.rs` | step 级暂态容器 `StepFrame` |
| `hooks/pipeline.rs` | hook 执行顺序和 effect 应用 |
| `hooks/registry.rs` | runtime hook 注册、快照、执行策略 |

---

## 三、Hook 设计

### 1. public hook

定义于 `agent/src/agents/hooks/public.rs`。

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

- 输入是稳定快照
- 不暴露 `&mut AgentState`
- 不暴露 scratchpad
- `ReplaceResult` 只允许 `after_step`

### 2. runtime hook

定义于 `agent/src/agents/hooks/runtime.rs`。

```rust
#[async_trait]
pub(crate) trait RuntimeHook: Send + Sync {
    fn name(&self) -> &'static str;
    fn configure_execution_policy(&self, _state: &AgentState, _policy: &mut ExecutionPolicy) {}
    fn snapshot(&self) -> Option<Value> { None }

    async fn before_step(&self, input: BeforeStep<'_>) -> Result<Vec<Effect>, HookError>;
    async fn before_call_model(&self) -> Result<Vec<Effect>, HookError>;
    async fn on_model_event(&self, input: ModelEventCtx<'_>) -> Result<Vec<Effect>, HookError>;
    async fn after_call_model(&self, input: AfterCallModel<'_>) -> Result<Vec<Effect>, HookError>;
    async fn before_call_tools(&self, input: BeforeCallTools<'_>) -> Result<Vec<Effect>, HookError>;
    async fn after_call_tools(&self, input: AfterCallTools<'_>) -> Result<Vec<Effect>, HookError>;
    async fn after_step(&self, input: AfterStep<'_>) -> Result<Vec<Effect>, HookError>;
}
```

特点：

- 输入是只读 runtime view
- runtime view 已收紧为“最小必要视图”：
  - `BeforeStep`：`state`
  - `before_call_model()`：当前不需要额外上下文
  - `ModelEventCtx`：`event`
  - `AfterCallModel`：`output`
  - `BeforeCallTools`：`tool_calls`
  - `AfterCallTools`：`tool_results`
  - `AfterStep`：`frame + result`
- 改写能力也统一收口到 `Effect`
- 不再直接持有 `event_tx` 或 `&mut AgentState`

### 3. 为什么保留双层 hook

因为目标不同：

- public hook 追求协议稳定和边界清晰
- runtime hook 追求内部编排能力和低复制成本

两者现在统一的是“写通道”，不是“输入形态”。

---

## 四、统一 Effect 模型

内部统一 effect 定义于 `agent/src/agents/hooks/effects.rs`：

```rust
pub(crate) enum Effect {
    EmitNow(AgentActorEvent),
    SetMetadata { key: String, value: Value },
    RemoveMetadata { key: String },
    StoreScratchpad { key: &'static str, value: Box<dyn Any + Send + Sync> },
    SetResult(StepResultDraft),
    Abort(AgentError),
}
```

`EffectHandle` 负责把 effect 应用到 `StepFrame`：

- `EmitNow`：立即发送事件
- `SetMetadata` / `RemoveMetadata`：修改 step 级 metadata
- `StoreScratchpad`：写 step 级 scratchpad
- `SetResult` / `Abort`：改写 `StepFrame.final_result`

这就是统一的写入口。

---

## 五、StepFrame

`StepFrame` 定义于 `agent/src/agents/hooks/frame.rs`：

```rust
pub(crate) struct StepFrame {
    pub metadata: HashMap<String, Value>,
    pub scratchpad: StepScratchpad,
    pub model_output: ModelCallOutput,
    pub tool_calls: Vec<ToolCall>,
    pub tool_results: Vec<CallToolResult>,
    pub final_result: Option<StepResultDraft>,
}
```

职责：

- 承载单步执行的全部暂态
- 作为 hook effect 的统一作用目标
- 保存唯一最终结果 `final_result`

这让 `run_step()` 在任何时刻都只有一个“最终结果槽位”，不会再出现多个提交源。

---

## 六、提交层

提交逻辑定义于 `agent/src/agents/agent_actor/commit.rs`。

### 1. `CommitReducer`

纯规则层：

- 根据 `StepFrame.final_result` 计算 `JobState`
- 生成要持久化的 context messages
- 生成提交事件序列

### 2. `StepCommitter`

副作用层：

- 写入 `state.state`
- 持久化 `Context`
- 顺序发送提交事件

内核职责已经从 runtime hook 中收回：

- 状态迁移
- context 持久化
- iteration / error / max-iterations / cancelled 提交事件

这些逻辑不再依赖 hook 顺序偶然成立。

---

## 七、关键结构

### 1. `AgentActor<C, E>`

```rust
pub struct AgentActor<C, E> {
    state: AgentState,
    chat: Arc<C>,
    tool_executor: Arc<E>,
    runtime_hooks: RuntimeHookRegistry,
    hooks: HookRegistry,
}
```

职责：

- 持有模型、工具执行器、状态和两类 hook registry
- 暴露 `run_step()` / `run_loop()`
- 只对外公开 public hook 注册入口

### 2. `StepLifecycle<'a>`

```rust
struct StepLifecycle<'a> {
    state: &'a mut AgentState,
    event_tx: Option<&'a mpsc::Sender<AgentActorEvent>>,
    hooks: HookPipeline<'a>,
    execution_policy: ExecutionPolicy,
    frame: StepFrame,
}
```

职责：

- 执行单步主流程
- 维护 `StepFrame`
- 在 `finish()` 中进入统一提交阶段

### 3. `HookPipeline<'a>`

```rust
pub(crate) struct HookPipeline<'a> {
    runtime_hooks: &'a RuntimeHookRegistry,
    hooks: &'a HookRegistry,
}
```

职责：

- 统一执行 public/runtime hook
- 把 public `HookEffect` 转换成内部 `Effect`
- 调用 `EffectHandle` 修改 `StepFrame`

### 4. `CommitPlan`

```rust
pub(super) struct CommitPlan {
    next_state: JobState,
    context_messages: Vec<Message>,
    events: Vec<AgentActorEvent>,
}
```

职责：

- 表达“这一步最终要怎么提交”
- 把纯规则和副作用执行拆开

---

## 八、事件语义

当前事件分成两类：

### 1. 轨迹事件

- `ContentChunk`
- `ReasoningChunk`
- `StepCompleted`
- `ToolCalls`
- `ToolResult`
- `HookEvent`

这些事件反映执行轨迹，不表示最终提交结果。

其中 `StepCompleted` 语义是：

- “模型输出已经收齐”
- 不是“这一步已经最终提交”

### 2. 提交事件

- `StepFinalized { result }`
- `Iteration`
- `Error`
- `MaxIterations`
- `Completed`
- `Cancelled`

`StepFinalized` 是单步真正的提交边界，表示：

- `StepResult`
- `Context`
- `JobState`

三者已经对齐到同一个最终版本。

`Completed` 则只在 `run_loop()` 里出现，表示 actor 级循环已经在某个 `Done` step 提交之后结束。

---

## 九、默认 runtime hook 链

`RuntimeHookRegistry::default()` 现在只保留真正的 runtime 扩展能力：

```text
1. MetricsHook
2. StreamingEventHook
```

补充：

- `TimeoutPolicyHook` 仍通过 builder 按需注入
- `max_iterations` 已收回到 `StepLifecycle` preflight guard
- lifecycle / context persistence / iteration events / error events 不再属于默认 runtime hook 链

---

## 十、执行流程

### 1. `run_step()`

```text
1. 计算 ExecutionPolicy
2. 创建 StepLifecycle { frame: StepFrame::default() }
3. 用 step timeout 包裹 lifecycle.start()
4. start() 只负责把结果写入 frame.final_result
5. finish() 执行 after_step hook，并统一提交
```

### 2. `StepLifecycle::start()`

```text
preflight:
    - max_iterations guard
    - state.iteration += 1
    - state.state = Running

before_step
before_call_model
stream_model_output
after_call_model
[有工具调用?]
    否 -> frame.final_result = Done
    是 -> before_call_tools -> execute_tools -> after_call_tools
         -> frame.final_result = Continue
```

### 3. `StepLifecycle::finish()`

```text
1. HookPipeline::after_step()
   - public after_step 先运行
   - runtime after_step 后运行
   - 两者都只能通过 effect 改 final_result

2. CommitReducer::reduce()

3. StepCommitter::apply()
   - state
   - context
   - StepFinalized / Iteration / Error / MaxIterations / Cancelled

4. 如果 `run_loop()` 收到 `Done`
   - 在 step 已经提交完成之后额外发送 actor 级 `Completed`
```

---

## 十一、设计原则

1. **单一提交点**
   - `StepFrame.final_result` 是唯一最终事实源

2. **统一写通道**
   - 所有 hook 改写都通过 effect 进入内核

3. **kernel 负责提交**
   - hook 只提议
   - 内核才落地

4. **轨迹与提交分离**
   - 执行轨迹事件不等于最终提交结果
   - `StepFinalized` 才是最终提交边界

5. **双层 hook，单条 effect 脊柱**
   - public/runtime 输入模型不同
   - 写入模型统一

6. **时序一致性优先**
   - `Abort` 不再绕开 internal finalize
   - context/state/final result 保持一致

---

## 十二、当前收益

这次重构解决了旧架构里最核心的几个问题：

- public `after_step` 改写结果后，内核提交能看到同一个最终结果
- `Abort` 不再跳过 runtime `after_step`
- 状态迁移、context 持久化和提交事件不再依赖 hook 链顺序
- `StepCompleted` 与 `StepFinalized` 语义分离，事件边界更清晰

这也是当前 `agent_actor` 的主设计方向：**hook 产出 effect，kernel 统一提交**。
