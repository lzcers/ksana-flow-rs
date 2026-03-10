//! Agent Runner - 执行器
//!
//! Runner 持有外部依赖（model, tool_executor），执行副作用。
//! 它是 Reducer 模式的执行层。

use async_stream::stream;
use futures::{future::join_all, stream::BoxStream, StreamExt};
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::agents::{
    AgentConfig, AgentEffectResult, AgentError, AgentEvent, AgentInput, AgentOutput, AgentState,
    reduce, apply_effect,
    ToolCall, ToolExecutor, ToolExecutorError, ToolResult,
};
use crate::models::ChatCapability;

/// Agent 执行器
///
/// 持有外部依赖，执行副作用（调用 LLM、执行工具）。
/// 不持有状态，状态由调用者管理。
pub struct AgentRunner<M, E>
where
    M: ChatCapability,
    E: ToolExecutor,
{
    model: Arc<Mutex<M>>,
    tool_executor: Arc<Mutex<E>>,
    config: AgentConfig,
}

impl<M, E> AgentRunner<M, E>
where
    M: ChatCapability,
    E: ToolExecutor,
{
    /// 创建新的 Runner
    pub fn new(model: M, tool_executor: E) -> Self {
        Self {
            model: Arc::new(Mutex::new(model)),
            tool_executor: Arc::new(Mutex::new(tool_executor)),
            config: AgentConfig::new(),
        }
    }

    /// 设置配置
    pub fn with_config(mut self, config: AgentConfig) -> Self {
        self.config = config;
        self
    }

    /// 设置最大迭代次数
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.config.max_iterations = max;
        self
    }

    /// 获取配置引用
    pub fn config(&self) -> &AgentConfig {
        &self.config
    }
}

impl<M, E> AgentRunner<M, E>
where
    M: ChatCapability + Send + 'static,
    E: ToolExecutor + Send + 'static,
{
    /// 执行一步
    ///
    /// 根据当前状态决定下一步操作，执行副作用，返回新状态和输出。
    pub async fn step(
        &self,
        mut state: AgentState,
    ) -> Result<(AgentState, AgentOutput), AgentError> {
        // 更新状态为 Running
        if state.state == crate::agents::JobState::Pending {
            state.transition(crate::agents::JobState::Running, Some("开始执行".to_string()));
        }

        // 调用 reducer 获取指令
        let (state, output) = reduce(state, AgentInput::Continue, &self.config);

        // 根据指令执行副作用
        match output {
            AgentOutput::ChatRequest { messages, tools } => {
                // 调用 LLM
                let response = {
                    let model_guard = self.model.lock().await;
                    model_guard
                        .chat(messages, if tools.is_empty() { None } else { Some(tools) })
                        .await
                        .map_err(AgentError::Chat)?
                };

                Ok(apply_effect(state, AgentEffectResult::ChatResponse(response), &self.config))
            }

            AgentOutput::ToolCalls(calls) => {
                // 执行工具
                let results = self.execute_tools(&calls).await;
                Ok(apply_effect(state, AgentEffectResult::ToolResults(results), &self.config))
            }

            AgentOutput::Complete | AgentOutput::MaxIterationsReached | AgentOutput::BudgetExceeded => {
                Ok((state, output))
            }
        }
    }

    /// 添加用户消息并执行
    pub async fn run_with_message(
        &self,
        mut state: AgentState,
        message: crate::core::Message,
    ) -> Result<(AgentState, AgentOutput), AgentError> {
        // 添加用户消息
        state.context.add_message(message);

        // 执行
        self.step(state).await
    }

    /// 流式执行
    ///
    /// 返回事件流，可以实时获取执行进度。
    pub async fn run_stream(
        &self,
        initial_state: AgentState,
    ) -> Result<BoxStream<'static, Result<AgentEvent, AgentError>>, AgentError> {
        let model = Arc::clone(&self.model);
        let tool_executor = Arc::clone(&self.tool_executor);
        let config = self.config.clone();

        Ok(stream! {
            let mut state = initial_state;

            // 更新状态为 Running
            if state.state == crate::agents::JobState::Pending {
                state.transition(crate::agents::JobState::Running, Some("开始执行".to_string()));
            }

            loop {
                // 调用 reducer
                let (new_state, output) = reduce(state, AgentInput::Continue, &config);
                state = new_state;

                match output {
                    AgentOutput::ChatRequest { messages, tools } => {
                        // 调用 LLM
                        let response = {
                            let model_guard = model.lock().await;
                            model_guard
                                .chat(messages, if tools.is_empty() { None } else { Some(tools.clone()) })
                                .await
                                .map_err(AgentError::Chat)?
                        };

                        yield Ok(AgentEvent::AssistantMessage(response.clone()));

                        // 应用效果
                        let (new_state, output) = apply_effect(
                            state,
                            AgentEffectResult::ChatResponse(response),
                            &config,
                        );
                        state = new_state;

                        // 检查输出
                        match output {
                            AgentOutput::ToolCalls(calls) => {
                                yield Ok(AgentEvent::ToolCalls(calls.clone()));

                                // 执行工具
                                let results = execute_tools_internal(&tool_executor, &calls).await;

                                // 发送工具结果事件
                                for (_call, result) in results.iter() {
                                    match result {
                                        Ok(tool_result) => {
                                            yield Ok(AgentEvent::ToolResult {
                                                call_id: tool_result.id.clone(),
                                                success: tool_result.success,
                                                output: tool_result.output.clone(),
                                            });
                                        }
                                        Err(e) => {
                                            yield Ok(AgentEvent::ToolResult {
                                                call_id: String::new(),
                                                success: false,
                                                output: serde_json::Value::String(format!("Error: {}", e)),
                                            });
                                        }
                                    }
                                }

                                // 应用工具结果
                                let (new_state, output) = apply_effect(
                                    state,
                                    AgentEffectResult::ToolResults(results),
                                    &config,
                                );
                                state = new_state;

                                yield Ok(AgentEvent::Iteration {
                                    iteration: state.iteration,
                                    message_count: state.context.conversation().len(),
                                });

                                match output {
                                    AgentOutput::Complete => {
                                        state.transition(
                                            crate::agents::JobState::Completed,
                                            Some("任务完成".to_string()),
                                        );
                                        yield Ok(AgentEvent::Complete(state.context.conversation()));
                                        return;
                                    }
                                    AgentOutput::MaxIterationsReached => {
                                        state.transition(
                                            crate::agents::JobState::Failed,
                                            Some("达到最大迭代次数".to_string()),
                                        );
                                        yield Err(AgentError::MaxIterationsReached(config.max_iterations));
                                        return;
                                    }
                                    AgentOutput::BudgetExceeded => {
                                        state.transition(
                                            crate::agents::JobState::Failed,
                                            Some("预算超限".to_string()),
                                        );
                                        yield Err(AgentError::InvalidMessage("预算超限".to_string()));
                                        return;
                                    }
                                    _ => continue,
                                }
                            }
                            AgentOutput::Complete => {
                                state.transition(
                                    crate::agents::JobState::Completed,
                                    Some("任务完成".to_string()),
                                );
                                yield Ok(AgentEvent::Complete(state.context.conversation()));
                                return;
                            }
                            _ => continue,
                        }
                    }

                    AgentOutput::ToolCalls(calls) => {
                        yield Ok(AgentEvent::ToolCalls(calls.clone()));

                        // 执行工具
                        let results = execute_tools_internal(&tool_executor, &calls).await;

                        // 发送工具结果事件
                        for (_call, result) in results.iter() {
                            match result {
                                Ok(tool_result) => {
                                    yield Ok(AgentEvent::ToolResult {
                                        call_id: tool_result.id.clone(),
                                        success: tool_result.success,
                                        output: tool_result.output.clone(),
                                    });
                                }
                                Err(e) => {
                                    yield Ok(AgentEvent::ToolResult {
                                        call_id: String::new(),
                                        success: false,
                                        output: serde_json::Value::String(format!("Error: {}", e)),
                                    });
                                }
                            }
                        }

                        // 应用工具结果
                        let (new_state, output) = apply_effect(
                            state,
                            AgentEffectResult::ToolResults(results),
                            &config,
                        );
                        state = new_state;

                        yield Ok(AgentEvent::Iteration {
                            iteration: state.iteration,
                            message_count: state.context.conversation().len(),
                        });

                        match output {
                            AgentOutput::Complete => {
                                state.transition(
                                    crate::agents::JobState::Completed,
                                    Some("任务完成".to_string()),
                                );
                                yield Ok(AgentEvent::Complete(state.context.conversation()));
                                return;
                            }
                            AgentOutput::MaxIterationsReached => {
                                state.transition(
                                    crate::agents::JobState::Failed,
                                    Some("达到最大迭代次数".to_string()),
                                );
                                yield Err(AgentError::MaxIterationsReached(config.max_iterations));
                                return;
                            }
                            AgentOutput::BudgetExceeded => {
                                state.transition(
                                    crate::agents::JobState::Failed,
                                    Some("预算超限".to_string()),
                                );
                                yield Err(AgentError::InvalidMessage("预算超限".to_string()));
                                return;
                            }
                            _ => continue,
                        }
                    }

                    AgentOutput::Complete => {
                        state.transition(
                            crate::agents::JobState::Completed,
                            Some("任务完成".to_string()),
                        );
                        yield Ok(AgentEvent::Complete(state.context.conversation()));
                        return;
                    }

                    AgentOutput::MaxIterationsReached => {
                        state.transition(
                            crate::agents::JobState::Failed,
                            Some("达到最大迭代次数".to_string()),
                        );
                        yield Err(AgentError::MaxIterationsReached(config.max_iterations));
                        return;
                    }

                    AgentOutput::BudgetExceeded => {
                        state.transition(
                            crate::agents::JobState::Failed,
                            Some("预算超限".to_string()),
                        );
                        yield Err(AgentError::InvalidMessage("预算超限".to_string()));
                        return;
                    }
                }
            }
        }
        .boxed())
    }

    /// 执行工具
    async fn execute_tools(
        &self,
        tool_calls: &[ToolCall],
    ) -> Vec<(ToolCall, Result<ToolResult, ToolExecutorError>)> {
        execute_tools_internal(&self.tool_executor, tool_calls).await
    }
}

/// 执行工具（内部函数）
async fn execute_tools_internal<E: ToolExecutor>(
    tool_executor: &Arc<Mutex<E>>,
    tool_calls: &[ToolCall],
) -> Vec<(ToolCall, Result<ToolResult, ToolExecutorError>)> {
    let futures = tool_calls.iter().map(|call| {
        let executor = Arc::clone(tool_executor);
        let call = call.clone();
        async move {
            let result = executor.lock().await.execute(call.clone()).await;
            (call, result)
        }
    });

    join_all(futures).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::{GenericToolExecutor, ToolDef};
    use crate::core::Message;
    use crate::models::{ChatError, ChatModel};
    use crate::providers::DeepSeekProvider;
    use async_trait::async_trait;

    // Mock ChatCapability for testing
    struct MockModel;

    #[async_trait]
    impl ChatCapability for MockModel {
        async fn chat(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<Message, ChatError> {
            Ok(Message::assistant("Hello from mock!"))
        }

        async fn chat_stream(
            &self,
            _msgs: Vec<Message>,
            _tools: Option<Vec<ToolDef>>,
        ) -> Result<BoxStream<'static, crate::models::ChatChunk>, ChatError> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_runner_step() {
        let model = MockModel;
        let tool_executor = GenericToolExecutor::new();
        let runner = AgentRunner::new(model, tool_executor);

        let state = AgentState::new("user-123")
            .with_user_message("Hello");

        let (new_state, output) = runner.step(state).await.unwrap();

        assert!(matches!(output, AgentOutput::Complete));
        assert_eq!(new_state.state, crate::agents::JobState::Completed);
    }

    #[tokio::test]
    async fn test_runner_max_iterations() {
        let model = MockModel;
        let tool_executor = GenericToolExecutor::new();
        let runner = AgentRunner::new(model, tool_executor)
            .with_max_iterations(0);

        let state = AgentState::new("user-123")
            .with_user_message("Hello");

        let result = runner.step(state).await;

        // 由于 max_iterations = 0，应该达到最大迭代
        assert!(result.is_ok());
        let (_, output) = result.unwrap();
        assert!(matches!(output, AgentOutput::MaxIterationsReached));
    }

    #[tokio::test]
    async fn test_runner_with_real_model() {
        dotenv::dotenv().ok();

        let provider = match DeepSeekProvider::from_env() {
            Ok(p) => Arc::new(p) as Arc<dyn crate::providers::Provider>,
            Err(_) => {
                eprintln!("DEEPSEEK_API_KEY not set, skipping test");
                return;
            }
        };

        let mut model = ChatModel::new();
        model.add_model_provider("deepseek-chat", provider);
        model.set_active_model("deepseek-chat").unwrap();

        let tool_executor = GenericToolExecutor::new();
        let runner = AgentRunner::new(model, tool_executor)
            .with_max_iterations(3);

        let state = AgentState::new("user-123")
            .with_user_message("说 '测试成功'");

        let (new_state, output) = runner.step(state).await.unwrap();

        assert!(matches!(output, AgentOutput::Complete));
        let messages = new_state.context.conversation();
        assert!(messages.iter().any(|m| m.content().contains("测试成功") || m.content().contains("成功")));
    }
}