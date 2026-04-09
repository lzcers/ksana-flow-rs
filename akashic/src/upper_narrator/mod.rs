mod prompt;
pub mod types;

use agent::{
    agent::{AgentActor, Context, GenericToolExecutor, LayerKind},
    models::ChatModel,
};
use serde_json::json;
use prompt::SYS_PORMPT;
use crate::{
    shared::{build_chat_model, build_layer},
};


pub struct UpperNarrator {
    agent_actor: AgentActor<ChatModel, GenericToolExecutor>,
}

impl UpperNarrator {
    pub fn new() -> Self { 
        let model = build_chat_model();
        let tool_executor = GenericToolExecutor::new();
        let context = Context::new()
            .layer(build_layer(
                "upper-narrator-system",
                LayerKind::System,
                json!(SYS_PORMPT),
                100,
            ))
            .layer(build_layer(
                "upper-narrator-soul",
                LayerKind::Soul,
                json!({
                    "name": "上层叙事者",
                    "role": "将事实清单转写为文学叙事文本",
                    "guidelines": [
                        "绝不改写事实结果",
                        "要体现节奏、风格与情绪基调",
                        "输出应直接面向读者"
                    ]
                }),
                90,
            ));
        let agent_actor = AgentActor::new(model, tool_executor, context);
        Self {
            agent_actor,
        }
    }

    pub async fn start(mut self) {
   
    }


}
