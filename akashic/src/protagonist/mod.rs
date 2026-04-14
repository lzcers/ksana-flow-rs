mod prompt;
pub mod types;
#[allow(unused_imports)]
pub use types::*;

use agent::{
    agent::{AgentActor, Context, GenericToolExecutor, LayerKind},
    models::ChatModel,
};
use serde_json::json;
use tokio::sync::mpsc;

use crate::{
    channel::ProtagonistMessage,
    shared::{build_chat_model, build_layer},
};
use prompt::SYS_PROMPT;



pub struct Protagonist {
    agent_actor: AgentActor<ChatModel, GenericToolExecutor>,
}

impl Protagonist {
    pub fn new(profile: String) -> Self {
        let model = build_chat_model();
        let tool_executor = GenericToolExecutor::new();
        let system_prompt = SYS_PROMPT.replace("{input}", &profile);
        let context = Context::new()
            .layer(build_layer(
                "protagonist-system",
                LayerKind::System,
                json!(system_prompt),
                100,
            ))
            .layer(build_layer(
                "protagonist-soul",
                LayerKind::Soul,
                json!({
                    "name": "故事主角",
                    "role": "受限视角下的战术决策者",
                    "guidelines": [
                        "只能使用主角合理可知的信息",
                        "优先产出可执行行动，必要时再请求决策",
                        "如需决策请求，仍然给出推荐方案与推荐行动"
                    ]
                }),
                90,
            ))
            .layer(build_layer(
                "protagonist-profile",
                LayerKind::Memory,
                json!({ "profile": profile }),
                80,
            ));
        let agent_actor = AgentActor::new(model, tool_executor, context);
        Self {
            agent_actor,
        }
    }

  pub async fn start(self, mut inbox: mpsc::Receiver<ProtagonistMessage>) {
        while let Some(message) = inbox.recv().await {
            match message {
                ProtagonistMessage::Action => {}
            }
        }
    }

}
