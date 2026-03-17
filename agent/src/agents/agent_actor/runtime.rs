use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use futures::StreamExt;
use tokio::sync::mpsc;

use super::types::step_result_from_draft;
use super::{AgentActor, AgentActorEvent, AgentError, StepResult};
use crate::agents::agent_utils::{CallModelEvent, CallToolResult, call_model, call_tools};
use crate::agents::hooks::{
    AfterCallModel, AfterCallTools, AfterStep, BeforeCallModel, BeforeCallTools, ExecutionPolicy,
    HookRegistry, ModelCallOutput, ModelEventCtx, StepHookContext, StepResultDraft, StepScratchpad,
};
use crate::agents::{AgentState, ToolCall, ToolDef, ToolExecutor};
use crate::models::ChatCapability;

struct StepRuntime<'a> {
    state: &'a mut AgentState,
    hooks: &'a HookRegistry,
    event_tx: Option<&'a mpsc::Sender<AgentActorEvent>>,
    scratchpad: StepScratchpad,
    execution_policy: ExecutionPolicy,
}

impl<'a> StepRuntime<'a> {
    fn new(
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

    fn step_timeout(&self) -> Option<Duration> {
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

    async fn before_step(&mut self) -> Option<StepResultDraft> {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            hooks.before_step(&mut ctx).await
        };
        Self::phase_result(result)
    }

    async fn before_call_model(&mut self, tools: &[ToolDef]) -> Option<StepResultDraft> {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            let mut before_call_model = BeforeCallModel { tools };
            hooks
                .before_call_model(&mut ctx, &mut before_call_model)
                .await
        };
        Self::phase_result(result)
    }

    async fn on_model_event(&mut self, event: &CallModelEvent) -> Option<StepResultDraft> {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            let model_event = ModelEventCtx { event };
            hooks.on_model_event(&mut ctx, &model_event).await
        };
        Self::phase_result(result)
    }

    async fn after_call_model(&mut self, output: &mut ModelCallOutput) -> Option<StepResultDraft> {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            let mut after_call_model = AfterCallModel { output };
            hooks
                .after_call_model(&mut ctx, &mut after_call_model)
                .await
        };
        Self::phase_result(result)
    }

    async fn before_call_tools(
        &mut self,
        tool_calls: &mut Vec<ToolCall>,
    ) -> Option<StepResultDraft> {
        let result = {
            let hooks = self.hooks;
            let event_tx = self.event_tx;
            let mut ctx = Self::make_ctx(&mut *self.state, event_tx, &mut self.scratchpad);
            let mut before_call_tools = BeforeCallTools { tool_calls };
            hooks
                .before_call_tools(&mut ctx, &mut before_call_tools)
                .await
        };
        Self::phase_result(result)
    }

    async fn after_call_tools(
        &mut self,
        tool_calls: &[ToolCall],
        tool_results: &mut Vec<CallToolResult>,
    ) -> Option<StepResultDraft> {
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
        Self::phase_result(result)
    }

    async fn execute_core(
        &mut self,
        model: &(dyn ChatCapability + Sync),
        tool_executor: &dyn ToolExecutor,
    ) -> StepResultDraft {
        let tools = tool_executor.tools().clone();

        if let Some(result) = self.before_step().await {
            return result;
        }

        if let Some(result) = self.before_call_model(&tools).await {
            return result;
        }

        let mut model_output = match self.stream_model_output(model, &tools).await {
            Ok(output) => output,
            Err(result) => return result,
        };

        if let Some(result) = self.after_call_model(&mut model_output).await {
            return result;
        }

        if model_output.tool_calls.is_empty() {
            return StepResultDraft::Done {
                content: model_output.content,
                reasoning_content: model_output.reasoning_content,
            };
        }

        let mut tool_calls = model_output.tool_calls.clone();
        if let Some(result) = self.before_call_tools(&mut tool_calls).await {
            return result;
        }

        if tool_calls.is_empty() {
            return StepResultDraft::Done {
                content: model_output.content,
                reasoning_content: model_output.reasoning_content,
            };
        }

        let mut tool_results =
            match execute_tools_with_timeout(tool_executor, &tool_calls, self.tool_timeout()).await
            {
                Ok(results) => results,
                Err(err) => return StepResultDraft::Error(err),
            };

        if let Some(result) = self.after_call_tools(&tool_calls, &mut tool_results).await {
            return result;
        }

        StepResultDraft::Continue {
            content: model_output.content,
            reasoning_content: model_output.reasoning_content,
            tool_calls,
            tool_results,
        }
    }

    async fn stream_model_output(
        &mut self,
        model: &(dyn ChatCapability + Sync),
        tools: &[ToolDef],
    ) -> Result<ModelCallOutput, StepResultDraft> {
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

            if let Some(result) = self.on_model_event(&event).await {
                return Err(result);
            }

            if let Some(err) = model_error.take() {
                return Err(StepResultDraft::Error(err));
            }
        }

        Ok(model_output)
    }

    async fn finish(&mut self, mut result: StepResultDraft) -> StepResult {
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

    fn phase_result(
        result: Result<Option<StepResultDraft>, AgentError>,
    ) -> Option<StepResultDraft> {
        match result {
            Ok(result) => result,
            Err(err) => Some(StepResultDraft::Error(err)),
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

impl<C, E> AgentActor<C, E>
where
    C: ChatCapability + Send + Sync,
    E: ToolExecutor + Send,
{
    /// 执行单步迭代
    ///
    /// # Arguments
    /// * `event_tx` - 可选的事件发送器，用于报告执行过程中的各种事件
    ///
    /// # Returns
    /// 返回 `StepResult` 表示执行结果：
    /// - `Continue`: 有工具调用，需要继续迭代
    /// - `Done`: 无工具调用，执行完成
    /// - `Error`: 执行出错
    pub async fn run_step(
        &mut self,
        event_tx: Option<mpsc::Sender<AgentActorEvent>>,
    ) -> StepResult {
        let chat = Arc::clone(&self.chat);
        let tool_executor = Arc::clone(&self.tool_executor);
        let execution_policy = self.hooks.execution_policy(&self.state);
        let mut step = StepRuntime::new(
            &mut self.state,
            &self.hooks,
            event_tx.as_ref(),
            execution_policy,
        );

        let final_result = if let Some(timeout) = step.step_timeout() {
            match tokio::time::timeout(
                timeout,
                step.execute_core(chat.as_ref(), tool_executor.as_ref()),
            )
            .await
            {
                Ok(result) => result,
                Err(_) => StepResultDraft::Error(AgentError::Timeout),
            }
        } else {
            step.execute_core(chat.as_ref(), tool_executor.as_ref())
                .await
        };

        step.finish(final_result).await
    }
}
