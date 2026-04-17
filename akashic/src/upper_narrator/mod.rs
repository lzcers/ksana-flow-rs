mod prompt;
pub mod types;
pub use types::*;

use agent::{
    agent::{AgentActor, Context, GenericToolExecutor, LayerKind},
    core::Message,
    models::ChatModel,
};
use serde_json::json;
use prompt::SYS_PORMPT;
use tokio::sync::mpsc;
use crate::{
    channel::{AgentChannel, AkashicEvent, FateWeaverMessage, UpperNarratorMessage},
    shared::{build_chat_model, build_layer, extract_step_content},
};


pub struct UpperNarrator {
    agent_actor: AgentActor<ChatModel, GenericToolExecutor>,
    channel: AgentChannel,
}

impl UpperNarrator {
    pub fn new(channel: AgentChannel) -> Self { 
        let mut model = build_chat_model();
        model.set_output_json(false);
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
            channel,
        }
    }

    pub async fn start(mut self, mut inbox: mpsc::Receiver<UpperNarratorMessage>) {
        while let Some(message) = inbox.recv().await {
            match message {
                UpperNarratorMessage::DraftScene(request) => {
                    self.handle_draft_scene(request).await;
                }
            }
        }
    }

    async fn handle_draft_scene(&mut self, request: UpperNarratorRequest) {
        let prompt = build_draft_scene_prompt(&request);
        self.agent_actor.context_mut().add_message(Message::user(prompt));

        let content = match extract_step_content(self.agent_actor.run_step(None).await) {
            Ok(content) => content.trim().to_string(),
            Err(err) => format!("叙事生成失败：{}", err),
        };

        let narration = UpperNarration {
            round: request.round,
            title: format!("{} · {}", request.chapter, request.section),
            content,
        };
        self.channel
            .send_event(AkashicEvent::NarrationGenerated(narration.clone()));
        if let Err(err) = self
            .channel
            .send_fate_weaver(FateWeaverMessage::NarrationGenerated(narration))
            .await
        {
            eprintln!("failed to send narration back to fate weaver: {:?}", err);
        }
    }

}

fn build_draft_scene_prompt(request: &UpperNarratorRequest) -> String {
    let request_json = serde_json::to_string_pretty(request)
        .unwrap_or_else(|_| "{\"error\":\"serialize request failed\"}".to_string());
    format!(
        "请把以下事实快照改写为面向读者的短篇叙事段落。\n\
要求：\n\
1. 不增删事实，不篡改因果。\n\
2. 可润色节奏、氛围和人物感受，但不要引入新设定。\n\
3. 直接输出正文，不要使用 JSON，不要附加说明。\n\
\n事实数据:\n{}",
        request_json
    )
}
