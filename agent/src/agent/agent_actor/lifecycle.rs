use std::ops::ControlFlow;
use std::pin::pin;
use std::time::Duration;

use futures::StreamExt;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use super::AgentActorEvent;
use crate::agent::call_model::{
    CallModelEvent, CallToolError, CallToolResult, call_model, call_tools,
};
use crate::agent::hooks::LifeCycleHook;
use crate::agent::hooks::ask_user::AskUserHook;
use crate::agent::hooks::execution_policy::ExecutionPolicyHook;
use crate::agent::hooks::metrics::MetricsHook;
use crate::agent::hooks::send_model_evt::SendModelEvtHook;
use crate::agent::hooks::update_frame::UpdateFrameHook;
use crate::agent::{AgentState, ToolCall, ToolExecutor};
use crate::core::Usage;
use crate::models::ChatCapability;

//   - BeforeStep: 扩展 step 级控制。适合做最大迭代检查、预算/配额校验、任务取消判断、加载记忆、恢复 checkpoint、初始化 tracing/span。
//   - BeforeCallModel: 扩展模型调用前编排。适合做 Prompt 注入、上下文裁剪/压缩、模型路由、动态开关工具、调用前安全策略检查。
//   - OnModelEvent: 扩展流式模型事件处理。适合做 UI 流式输出、token/耗时统计、增量内容审核、收集 reasoning、拼装 tool call、实时日志。
//   - AfterCallModel: 扩展模型完成后的收敛处理。适合做结果校验、结构化解析、tool call 合法性检查、重试/降级决策、决定 Done 还是 Continue。
//   - BeforeCallTools: 扩展工具执行前治理。适合做权限审批、参数修正、工具路由、并发/超时/重试策略、缓存命中、危险工具拦截。
//   - AfterCallTools: 扩展工具结果后处理。适合做结果标准化、错误映射、脱敏、结果持久化、缓存写回、把工具输出整理成下一轮上下文。
//   - AfterStep: 扩展 step 提交与收尾。适合做上下文落库、iteration 自增、状态迁移、发送迭代事件、生成审计记录、判断终态。
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum LifeCycle {
    BeforeStep,
    BeforeCallModel,
    OnModelEvent,
    AfterCallModel,
    BeforeCallTools,
    AfterCallTools,
    AfterStep,
}

pub enum LifeCycleResult {
    None,
    Frame(StepFrame),
}

pub enum StepExecutionResult {
    Frame(StepFrame),
    AskUser { question: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum LifeCycleInterrupt {
    // 错误中断
    HookError(LifeCycle, String, String),
    ToolError(String),
    ModelError,
    // 特殊操作中断
    AskUser { question: String },
}

impl std::fmt::Display for LifeCycleInterrupt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HookError(stage, hook, msg) => {
                write!(f, "hook error: {:?} {} {}", stage, hook, msg)
            }
            Self::ToolError(err) => write!(f, "tool error: {}", err),
            Self::ModelError => write!(f, "model error"),
            Self::AskUser { question } => write!(f, "ask user: {}", question),
        }
    }
}

impl std::error::Error for LifeCycleInterrupt {}

impl LifeCycleInterrupt {
    pub fn hook_error(stage: &LifeCycle, hook_name: &str, msg: String) -> Self {
        Self::HookError(stage.clone(), hook_name.to_string(), msg)
    }
    pub fn tool_error(err: String) -> Self {
        Self::ToolError(err)
    }
    pub fn ask_user(question: String) -> Self {
        Self::AskUser { question }
    }
}

pub type LifeCycleFlow = ControlFlow<LifeCycleInterrupt, LifeCycleResult>;

// 生命周期上下文, 给 Hook 使用
pub struct LifeCycleContext {
    pub stage: LifeCycle,
    pub state: AgentState,
    pub frame: StepFrame,
    pub model_event: Option<CallModelEvent>,
    pub agent_tx: Option<mpsc::Sender<AgentActorEvent>>,
}

impl LifeCycleContext {
    pub fn set_stage(&mut self, stage: &LifeCycle) {
        self.stage = stage.clone();
    }
    pub fn set_state(&mut self, state: AgentState) {
        self.state = state;
    }
    pub fn set_frame(&mut self, frame: StepFrame) {
        self.frame = frame;
    }
    pub fn set_agent_tx(&mut self, agent_tx: Option<mpsc::Sender<AgentActorEvent>>) {
        self.agent_tx = agent_tx;
    }
    pub fn set_model_event(&mut self, model_event: &CallModelEvent) {
        self.model_event = Some(model_event.clone());
    }
    pub fn set_frame_model_output(&mut self, model_output: ModelOuput) {
        self.frame.set_model_output(model_output);
    }
    pub fn set_frame_tools_result(&mut self, tools_result: Vec<CallToolResult>) {
        self.frame.set_tools_result(tools_result);
    }
    pub fn set_frame_token_usage(&mut self, token_usage: Usage) {
        self.frame.set_token_usage(token_usage);
    }
    pub fn set_frame_call_model_duration_ms(&mut self, duration_ms: u32) {
        self.frame.set_call_model_duration_ms(duration_ms);
    }
    pub fn set_frame_call_tools_duration_ms(&mut self, duration_ms: u32) {
        self.frame.set_call_tools_duration_ms(duration_ms);
    }
}

impl Default for LifeCycleContext {
    fn default() -> Self {
        Self {
            stage: LifeCycle::BeforeStep,
            state: Default::default(),
            frame: Default::default(),
            model_event: None,
            agent_tx: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelOuput {
    pub content: String,
    pub reasoning_content: Option<String>,
    pub tools_call: Option<Vec<ToolCall>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StepMetrics {
    pub call_model_duration_ms: Option<u32>,
    pub call_tools_duration_ms: Option<u32>,
}

impl Default for StepMetrics {
    fn default() -> Self {
        Self {
            call_model_duration_ms: None,
            call_tools_duration_ms: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StepFrame {
    pub model_output: Option<ModelOuput>,
    pub tools_result: Option<Vec<CallToolResult>>,
    pub token_usage: Option<Usage>,
    pub metrics: StepMetrics,
}

impl StepFrame {
    pub fn get_tools_call(&self) -> Option<&Vec<ToolCall>> {
        self.model_output
            .as_ref()
            .and_then(|o| o.tools_call.as_ref())
    }
    pub fn set_model_output(&mut self, model_output: ModelOuput) {
        self.model_output = Some(model_output);
    }
    pub fn set_tools_result(&mut self, tools_result: Vec<CallToolResult>) {
        self.tools_result = Some(tools_result);
    }
    pub fn set_token_usage(&mut self, token_usage: Usage) {
        self.token_usage = Some(token_usage);
    }
    pub fn set_call_model_duration_ms(&mut self, duration_ms: u32) {
        self.metrics.call_model_duration_ms = Some(duration_ms);
    }
    pub fn set_call_tools_duration_ms(&mut self, duration_ms: u32) {
        self.metrics.call_tools_duration_ms = Some(duration_ms);
    }
}

impl Default for StepFrame {
    fn default() -> Self {
        Self {
            model_output: None,
            tools_result: None,
            token_usage: None,
            metrics: Default::default(),
        }
    }
}

pub(super) struct StepLifeCycle {
    hooks: Vec<Box<dyn LifeCycleHook>>,
    ctx: LifeCycleContext,
}

impl StepLifeCycle {
    pub(super) fn new(state: AgentState) -> Self {
        let mut ctx = LifeCycleContext::default();
        ctx.set_state(state);
        Self {
            ctx,
            hooks: vec![
                Box::new(ExecutionPolicyHook::new()),
                Box::new(MetricsHook::new()),
                Box::new(UpdateFrameHook::new()),
                Box::new(AskUserHook::new()),
                Box::new(SendModelEvtHook::new()),
            ],
        }
    }

    pub(super) async fn start(
        &mut self,
        model: &(dyn ChatCapability + Sync),
        tool_executor: &dyn ToolExecutor,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
    ) -> LifeCycleFlow {
        self.ctx.set_agent_tx(event_tx.cloned());
        let messages = self.ctx.state.get_message();
        let tools = tool_executor.tools().clone();

        self.call_life_cycle_hook(LifeCycle::BeforeStep).await?;
        self.call_life_cycle_hook(LifeCycle::BeforeCallModel)
            .await?;

        let mut stream = pin!(call_model(model, &messages, Some(&tools)));
        while let Some(evt) = stream.next().await {
            self.ctx.set_model_event(&evt);
            self.call_life_cycle_hook(LifeCycle::OnModelEvent).await?;
        }

        // 调用 AfterCallModel 钩子，这里会有 AskUserHook 检测 ask_user 工具
        self.call_life_cycle_hook(LifeCycle::AfterCallModel).await?;

        // 继续执行正常流程
        if let Some(tools_call) = self.ctx.frame.get_tools_call().cloned() {
            // 执行工具
            self.call_life_cycle_hook(LifeCycle::BeforeCallTools)
                .await?;

            match Self::execute_tools_with_timeout(
                tool_executor,
                &tools_call,
                Some(Duration::from_secs(120)),
            )
            .await
            {
                Ok(results) => self.ctx.set_frame_tools_result(results),
                Err(err) => {
                    return Self::break_step(LifeCycleInterrupt::tool_error(err.to_string()));
                }
            }
            self.call_life_cycle_hook(LifeCycle::AfterCallTools).await?;
        };

        self.call_life_cycle_hook(LifeCycle::AfterStep).await?;

        Self::continue_step_with_result(LifeCycleResult::Frame(self.ctx.frame.clone()))
    }

    fn break_step(err: LifeCycleInterrupt) -> LifeCycleFlow {
        LifeCycleFlow::Break(err)
    }

    fn continue_step() -> LifeCycleFlow {
        LifeCycleFlow::Continue(LifeCycleResult::None)
    }
    fn continue_step_with_result(result: LifeCycleResult) -> LifeCycleFlow {
        LifeCycleFlow::Continue(result)
    }

    async fn call_life_cycle_hook(&mut self, lifecycle: LifeCycle) -> LifeCycleFlow {
        self.ctx.set_stage(&lifecycle);
        for hook in &mut self.hooks {
            if hook.on(&lifecycle) {
                let result = hook.handle(&mut self.ctx).await;
                // 如果是中断（包括错误或 AskUser），立即返回
                if let LifeCycleFlow::Break(_) = result {
                    return result;
                }
            }
        }
        Self::continue_step()
    }

    async fn execute_tools_with_timeout(
        tool_executor: &dyn ToolExecutor,
        tool_calls: &[crate::agent::ToolCall],
        timeout: Option<Duration>,
    ) -> Result<Vec<CallToolResult>, CallToolError> {
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
                Err(_) => Err(CallToolError::Timeout),
            }
        } else {
            Ok(execute.await)
        }
    }
}
