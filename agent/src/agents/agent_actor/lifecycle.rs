use std::ops::ControlFlow;
use std::pin::{Pin, pin};
use std::time::Duration;

use futures::{Stream, StreamExt};
use tokio::sync::mpsc;

use super::types::step_result_from_draft;
use super::{AgentActorEvent, AgentError, StepResult};
use crate::agents::call_model::{CallModelEvent, CallToolResult, call_model, call_tools};
use crate::agents::hooks::{
    ExecutionPolicy, HookPipeline, HookRegistry, ModelCallOutput, RuntimeHookRegistry,
    StepResultDraft,
};
use crate::agents::{AgentState, ToolCall, ToolDef, ToolExecutor};
use crate::models::ChatCapability;

pub(super) type StepControl<T = ()> = ControlFlow<StepResultDraft, T>;

pub(super) struct StepLifecycle<'a> {
    state: &'a mut AgentState,
    event_tx: Option<&'a mpsc::Sender<AgentActorEvent>>,
    hooks: HookPipeline<'a>,
    execution_policy: ExecutionPolicy,
}

impl<'a> StepLifecycle<'a> {
    pub(super) fn new(
        state: &'a mut AgentState,
        runtime_hooks: &'a RuntimeHookRegistry,
        hooks: &'a HookRegistry,
        event_tx: Option<&'a mpsc::Sender<AgentActorEvent>>,
        execution_policy: ExecutionPolicy,
    ) -> Self {
        Self {
            state,
            event_tx,
            hooks: HookPipeline::new(runtime_hooks, hooks),
            execution_policy,
        }
    }

    pub(super) fn step_timeout(&self) -> Option<Duration> {
        self.execution_policy.step_timeout
    }

    fn tool_timeout(&self) -> Option<Duration> {
        self.execution_policy.tool_timeout
    }

    async fn before_step(&mut self) -> StepControl {
        Self::phase_control(
            self.hooks
                .before_step(&mut *self.state, self.event_tx)
                .await,
        )
    }

    async fn before_call_model(&mut self, tools: &[ToolDef]) -> StepControl {
        Self::phase_control(
            self.hooks
                .before_call_model(&mut *self.state, self.event_tx, tools)
                .await,
        )
    }

    async fn on_model_event(&mut self, event: &CallModelEvent) -> StepControl {
        Self::phase_control(
            self.hooks
                .on_model_event(&mut *self.state, self.event_tx, event)
                .await,
        )
    }

    async fn after_call_model(&mut self, output: &mut ModelCallOutput) -> StepControl {
        Self::phase_control(
            self.hooks
                .after_call_model(&mut *self.state, self.event_tx, output)
                .await,
        )
    }

    async fn before_call_tools(&mut self, tool_calls: &mut Vec<ToolCall>) -> StepControl {
        Self::phase_control(
            self.hooks
                .before_call_tools(&mut *self.state, self.event_tx, tool_calls)
                .await,
        )
    }

    async fn after_call_tools(
        &mut self,
        tool_calls: &[ToolCall],
        tool_results: &mut Vec<CallToolResult>,
    ) -> StepControl {
        Self::phase_control(
            self.hooks
                .after_call_tools(&mut *self.state, self.event_tx, tool_calls, tool_results)
                .await,
        )
    }

    async fn stream_model_output(
        &mut self,
        mut stream: Pin<&mut impl Stream<Item = CallModelEvent>>,
    ) -> StepControl<ModelCallOutput> {
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

    fn phase_control(result: Result<Option<StepResultDraft>, AgentError>) -> StepControl {
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

    pub(super) fn resolve_control<T>(
        result: StepControl<T>,
        on_continue: impl FnOnce(T) -> StepResultDraft,
    ) -> StepResultDraft {
        match result {
            ControlFlow::Break(result) => result,
            ControlFlow::Continue(value) => on_continue(value),
        }
    }

    pub(super) async fn start(
        &mut self,
        model: &(dyn ChatCapability + Sync),
        tool_executor: &dyn ToolExecutor,
    ) -> StepControl<StepResultDraft> {
        let tools = tool_executor.tools();
        let messages = self.state.context.to_messages();

        self.before_step().await?;
        self.before_call_model(tools).await?;

        // 调用模型
        let stream = pin!(call_model(model, &messages, Some(tools)));
        let mut model_output = self.stream_model_output(stream).await?;

        self.after_call_model(&mut model_output).await?;

        if model_output.tool_calls.is_empty() {
            return ControlFlow::Continue(StepResultDraft::Done {
                content: model_output.content,
                reasoning_content: model_output.reasoning_content,
            });
        }

        let mut tool_calls = model_output.tool_calls;
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

    pub(super) async fn finish(&mut self, mut result: StepResultDraft) -> StepResult {
        match self
            .hooks
            .after_step(&mut *self.state, self.event_tx, &mut result)
            .await
        {
            Ok(()) => {}
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
