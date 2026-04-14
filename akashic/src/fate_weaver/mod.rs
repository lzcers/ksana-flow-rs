mod prompt;
pub mod types;
use agent::{
    agent::{AgentActor, AgentActorEvent, Context, GenericToolExecutor, LayerKind},
    models::ChatModel,
};
use serde_json::json;
use prompt::SYS_PROMPT;
use tokio::sync::mpsc;

use crate::{ channel::{AgentChannel, AkashicEvent, FateWeaverMessage}, fate_weaver::types::FateNode, shared::{build_chat_model, build_layer}};


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
        let system_prompt = SYS_PROMPT.replace("{world_profile}", &world_profile).replace("{protagonist_profile}", &prota_profile);
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
            ))
            .layer(build_layer(
                "story-setup",
                LayerKind::Memory,
                json!({
                    "world_profile": world_profile,
                    "protagonist_profile": prota_profile,
                    "max_rounds": max_rounds
                }),
                80,
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
                    self.current_round += 1;
                    self.next().await;
                }
            }
        }
    }



    fn handle_fate_node(&mut self, _fate_node: &FateNode) {
        // todo:  解析并处理 choices
        // 如果 choices 为空，那么直接将 compact_context 发送给叙事者，然后写入当前上下文
        // 如果 choices 不为空，那么将 choices 发送给主角，等待主角响应
        // 根据主角的响应，生成 compact_context ，并发送给叙事者，然后写入当前上下文
        todo!()
    }

    async fn next(&mut self) {
        let (tx, mut rx) = mpsc::channel::<AgentActorEvent>(16);
        let evt_sender = self.channel.get_evt_sender();
        tokio::spawn(async move {
            // 根据上下文进行下一轮推演
            while let Some(event) = rx.recv().await {
                let _ = evt_sender.send(AkashicEvent::AgentActor(event));
            }
        });
        self.agent_actor.run_step(Some(tx)).await;
    }


    




}
