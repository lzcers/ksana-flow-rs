# Agent Actor Refactor TODO

当前 `agent_actor` 已经完成主干重构：

- 执行主链已切到 `StepFrame + EffectHandle + CommitReducer/StepCommitter`
- `RuntimeHook` 已改成只读输入 + `Vec<Effect>`
- `StepFinalized` 已作为单步提交边界事件接入
- 关键测试已通过：
  - `cargo test -p agent test_public_`
  - `cargo test -p agent test_agent_actor_hooks_`

但这还不是“最终收尾版”。下次启动后，按下面清单继续完成。

---

## 1. 删除已经退役的旧内核型 hook 文件

这些文件的职责已经被 `StepLifecycle` / `CommitReducer` / `StepCommitter` 回收，不应继续保留：

- `agent/src/agents/hooks/context_persistence.rs`
- `agent/src/agents/hooks/error_events.rs`
- `agent/src/agents/hooks/iteration_events.rs`
- `agent/src/agents/hooks/lifecycle.rs`
- `agent/src/agents/hooks/max_iterations.rs`

同时清理对应引用：

- `agent/src/agents/hooks/mod.rs`
- `agent/src/agents/hooks/registry.rs`
- `agent/src/agents/mod.rs`
- `agent/src/agents/tests.rs`

验收：

- `rg -n "ContextPersistenceHook|ErrorEventHook|IterationEventHook|LifecycleHook|MaxIterationsHook" agent/src`
  只剩下历史注释或文档，不再有真实代码依赖

---

## 2. 处理 `EmitOnCommit`

当前 `Effect::EmitOnCommit` 已定义，但没有 runtime/public hook 真正使用。

二选一：

1. 保留并接入实际用法
   - 让某些“提交后事件”明确走 `EmitOnCommit`
   - 候选：`StepFinalized` 之外的提交型扩展事件

2. 如果短期没有使用场景，直接删掉
   - 删除 `agent/src/agents/hooks/effects.rs` 中该变体
   - 清理 `CommitReducer` / `StepFrame.pending_events` 里不必要的保留设计

建议：

- 如果没有明确即将落地的 commit-stage 扩展事件，先删掉，避免半成品接口长期悬空

---

## 3. 收紧 runtime view 接口

当前 `runtime.rs` 里有多处 `#[allow(dead_code)]`，说明接口形状已经就位，但还没完全打磨完：

- `agent/src/agents/hooks/runtime.rs`

需要做的事：

- 逐个检查这些字段是否真的需要保留
- 删除不必要字段
- 删除对应的 `#[allow(dead_code)]`
- 如果字段保留是为了未来扩展，在文档中写清楚原因

建议优先检查：

- `BeforeStep.frame`
- `BeforeCallModel.state/frame/tools`
- `ModelEventCtx.state/frame`
- `AfterCallModel.state/frame`
- `BeforeCallTools.state/frame`
- `AfterCallTools.state/frame`
- `AfterStep.state`

目标：

- 让 runtime hook 输入既是只读的，也尽量是“最小必要视图”

---

## 4. 再校准事件语义

现在已经有：

- `StepCompleted`：模型输出收齐
- `StepFinalized`：单步最终提交完成

还要确认一遍对外语义是否最终定稿：

- `StepCompleted` 这个名字是否仍然足够准确
- 是否要改名为更显式的 `ModelOutputReady`
- `run_loop` 里的 `Completed` 是否只保留为 actor 级终态事件

涉及文件：

- `agent/src/agents/agent_actor/types.rs`
- `agent/ARCHITECTURE.md`
- `agent/src/agents/tests.rs`

如果决定改名：

- 必须同步更新测试和文档

---

## 5. 清理测试里的过渡性适配

本次为保证主链先稳定，测试做了最小修正，但仍需再整理一轮：

- `agent/src/agents/tests.rs`

需要做的事：

- 检查新增的 `StepFinalized` 断言是否足够
- 补一组更直接的测试，验证这三个面严格一致：
  - `run_step()` 返回值
  - `Context` 最终持久化内容
  - `StepFinalized.result`
- 补一组 `Abort` 测试，明确 runtime `after_step` 仍会执行
- 补一组 max-iterations 测试，明确它现在走的是 lifecycle preflight 而不是 runtime hook

建议新增测试名：

- `test_step_finalized_matches_return_value_and_context`
- `test_abort_still_runs_runtime_after_step_finalizers`
- `test_max_iterations_short_circuits_before_hooks`

---

## 6. 检查 `run_loop` 终态事件是否需要补充一致性

当前单步提交已经统一，但还需要再确认 actor 级终态事件和 step 级提交事件是否顺序合理：

- `StepFinalized`
- `Completed`
- `Cancelled`
- `Error`
- `MaxIterations`

重点检查：

- `run_loop()` 在 `Done` 后发送 `Completed`
- `run_loop()` 在 error/max-iterations/cancel 情况下是否存在重复或歧义事件

涉及文件：

- `agent/src/agents/agent_actor/loop_control.rs`
- `agent/src/agents/agent_actor/commit.rs`
- `agent/src/agents/tests.rs`

目标：

- 明确“step 级提交事件”和“actor 级终态事件”的层级关系

---

## 7. 最终清理文档

在代码收尾完成后，再做最后一轮文档同步：

- `agent/ARCHITECTURE.md`
- `agent/TODO.md`

需要做的事：

- 删除 TODO 已完成项
- 如果 `EmitOnCommit` 被删除，文档也同步删掉
- 如果事件名调整，文档全量同步
- 确认默认 runtime hook 链描述和代码一致

---

## 8. 最终验证

至少执行：

```bash
cargo fmt --package agent
cargo check -p agent --lib
cargo test -p agent test_public_
cargo test -p agent test_agent_actor_hooks_
```

如果收尾涉及公开接口或事件语义，建议再跑：

```bash
cargo test -p agent
```

注意：

- 当前完整 `cargo test -p agent` 里已知还有 3 个 OpenRouter 相关失败
- 这些失败与本次 `agent_actor` 主干重构无关
- 下次执行时要先确认它们是否仍然是外部依赖/环境问题

---

## 建议执行顺序

1. 删除退役旧 hook 文件和引用
2. 决定 `EmitOnCommit` 是接着做还是删掉
3. 收紧 runtime view 接口并去掉 `allow(dead_code)`
4. 补测试，尤其是 `StepFinalized` 一致性测试
5. 检查 `run_loop` 终态事件顺序
6. 更新文档并做最终验证
