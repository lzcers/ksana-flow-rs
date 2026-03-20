use std::ops::ControlFlow;
use std::pin::{Pin, pin};
use std::time::Duration;

use futures::{Stream, StreamExt};

use super::{AgentActorEvent, AgentError, StepResult};
use crate::agents::call_model::{CallModelEvent, CallToolResult, call_model, call_tools};
use crate::agents::{AgentState, ToolCall, ToolExecutor};
use crate::models::ChatCapability;

pub(super) enum LifeCycle {
    BeforeStep,
    BeforeCallModel,
    OnModelEvent,
    AfterCallModel,
    BeforeCallTools,
    AfterCallTools,
    AfterStep,
}

pub(super) type LifecycleResult = ();
pub(super) enum LifecycleError {
    // (LifecyclePhase, HookName, ErrorMessage)
    HookError(LifeCycle, String, String),
}

pub(super) type LifecycleFlow = ControlFlow<LifecycleError, LifecycleResult>;

pub(super) struct StepFrame {
    pub model_output: Option<StepResult>,
    pub tools_result: Option<Vec<CallToolResult>>,
    pub tools_call: Option<Vec<ToolCall>>,
}

impl Default for StepFrame {
    fn default() -> Self {
        Self {
            model_output: None,
            tools_result: None,
            tools_call: None,
        }
    }
}

pub(super) struct StepLifecycle {
    state: AgentState,
    frame: StepFrame,
}

// 供外部调用
impl StepLifecycle {
    pub(super) fn new(state: AgentState) -> Self {
        Self {
            state,
            frame: Default::default(),
        }
    }

    pub(super) async fn start(
        &mut self,
        model: &(dyn ChatCapability + Sync),
        tool_executor: &dyn ToolExecutor,
    ) -> LifecycleFlow {
        let messages = self.state.context.to_messages();
        let tools = tool_executor.tools().clone();

        self.call_life_cyle_hook(LifeCycle::BeforeStep).await?;

        self.call_life_cyle_hook(LifeCycle::BeforeCallModel).await?;

        let mut stream = pin!(call_model(model, &messages, Some(&tools)));
        while let Some(event) = stream.next().await {
            match event {
                CallModelEvent::TextChunk(chunk) => {}
                CallModelEvent::ReasoningChunk(tools_call) => {}
                CallModelEvent::Completed {
                    content,
                    reasoning_content,
                    tool_calls,
                } => {}
                CallModelEvent::Error(e) => {}
                _ => {}
            }
            self.call_life_cyle_hook(LifeCycle::OnModelEvent).await?;
        }

        if let Some(tools_call) = self.frame.tools_call.as_ref().cloned() {
            self.call_life_cyle_hook(LifeCycle::BeforeCallTools).await?;

            if let Ok(results) = Self::execute_tools_with_timeout(
                tool_executor,
                &tools_call,
                Some(Duration::from_secs(120)),
            )
            .await
            {
                self.frame.tools_result = Some(results);
            } else {
                todo!();
            }

            self.call_life_cyle_hook(LifeCycle::AfterCallTools).await?;
        };

        self.call_life_cyle_hook(LifeCycle::AfterStep).await?;
        Self::continue_step()
    }

    fn break_step(err: LifecycleError) -> LifecycleFlow {
        ControlFlow::Break(err)
    }

    fn continue_step() -> LifecycleFlow {
        ControlFlow::Continue(())
    }

    async fn call_life_cyle_hook(&mut self, lifecycle: LifeCycle) -> LifecycleFlow {
        Self::continue_step()
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
}
