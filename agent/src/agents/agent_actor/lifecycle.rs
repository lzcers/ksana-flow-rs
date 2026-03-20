use std::pin::{Pin, pin};
use std::time::Duration;

use futures::{Stream, StreamExt};
use tokio::sync::mpsc;

use super::commit::{CommitReducer, StepCommitter};
use super::types::step_result_from_draft;
use super::{AgentActorEvent, AgentError, StepResult};
use crate::agents::call_model::{CallModelEvent, CallToolResult, call_model, call_tools};
use crate::agents::hooks::{
    ExecutionPolicy, HookPipeline, HookRegistry, RuntimeHookRegistry, StepFrame, StepResultDraft,
};
use crate::agents::{AgentState, ToolExecutor};
use crate::models::ChatCapability;

pub(super) struct StepLifecycle<'a> {
    state: &'a mut AgentState,
    event_tx: Option<&'a mpsc::Sender<AgentActorEvent>>,
    hooks: HookPipeline<'a>,
    execution_policy: ExecutionPolicy,
    frame: StepFrame,
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
            frame: StepFrame::default(),
        }
    }

    pub(super) fn step_timeout(&self) -> Option<Duration> {
        self.execution_policy.step_timeout
    }

    fn tool_timeout(&self) -> Option<Duration> {
        self.execution_policy.tool_timeout
    }

    pub(super) fn set_result(&mut self, result: StepResultDraft) {
        self.frame.set_result(result);
    }

    fn has_result(&self) -> bool {
        self.frame.result().is_some()
    }

    fn begin_step(&mut self) -> bool {
        if self.state.iteration >= self.state.max_iterations {
            self.set_result(StepResultDraft::Error(AgentError::MaxIterations(
                self.state.iteration,
            )));
            return true;
        }

        self.state.iteration += 1;
        self.state.state = crate::agents::JobState::Running;
        false
    }

    async fn before_step(&mut self) -> bool {
        match self
            .hooks
            .before_step(&*self.state, &mut self.frame, self.event_tx)
            .await
        {
            Ok(()) => self.has_result(),
            Err(err) => {
                self.set_result(StepResultDraft::Error(err));
                true
            }
        }
    }

    async fn before_call_model(&mut self) -> bool {
        match self
            .hooks
            .before_call_model(&mut self.frame, self.event_tx)
            .await
        {
            Ok(()) => self.has_result(),
            Err(err) => {
                self.set_result(StepResultDraft::Error(err));
                true
            }
        }
    }

    async fn after_call_model(&mut self) -> bool {
        match self
            .hooks
            .after_call_model(&mut self.frame, self.event_tx)
            .await
        {
            Ok(()) => self.has_result(),
            Err(err) => {
                self.set_result(StepResultDraft::Error(err));
                true
            }
        }
    }

    async fn before_call_tools(&mut self) -> bool {
        match self
            .hooks
            .before_call_tools(&mut self.frame, self.event_tx)
            .await
        {
            Ok(()) => self.has_result(),
            Err(err) => {
                self.set_result(StepResultDraft::Error(err));
                true
            }
        }
    }

    async fn after_call_tools(&mut self) -> bool {
        match self
            .hooks
            .after_call_tools(&mut self.frame, self.event_tx)
            .await
        {
            Ok(()) => self.has_result(),
            Err(err) => {
                self.set_result(StepResultDraft::Error(err));
                true
            }
        }
    }

    async fn stream_model_output(
        &mut self,
        mut stream: Pin<&mut impl Stream<Item = CallModelEvent>>,
    ) -> bool {
        let mut model_error: Option<AgentError> = None;

        while let Some(event) = stream.next().await {
            match &event {
                CallModelEvent::TextChunk(text) => {
                    self.frame.model_output.content.push_str(text);
                }
                CallModelEvent::ReasoningChunk(text) => {
                    self.frame
                        .model_output
                        .reasoning_content
                        .get_or_insert_with(String::new)
                        .push_str(text);
                }
                CallModelEvent::Completed {
                    content,
                    reasoning_content,
                    tool_calls,
                } => {
                    self.frame.model_output.content = content.clone();
                    self.frame.model_output.reasoning_content = reasoning_content.clone();
                    self.frame.model_output.tool_calls = tool_calls.clone().unwrap_or_default();
                }
                CallModelEvent::Error(message) => {
                    model_error = Some(AgentError::Model(message.clone()));
                }
            }

            match self
                .hooks
                .on_model_event(&*self.state, &mut self.frame, self.event_tx, &event)
                .await
            {
                Ok(()) => {}
                Err(err) => {
                    self.set_result(StepResultDraft::Error(err));
                    return true;
                }
            }

            if let Some(err) = model_error.take() {
                self.set_result(StepResultDraft::Error(err));
                return true;
            }

            if self.has_result() {
                return true;
            }
        }

        false
    }

    pub(super) async fn start(
        &mut self,
        model: &(dyn ChatCapability + Sync),
        tool_executor: &dyn ToolExecutor,
    ) {
        let messages = self.state.context.to_messages();
        let tools = tool_executor.tools().clone();

        if self.begin_step() || self.before_step().await || self.before_call_model().await {
            return;
        }

        let stream = pin!(call_model(model, &messages, Some(&tools)));

        if self.stream_model_output(stream).await || self.after_call_model().await {
            return;
        }

        if self.frame.model_output.tool_calls.is_empty() {
            self.set_result(StepResultDraft::Done {
                content: self.frame.model_output.content.clone(),
                reasoning_content: self.frame.model_output.reasoning_content.clone(),
            });
            return;
        }

        self.frame.tool_calls = self.frame.model_output.tool_calls.clone();

        if self.before_call_tools().await {
            return;
        }

        if self.frame.tool_calls.is_empty() {
            self.set_result(StepResultDraft::Done {
                content: self.frame.model_output.content.clone(),
                reasoning_content: self.frame.model_output.reasoning_content.clone(),
            });
            return;
        }

        let tool_results = match execute_tools_with_timeout(
            tool_executor,
            &self.frame.tool_calls,
            self.tool_timeout(),
        )
        .await
        {
            Ok(results) => results,
            Err(err) => {
                self.set_result(StepResultDraft::Error(err));
                return;
            }
        };
        self.frame.tool_results = tool_results;

        if self.after_call_tools().await {
            return;
        }

        self.set_result(StepResultDraft::Continue {
            content: self.frame.model_output.content.clone(),
            reasoning_content: self.frame.model_output.reasoning_content.clone(),
            tool_calls: self.frame.tool_calls.clone(),
            tool_results: self.frame.tool_results.clone(),
        });
    }

    pub(super) async fn finish(&mut self) -> StepResult {
        if self.frame.result().is_none() {
            self.set_result(StepResultDraft::Error(AgentError::Model(
                "step finished without a final result".to_string(),
            )));
        }

        self.hooks
            .after_step(&*self.state, &mut self.frame, self.event_tx)
            .await;

        let final_result = self.frame.result().cloned().unwrap_or_else(|| {
            StepResultDraft::Error(AgentError::Model(
                "step finalizer removed the final result".to_string(),
            ))
        });
        let plan = CommitReducer::reduce(&*self.state, &final_result);
        StepCommitter::apply(plan, self.state, self.event_tx).await;

        step_result_from_draft(final_result)
    }
}

async fn execute_tools_with_timeout(
    tool_executor: &dyn ToolExecutor,
    tool_calls: &[crate::agents::ToolCall],
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
