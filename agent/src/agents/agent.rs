use async_stream::stream;
use futures::{future::join_all, stream::BoxStream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use thiserror::Error;

use crate::{
    agents::{
        memory::{Memory, SlidingWindowMemory},
        ToolCall, ToolExecutor, ToolExecutorError, ToolResult,
    },
    core::Message,
    models::{ChatCapability, ChatError},
};

/// Agent 运行时事件，用于流式输出
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentEvent {
    /// 模型回复消息
    AssistantMessage(Message),
    /// 检测到工具调用
    ToolCalls(Vec<ToolCall>),
    /// 单个工具执行完成
    ToolResult {
        call_id: String,
        success: bool,
        output: Value,
    },
    /// 迭代完成（一轮 chat + tool execution）
    Iteration {
        iteration: usize,
        message_count: usize,
    },
    /// 执行完成
    Complete(Vec<Message>),
}

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Chat error: {0}")]
    Chat(#[from] ChatError),
    #[error("Tool execution error: {call_id}: {error}")]
    Tool {
        call_id: String,
        #[source]
        error: ToolExecutorError,
    },
    #[error("Max iterations reached: {0}")]
    MaxIterationsReached(usize),
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
    #[error("All tools failed in iteration")]
    AllToolsFailed,
}

/// Agent 结构
///
/// 泛型参数：
/// - `M`: Chat model (实现 ChatCapability)
/// - `E`: Tool executor (实现 ToolExecutor)
/// - `Mem`: Memory 存储 (实现 Memory trait，默认为 SlidingWindowMemory)
///
/// 使用 Arc<Mutex<>> 包装内部组件，支持 &self 访问的同时允许内部状态变化
pub struct Agent<M, E, Mem = SlidingWindowMemory>
where
    M: ChatCapability,
    E: ToolExecutor,
    Mem: Memory,
{
    model: Arc<Mutex<M>>,
    tool_executor: Arc<Mutex<E>>,
    memory: Arc<Mutex<Mem>>,
    max_iterations: usize,
}

impl<M, E> Agent<M, E, SlidingWindowMemory>
where
    M: ChatCapability + Send + 'static,
    E: ToolExecutor + Send + 'static,
{
    /// 创建新的 Agent，使用默认的滑动窗口记忆（无限制）
    pub fn new(model: M, tool_executor: E) -> Self {
        Self {
            model: Arc::new(Mutex::new(model)),
            tool_executor: Arc::new(Mutex::new(tool_executor)),
            memory: Arc::new(Mutex::new(SlidingWindowMemory::unbounded())),
            max_iterations: 10,
        }
    }
}

impl<M, E, Mem> Agent<M, E, Mem>
where
    M: ChatCapability + Send + 'static,
    E: ToolExecutor + Send + 'static,
    Mem: Memory + Send + 'static,
{
    /// 使用自定义 Memory 创建 Agent
    pub fn with_memory(model: M, tool_executor: E, memory: Mem) -> Self {
        Self {
            model: Arc::new(Mutex::new(model)),
            tool_executor: Arc::new(Mutex::new(tool_executor)),
            memory: Arc::new(Mutex::new(memory)),
            max_iterations: 10,
        }
    }

    /// 设置最大迭代次数
    pub fn with_max_iterations(mut self, max_iterations: usize) -> Self {
        self.max_iterations = max_iterations;
        self
    }

    /// 获取 memory 的引用
    pub fn memory(&self) -> &Arc<Mutex<Mem>> {
        &self.memory
    }

    /// 获取 model 的引用
    pub fn model(&self) -> &Arc<Mutex<M>> {
        &self.model
    }

    /// 获取 tool_executor 的引用
    pub fn tool_executor(&self) -> &Arc<Mutex<E>> {
        &self.tool_executor
    }

    /// 添加消息到记忆
    pub async fn add_to_memory(&self, message: &Message) {
        self.memory.lock().await.add(message).await;
    }

    /// 清空记忆
    pub async fn clear_memory(&self) {
        self.memory.lock().await.clear().await;
    }

    /// 流式运行 Agent，返回事件流
    ///
    /// 这是推荐的默认使用方式，可以实时获取：
    /// - 每轮模型回复
    /// - 工具调用列表
    /// - 每个工具执行结果
    /// - 迭代进度
    /// - 最终结果
    ///
    /// # 参数
    /// - `messages`: 新消息列表，会被添加到记忆中
    ///
    /// # 行为
    /// 1. 将输入消息添加到记忆
    /// 2. 每轮从记忆获取完整上下文发送给模型
    /// 3. 将模型回复和工具结果添加到记忆
    /// 4. 返回事件流
    pub async fn run_stream(
        &self,
        messages: Vec<Message>,
    ) -> Result<BoxStream<'static, Result<AgentEvent, AgentError>>, AgentError> {
        let model = Arc::clone(&self.model);
        let tool_executor = Arc::clone(&self.tool_executor);
        let memory = Arc::clone(&self.memory);
        let max_iterations = self.max_iterations;

        // 先将输入消息添加到记忆
        {
            let mem = memory.lock().await;
            for msg in &messages {
                mem.add(msg).await;
            }
        }

        Ok(stream! {
            let tools = tool_executor.lock().await.tools().clone();

            for iteration in 0..max_iterations {
                // 从记忆获取完整上下文
                let context = memory.lock().await.get_all().await;

                // Chat
                let response = {
                    let model_guard = model.lock().await;
                    model_guard
                        .chat(context, Some(tools.clone()))
                        .await?
                };

                yield Ok(AgentEvent::AssistantMessage(response.clone()));

                // 将回复添加到记忆
                memory.lock().await.add(&response).await;

                // Extract tool calls
                let tool_calls = match &response {
                    Message::Assistant { tool_calls, .. } => tool_calls,
                    _ => {
                        yield Err(AgentError::InvalidMessage(
                            "Expected assistant message".to_string(),
                        ));
                        return;
                    }
                };

                let Some(tool_calls) = tool_calls else {
                    let final_messages = memory.lock().await.get_all().await;
                    yield Ok(AgentEvent::Complete(final_messages));
                    return;
                };

                if tool_calls.is_empty() {
                    let final_messages = memory.lock().await.get_all().await;
                    yield Ok(AgentEvent::Complete(final_messages));
                    return;
                }

                yield Ok(AgentEvent::ToolCalls(tool_calls.clone()));

                // Execute tools and collect results
                let tool_results = execute_tools(&tool_executor, tool_calls).await;

                // 发送每个工具的结果事件并添加到记忆
                for (call, result) in tool_calls.iter().zip(tool_results.iter()) {
                    match result {
                        Ok(tool_result) => {
                            yield Ok(AgentEvent::ToolResult {
                                call_id: tool_result.id.clone(),
                                success: tool_result.success,
                                output: tool_result.output.clone(),
                            });

                            // 添加工具结果到记忆
                            let tool_msg = Message::Tool {
                                tool_call_id: tool_result.id.clone(),
                                content: serde_json::to_string(&tool_result.output).unwrap_or_default(),
                            };
                            memory.lock().await.add(&tool_msg).await;
                        }
                        Err(e) => {
                            yield Ok(AgentEvent::ToolResult {
                                call_id: call.id.clone(),
                                success: false,
                                output: Value::String(format!("Error: {}", e)),
                            });

                            // 添加错误信息到记忆
                            let tool_msg = Message::Tool {
                                tool_call_id: call.id.clone(),
                                content: format!("Error: {}", e),
                            };
                            memory.lock().await.add(&tool_msg).await;
                        }
                    }
                }

                let msg_count = memory.lock().await.len().await;
                yield Ok(AgentEvent::Iteration {
                    iteration,
                    message_count: msg_count,
                });

                if iteration + 1 >= max_iterations {
                    yield Err(AgentError::MaxIterationsReached(max_iterations));
                    return;
                }
            }

            let final_messages = memory.lock().await.get_all().await;
            yield Ok(AgentEvent::Complete(final_messages));
        }
        .boxed())
    }

    /// 非流式运行，收集所有事件后返回最终消息列表
    ///
    /// 这是 `run_stream` 的降级包装，适用于不需要实时反馈的简单场景
    pub async fn run(&self, messages: Vec<Message>) -> Result<Vec<Message>, AgentError> {
        let mut stream = self.run_stream(messages).await?;

        let mut final_messages = None;
        let mut last_error = None;

        while let Some(event) = stream.next().await {
            match event {
                Ok(AgentEvent::Complete(messages)) => {
                    final_messages = Some(messages);
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                }
                _ => continue,
            }
        }

        if let Some(messages) = final_messages {
            Ok(messages)
        } else if let Some(e) = last_error {
            Err(e)
        } else {
            Err(AgentError::InvalidMessage(
                "Stream ended without Complete event".to_string(),
            ))
        }
    }

    /// 继续对话（使用已有记忆）
    ///
    /// 与 `run` 类似，但不需要传入历史消息，直接使用记忆中的上下文
    pub async fn continue_conversation(&self, message: Message) -> Result<Vec<Message>, AgentError> {
        self.run(vec![message]).await
    }
}

/// 执行工具，返回所有结果（包含成功和失败）
///
/// 关键改进：
/// 1. 单个工具失败不影响其他工具
/// 2. 所有工具都会执行完毕
/// 3. 返回 Vec<Result<ToolResult, ToolExecutorError>> 让调用者决定如何处理
async fn execute_tools<E: ToolExecutor>(
    tool_executor: &Arc<Mutex<E>>,
    tool_calls: &[ToolCall],
) -> Vec<Result<ToolResult, ToolExecutorError>> {
    let futures = tool_calls.iter().map(|call| {
        let executor = Arc::clone(tool_executor);
        let call = call.clone();
        async move { executor.lock().await.execute(call).await }
    });

    join_all(futures).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::memory::ContextualMemory;
    use crate::agents::tools::GenericToolExecutor;
    use crate::models::ChatModel;
    use crate::providers::DeepSeekProvider;

    #[tokio::test]
    async fn test_agent_with_memory() {
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
        let memory = SlidingWindowMemory::new(10);
        let agent = Agent::with_memory(model, tool_executor, memory);

        // 第一次对话
        let result = agent.run(vec![Message::user("我叫张三")]).await;
        assert!(result.is_ok());

        // 继续对话，测试记忆
        let result = agent.continue_conversation(Message::user("我叫什么名字？")).await;
        assert!(result.is_ok());

        let messages = result.unwrap();
        // 最后一条应该是助手的回复
        if let Some(last) = messages.last() {
            let content = last.content();
            // 助手应该记得名字
            assert!(
                content.contains("张三") || content.contains("你的名字"),
                "Agent should remember the name. Got: {}",
                content
            );
        }
    }

    #[tokio::test]
    async fn test_agent_sliding_window() {
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
        let agent = Agent::new(model, tool_executor);

        agent.run(vec![Message::user("Hello")]).await.ok();

        // 验证记忆存储
        let messages = agent.memory.lock().await.get_all().await;
        assert!(messages.iter().any(|m| m.content().contains("Hello")));
    }

    #[tokio::test]
    async fn test_agent_with_contextual_memory() {
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
        let memory = ContextualMemory::new(10)
            .with_system_prompt("你是一个友好的助手，喜欢用表情符号。".to_string());

        let agent = Agent::with_memory(model, tool_executor, memory);

        let result = agent.run(vec![Message::user("你好")]).await;
        assert!(result.is_ok());

        // 检查记忆包含系统提示
        let messages = agent.memory.lock().await.get_all().await;
        assert!(messages.iter().any(|m| matches!(m, Message::System { .. })));
    }

    #[tokio::test]
    async fn test_agent_with_markdown_memory() {
        use crate::agents::memory::PersistentMemory;

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
        let memory = crate::agents::memory::MarkdownMemory::new(10);

        let agent = Agent::with_memory(model, tool_executor, memory);

        let result = agent.run(vec![Message::user("Hello")]).await;
        assert!(result.is_ok());

        // 测试持久化
        let mem = agent.memory.lock().await;
        let md = mem.serialize().await.unwrap();
        assert!(md.contains("## 👤 User"));
        assert!(md.contains("Hello"));
    }
}