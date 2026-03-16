use std::{pin::pin, sync::Arc};

use futures::StreamExt;
use tokio::sync::{Mutex, mpsc};

use crate::{
    agents::{
        AgentActorCommand, AgentActorEvent, AgentActorHandle, CallModelEvent, Context, ToolCall,
        ToolDef, ToolExecutor, call_model,
    },
    models::ChatCapability,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StepResult {
    Done,
    Continue,
    Err,
}

struct CallModelResult {
    reasoning: String,
    content: String,
    tool_call: Option<Vec<ToolCall>>,
}
impl Default for CallModelResult {
    fn default() -> Self {
        Self {
            reasoning: String::new(),
            content: String::new(),
            tool_call: None,
        }
    }
}

async fn before_call_model_handle(context: &mut Context, tools_def: &[ToolDef]) {}

async fn processing_call_model_handle(context: &mut Context, model_evt: CallModelEvent) {}

async fn after_call_model_handle(
    model_res: &CallModelResult,
    context: &mut Context,
    tools_executor: &dyn ToolExecutor,
) {
}

async fn step(
    model: &(dyn ChatCapability + Sync),
    context: &mut Context,
    tools_executor: &dyn ToolExecutor,
) -> StepResult {
    let msgs = context.to_messages();
    let tools_def = tools_executor.tools();
    // before call model
    // 1. Contex 预处理
    //   - Context 里有没有违规内容？
    //   - token 校验，是否超过最大 token 数，是否超过最大调用轮次约束？
    //   - Context 压缩或者其它转换，例如：压缩记忆层，合并对话层等。
    // 2. Agent 状态更新，观测指标状态更新、各类状态更新
    before_call_model_handle(context, tools_def).await;
    let mut model_res_stream = pin!(call_model(model, &msgs, Some(tools_def)));
    let model_result = CallModelResult::default();
    while let Some(model_evt) = model_res_stream.next().await {
        // prcessing call model
        // 1. 主要是把流式响应的结果透传到用户界面
        // 2. 处理调用异常，例如：调用超时、调用失败等。
        processing_call_model_handle(context, model_evt).await;
    }
    // after call model
    // 1. 处理工具调用
    // 2. 更新 Agent 状态，例如：迭代次数、调用次数、成本等，观测指标状态等。
    // 3. 决定是否进入下一轮
    after_call_model_handle(&model_result, context, tools_executor).await;
    StepResult::Continue
}

async fn agent_loop(
    model: Arc<dyn ChatCapability + Sync + Send>,
    context: Arc<Mutex<Context>>,
    tools_executor: Arc<dyn ToolExecutor + Send>,
) -> AgentActorHandle {
    let (cmd_tx, mut cmd_rx) = mpsc::channel::<AgentActorCommand>(16);
    let (event_tx, event_rx) = mpsc::channel::<AgentActorEvent>(64);
    tokio::spawn(async move {
        loop {
            // 控制面切切入点
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    AgentActorCommand::Pause => {}
                    AgentActorCommand::Continue => {}
                    AgentActorCommand::Cancel => {
                        break;
                    }
                }
            }
            let mut ctx = context.lock().await;
            let result = step(model.as_ref(), &mut ctx, tools_executor.as_ref()).await;
            if result == StepResult::Done {
                break;
            }
        }
    });

    AgentActorHandle { cmd_tx, event_rx }
}
