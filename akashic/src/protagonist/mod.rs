mod prompt;
pub mod types;
#[allow(unused_imports)]
pub use types::*;

use agent::{
    agent::{AgentActor, Context, GenericToolExecutor, LayerKind},
    core::Message,
    models::ChatModel,
};
use serde_json::json;
use tokio::sync::mpsc;

use crate::{
    channel::{AgentChannel, AkashicEvent, FateWeaverMessage, ProtagonistMessage},
    shared::{build_chat_model, build_layer, extract_step_content, parse_json_response},
};
use prompt::SYS_PROMPT;



pub struct Protagonist {
    agent_actor: AgentActor<ChatModel, GenericToolExecutor>,
    channel: AgentChannel,
}

impl Protagonist {
    pub fn new(profile: String, channel: AgentChannel) -> Self {
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
            channel,
        }
    }

  pub async fn start(mut self, mut inbox: mpsc::Receiver<ProtagonistMessage>) {
        while let Some(message) = inbox.recv().await {
            match message {
                ProtagonistMessage::ActionRequest(request) => {
                    self.handle_action_request(request).await;
                }
            }
        }
    }

    async fn handle_action_request(&mut self, request: ProtagonistActionRequest) {
        let prompt = build_action_request_prompt(&request);
        self.agent_actor.context_mut().add_message(Message::user(prompt));

        let decision = match extract_step_content(self.agent_actor.run_step(None).await)
            .and_then(|raw| parse_json_response::<ProtagonistDecision>(&raw))
        {
            Ok(decision) => decision,
            Err(err) => fallback_decision(&request, err),
        };

        self.channel.send_event(AkashicEvent::ProtagonistDecisionMade {
            round: request.round,
            choice_id: decision.choice_id.clone(),
            action: decision.action.clone(),
            rationale: decision.rationale.clone(),
        });

        if let Err(err) = self
            .channel
            .send_fate_weaver(FateWeaverMessage::ProtagonistDecision(decision))
            .await
        {
            eprintln!("failed to send protagonist decision: {:?}", err);
        }
    }

}

fn build_action_request_prompt(request: &ProtagonistActionRequest) -> String {
    let request_json = serde_json::to_string_pretty(request)
        .unwrap_or_else(|_| "{\"error\":\"serialize request failed\"}".to_string());
    format!(
        "请基于以下世界快照与候选行动，为主角选择一个最合适的动作。\n\
必须只返回 JSON，不要输出额外说明。\n\
返回格式:\n\
{{\n  \"choice_id\": \"选项 id\",\n  \"action\": \"主角将执行的具体动作\",\n  \"rationale\": \"不超过60字的简短理由\"\n}}\n\
\n请求数据:\n{}",
        request_json
    )
}

fn fallback_decision(request: &ProtagonistActionRequest, reason: String) -> ProtagonistDecision {
    if let Some(choice) = request.choices.first() {
        return ProtagonistDecision {
            choice_id: choice.id.clone(),
            action: choice.next_trigger.clone(),
            rationale: format!("模型解析失败，按首个选项回退: {}", reason),
        };
    }

    ProtagonistDecision {
        choice_id: "wait".to_string(),
        action: "主角暂时观察局势，等待新的可行动线索。".to_string(),
        rationale: format!("无可选项，默认保守推进: {}", reason),
    }
}
