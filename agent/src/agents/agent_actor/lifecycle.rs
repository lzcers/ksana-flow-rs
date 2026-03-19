use std::ops::ControlFlow;
use std::pin::pin;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;

use super::types::step_result_from_draft;
use super::{AgentActorEvent, AgentError, StepResult};
use crate::agents::call_model::{call_model, call_tools, CallModelEvent, CallToolResult};
use crate::agents::hooks::{
    AfterCallModel, AfterCallTools, AfterStep, BeforeCallModel, BeforeCallTools, ExecutionPolicy,
    HookRegistry, ModelCallOutput, ModelEventCtx, StepHookContext, StepResultDraft, StepScratchpad,
};
use crate::agents::{AgentState, ToolCall, ToolDef, ToolExecutor};
use crate::models::ChatCapability;

pub(super) type StepControl<T = ()> = ControlFlow<StepResultDraft, T>;
type PhaseControl = StepControl<()>;

pub(super) struct StepLifecycle<'a> {
    state: &'a mut AgentState,
    hooks: &'a HookRegistry,
    event_tx: Option<&'a mpsc::Sender<AgentActorEvent>>,
    scratchpad: StepScratchpad,
    execution_policy: ExecutionPolicy,
}

impl<'a> StepLifecycle<'a> {
    pub(super) fn new(
        state: &'a mut AgentState,
        hooks: &'a HookRegistry,
        event_tx: Option<&'a mpsc::Sender<AgentActorEvent>>,
        execution_policy: ExecutionPolicy,
    ) -> Self {
        Self {
            state,
            hooks,
            event_tx,
            scratchpad: StepScratchpad::default(),
            execution_policy,
        }
    }

    pub(super) fn step_timeout(&self) -> Option<Duration> {
        self.execution_policy.step_timeout
    }

    fn tool_timeout(&self) -> Option<Duration> {
        self.execution_policy.tool_timeout
    }

    fn make_ctx<'b>(
        state: &'b mut AgentState,
        event_tx: Option<&'b mpsc::Sender<AgentActorEvent>>,
        scratchpad: &'b mut StepScratchpad,
    ) -> StepHookContext<'b> {
        StepHookContext {
            state,
            event_tx,
            scratchpad,
        }
    }

    async fn before_step(&mut self) -> PhaseControl {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            hooks.before_step(&mut ctx).await
        };
        Self::phase_control(result)
    }

    async fn before_call_model(&mut self, tools: &[ToolDef]) -> PhaseControl {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            let mut before_call_model = BeforeCallModel { tools };
            hooks
                .before_call_model(&mut ctx, &mut before_call_model)
                .await
        };
        Self::phase_control(result)
    }

    async fn on_model_event(&mut self, event: &CallModelEvent) -> PhaseControl {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            let model_event = ModelEventCtx { event };
            hooks.on_model_event(&mut ctx, &model_event).await
        };
        Self::phase_control(result)
    }

    async fn after_call_model(&mut self, output: &mut ModelCallOutput) -> PhaseControl {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            let mut after_call_model = AfterCallModel { output };
            hooks
                .after_call_model(&mut ctx, &mut after_call_model)
                .await
        };
        Self::phase_control(result)
    }

    async fn before_call_tools(&mut self, tool_calls: &mut Vec<ToolCall>) -> PhaseControl {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            let mut before_call_tools = BeforeCallTools { tool_calls };
            hooks
                .before_call_tools(&mut ctx, &mut before_call_tools)
                .await
        };
        Self::phase_control(result)
    }

    async fn after_call_tools(
        &mut self,
        tool_calls: &[ToolCall],
        tool_results: &mut Vec<CallToolResult>,
    ) -> PhaseControl {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            let mut after_call_tools = AfterCallTools {
                tool_calls,
                tool_results,
            };
            hooks
                .after_call_tools(&mut ctx, &mut after_call_tools)
                .await
        };
        Self::phase_control(result)
    }

    pub(super) async fn execute_core(
        &mut self,
        model: &(dyn ChatCapability + Sync),
        tool_executor: &dyn ToolExecutor,
    ) -> StepControl<StepResultDraft> {
        let tools = tool_executor.tools().clone();

        self.before_step().await?;
        self.before_call_model(&tools).await?;

        let mut model_output = self.stream_model_output(model, &tools).await?;

        self.after_call_model(&mut model_output).await?;

        if model_output.tool_calls.is_empty() {
            return ControlFlow::Continue(StepResultDraft::Done {
                content: model_output.content,
                reasoning_content: model_output.reasoning_content,
            });
        }

        let mut tool_calls = model_output.tool_calls.clone();
        self.before_call_tools(&mut tool_calls).await?;

        if tool_calls.is_empty() {
            return ControlFlow::Continue(StepResultDraft::Done {
                content: model_output.content,
                reasoning_content: model_output.reasoning_content,
            });
        }

        let mut tool_results = Self::agent_error_control(
            execute_tools_with_timeout(tool_executor, &tool_calls, self.tool_timeout()).await,
        )?;

        self.after_call_tools(&tool_calls, &mut tool_results)
            .await?;

        ControlFlow::Continue(StepResultDraft::Continue {
            content: model_output.content,
            reasoning_content: model_output.reasoning_content,
            tool_calls,
            tool_results,
        })
    }

    async fn stream_model_output(
        &mut self,
        model: &(dyn ChatCapability + Sync),
        tools: &[ToolDef],
    ) -> StepControl<ModelCallOutput> {
        let messages = self.state.context.to_messages();
        let tools = tools.to_vec();
        let mut stream = pin!(call_model(model, &messages, Some(&tools)));
        let mut model_output = ModelCallOutput::default();
        let mut model_error: Option<AgentError> = None;

        while let Some(event) = stream.next().await {
            match &event {
                CallModelEvent::TextChunk(text) => {
                    model_output.content.push_str(text);
                }
                CallModelEvent::ReasoningChunk(text) => {
                    model_output
                        .reasoning_content
                        .get_or_insert_with(String::new)
                        .push_str(text);
                }
                CallModelEvent::Completed {
                    content,
                    reasoning_content,
                    tool_calls,
                } => {
                    model_output.content = content.clone();
                    model_output.reasoning_content = reasoning_content.clone();
                    model_output.tool_calls = tool_calls.clone().unwrap_or_default();
                }
                CallModelEvent::Error(message) => {
                    model_error = Some(AgentError::Model(message.clone()));
                }
            }

            self.on_model_event(&event).await?;

            if let Some(err) = model_error.take() {
                return ControlFlow::Break(StepResultDraft::Error(err));
            }
        }

        ControlFlow::Continue(model_output)
    }

    pub(super) async fn finish(&mut self, mut result: StepResultDraft) -> StepResult {
        let after_step_result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            let mut after_step = AfterStep {
                result: &mut result,
            };
            hooks.after_step(&mut ctx, &mut after_step).await
        };

        match after_step_result {
            Ok(Some(next_result)) => result = next_result,
            Ok(None) => {}
            Err(err) => {
                self.state.state = crate::agents::JobState::Failed;
                if let Some(event_tx) = self.event_tx {
                    let _ = event_tx.send(AgentActorEvent::Error(err.clone())).await;
                }
                return StepResult::Error(err);
            }
        }

        step_result_from_draft(result)
    }

    fn phase_control(result: Result<Option<StepResultDraft>, AgentError>) -> PhaseControl {
        match result {
            Ok(Some(result)) => ControlFlow::Break(result),
            Ok(None) => ControlFlow::Continue(()),
            Err(err) => ControlFlow::Break(StepResultDraft::Error(err)),
        }
    }

    fn agent_error_control<T>(result: Result<T, AgentError>) -> StepControl<T> {
        match result {
            Ok(value) => ControlFlow::Continue(value),
            Err(err) => ControlFlow::Break(StepResultDraft::Error(err)),
        }
    }

    pub(super) fn settle_control<T>(
        result: StepControl<T>,
        on_continue: impl FnOnce(T) -> StepResultDraft,
    ) -> StepResultDraft {
        match result {
            ControlFlow::Break(result) => result,
            ControlFlow::Continue(value) => on_continue(value),
        }
    }
}

async fn execute_tools_with_timeout(
    tool_executor: &dyn ToolExecutor,
    tool_calls: &[ToolCall],
    timeout: Option<Duration>,
) -> Result<Vec<CallToolResult>, AgentError> {
    let execute = async {
        let mut stream = pin!(call_tools(tool_executor, tool_calls));
        let mut results = Vec::new();
        while let Some(result) = stream.next().await {
            results.push(result);
        }
        results
    };

    if let Some(dur) = timeout {
        match tokio::time::timeout(dur, execute).await {
            Ok(results) => Ok(results),
            Err(_) => Err(AgentError::Timeout),
        }
    } else {
        Ok(execute.await)
    }
}
