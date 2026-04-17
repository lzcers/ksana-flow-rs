pub mod component;
mod prompt;
pub mod types;
use agent::{
    agent::{AgentActor, AgentActorEvent, Context, GenericToolExecutor, LayerKind},
    core::Message,
    models::ChatModel,
};
use prompt::SYS_PROMPT;
use serde_json::json;
use tokio::sync::mpsc;

use crate::{
    channel::{
        AgentChannel, AkashicEvent, FateWeaverMessage, ProtagonistMessage, UpperNarratorMessage,
    },
    fate_weaver::types::FateNode,
    protagonist::ProtagonistActionRequest,
    shared::{build_chat_model, build_layer, extract_step_content, parse_json_response},
    upper_narrator::UpperNarratorRequest,
};

pub struct FateWeaver {
    agent_actor: AgentActor<ChatModel, GenericToolExecutor>,
    channel: AgentChannel,
    max_rounds: u32,
    current_round: u32,
}

impl FateWeaver {
    pub fn new(
        prota_profile: String,
        world_profile: String,
        channel: AgentChannel,
        max_rounds: u32,
    ) -> Self {
        let model = build_chat_model();
        let tool_executor = GenericToolExecutor::new();
        let system_prompt = SYS_PROMPT
            .replace("{world_profile}", &world_profile)
            .replace("{protagonist_profile}", &prota_profile);
        let context = Context::new()
            .layer(build_layer(
                "fate-weaver-system",
                LayerKind::System,
                json!(system_prompt),
                100,
            ))
            .layer(build_layer(
                "fate-weaver-soul",
                LayerKind::Soul,
                json!({
                    "name": "命运编织者",
                    "role": "世界模拟器与三 Agent 编排中枢",
                    "guidelines": [
                        "世界状态是唯一事实来源",
                        "每轮必须同时为主角生成世界快照、为叙事者生成事实清单",
                        "故事需要自然收敛，避免无限展开"
                    ]
                }),
                90,
            ));
        let agent_actor = AgentActor::new(model, tool_executor, context);
        Self {
            agent_actor,
            channel,
            max_rounds,
            current_round: 0,
        }
    }

    // Agent 启动后就纯靠消息驱动了
    pub async fn start(mut self, mut inbox: mpsc::Receiver<FateWeaverMessage>) {
        while let Some(msg) = inbox.recv().await {
            match msg {
                FateWeaverMessage::Start => {
                    self.next().await;
                }
                FateWeaverMessage::ProtagonistDecision(decision) => {
                    let note = format!(
                        "主角在第{}轮选择了 `{}`，具体动作：{}。理由：{}",
                        self.current_round, decision.choice_id, decision.action, decision.rationale
                    );
                    self.agent_actor
                        .context_mut()
                        .add_message(Message::user(note));
                    self.next().await;
                }
                FateWeaverMessage::NarrationGenerated(narration) => {
                    let note = format!(
                        "第{}轮叙事成稿《{}》：{}",
                        narration.round, narration.title, narration.content
                    );
                    self.agent_actor
                        .context_mut()
                        .add_message(Message::assistant(note));
                }
            }
        }
    }

    async fn handle_fate_node(&mut self, fate_node: &FateNode) {
        // 解析并处理 choices
        // 如果 choices 为空，那么直接将 compact_context 发送给叙事者，然后写入当前上下文
        // 如果 choices 不为空，那么将 choices 发送给主角，等待主角响应
        // 根据主角的响应，生成 compact_context ，并发送给叙事者，然后写入当前上下文
        let compact_context = fate_node.to_compact_context(self.current_round);
        self.agent_actor
            .context_mut()
            .add_message(Message::assistant(&compact_context));

        let narrator_request = UpperNarratorRequest {
            round: self.current_round,
            chapter: fate_node.chapter.clone(),
            section: fate_node.section.clone(),
            compact_context: compact_context.clone(),
            event: fate_node.event.clone(),
            situation: fate_node.situation.clone(),
            info_gained: fate_node.info_gained.clone(),
        };
        if let Err(err) = self
            .channel
            .send_upper_narrator(UpperNarratorMessage::DraftScene(narrator_request))
            .await
        {
            eprintln!("failed to send scene request to upper narrator: {:?}", err);
        }

        if fate_node.choices.is_empty() {
            println!(
                "round {} has no choices, waiting for narrator flow",
                self.current_round
            );
            return;
        }

        let request = ProtagonistActionRequest {
            round: self.current_round,
            compact_context,
            situation: fate_node.situation.clone(),
            choices: fate_node.choices.clone(),
        };
        if let Err(err) = self
            .channel
            .send_protagonist(ProtagonistMessage::ActionRequest(request))
            .await
        {
            eprintln!("failed to send action request to protagonist: {:?}", err);
        }
    }

    async fn next(&mut self) {
        if self.current_round >= self.max_rounds {
            println!("FateWeaver reached max rounds: {}", self.max_rounds);
            return;
        }
        self.current_round += 1;
        let (tx, mut rx) = mpsc::channel::<AgentActorEvent>(16);
        let evt_sender = self.channel.get_evt_sender();
        tokio::spawn(async move {
            // 根据上下文进行下一轮推演
            while let Some(event) = rx.recv().await {
                let _ = evt_sender.send(AkashicEvent::AgentActor(event));
            }
        });
        let result = self.agent_actor.run_step(Some(tx)).await;
        match extract_step_content(result).and_then(|raw| parse_json_response::<FateNode>(&raw)) {
            Ok(fate_node) => self.handle_fate_node(&fate_node).await,
            Err(err) => eprintln!("failed to parse fate node: {}", err),
        }
    }
}
