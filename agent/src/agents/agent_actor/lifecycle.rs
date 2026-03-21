use std::ops::ControlFlow;
use std::pin::{Pin, pin};
use std::time::Duration;

use futures::{Stream, StreamExt};

use super::{AgentActorEvent, AgentError, StepResult};
use crate::agents::call_model::{CallModelEvent, CallToolResult, call_model, call_tools};
use crate::agents::hooks::LifeCycleHook;
use crate::agents::hooks::max_iter_limit::MaxIterLimitHook;
use crate::agents::{AgentState, ToolCall, ToolExecutor};
use crate::models::ChatCapability;

//   - BeforeStep: 扩展 step 级控制。适合做最大迭代检查、预算/配额校验、任务取消判断、加载记忆、恢复 checkpoint、初始化 tracing/span。
//   - BeforeCallModel: 扩展模型调用前编排。适合做 Prompt 注入、上下文裁剪/压缩、模型路由、动态开关工具、调用前安全策略检查。
//   - OnModelEvent: 扩展流式模型事件处理。适合做 UI 流式输出、token/耗时统计、增量内容审核、收集 reasoning、拼装 tool call、实时日志。
//   - AfterCallModel: 扩展模型完成后的收敛处理。适合做结果校验、结构化解析、tool call 合法性检查、重试/降级决策、决定 Done 还是 Continue。
//   - BeforeCallTools: 扩展工具执行前治理。适合做权限审批、参数修正、工具路由、并发/超时/重试策略、缓存命中、危险工具拦截。
//   - AfterCallTools: 扩展工具结果后处理。适合做结果标准化、错误映射、脱敏、结果持久化、缓存写回、把工具输出整理成下一轮上下文。
//   - AfterStep: 扩展 step 提交与收尾。适合做上下文落库、iteration 自增、状态迁移、发送迭代事件、生成审计记录、判断终态。
#[derive(Debug, Clone, PartialEq)]
pub enum LifeCycle {
    BeforeStep,
    BeforeCallModel,
    OnModelEvent,
    AfterCallModel,
    BeforeCallTools,
    AfterCallTools,
    AfterStep,
}

pub enum LifeCycleEffect {
    None,
}
impl Default for LifeCycleEffect {
    fn default() -> Self {
        Self::None
    }
}

pub enum LifeCycleError {
    // (LifecyclePhase, HookName, ErrorMessage)
    HookError(LifeCycle, String, String),
}
impl LifeCycleError {
    pub fn hook_error(stage: &LifeCycle, hook_name: &str, msg: String) -> Self {
        Self::HookError(stage.clone(), hook_name.to_string(), msg)
    }
}

pub type LifeCycleFlow = ControlFlow<LifeCycleError, LifeCycleEffect>;

pub struct LifeCycleContext {
    pub stage: LifeCycle,
    pub state: AgentState,
    pub frame: StepFrame,
    pub model_event: Option<CallModelEvent>,
}

impl Default for LifeCycleContext {
    fn default() -> Self {
        Self {
            stage: LifeCycle::BeforeStep,
            state: Default::default(),
            frame: Default::default(),
            model_event: None,
        }
    }
}
pub struct StepFrame {
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

pub(super) struct StepLifeCycle {
    state: AgentState,
    frame: StepFrame,
}

// 供外部调用
impl StepLifeCycle {
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
    ) -> LifeCycleFlow {
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

    fn break_step(err: LifeCycleError) -> LifeCycleFlow {
        ControlFlow::Break(err)
    }

    fn continue_step() -> LifeCycleFlow {
        ControlFlow::Continue(LifeCycleEffect::None)
    }

    async fn call_life_cyle_hook(&mut self, lifecycle: LifeCycle) -> LifeCycleFlow {
        let lctx = LifeCycleContext::default();
        MaxIterLimitHook::new(10).handle(&lctx).await?;
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
