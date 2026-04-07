mod prompt;
pub mod types;
#[allow(unused_imports)]
pub use types::*;

use agent::{
    agent::{AgentActor, AgentActorEvent, Context, GenericToolExecutor, LayerKind},
    core::Message,
    models::ChatModel,
};
use serde_json::json;
use tokio::sync::{broadcast, mpsc};
use prompt::SYS_PORMPT;
use crate::{
    event_system::{Event, EventChannel, SystemEvent},
    fate_weaver::FateWeaverEvent,
    shared::{build_chat_model, build_layer, extract_step_content},
};


pub struct UpperNarrator {
    agent_actor: AgentActor<ChatModel, GenericToolExecutor>,
    channel: EventChannel,
    receiver: broadcast::Receiver<Event>,
    pending_ending_cue: Option<(u32, EndingCue)>,
}

impl UpperNarrator {
    pub fn new(channel: EventChannel) -> Self { 
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
        let receiver = channel.subscribe();
        let agent_actor = AgentActor::new(model, tool_executor, context);
        Self {
            agent_actor,
            channel,
            receiver,
            pending_ending_cue: None,
        }
    }

    pub async fn start(mut self) {
        loop {
            match self.receiver.recv().await {
                Ok(event) => {
                    if !self.handle_event(event).await {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    }

    async fn handle_event(&mut self, event: Event) -> bool {
        match event {
            Event::UpperNarrator(event) => self.handle_upper_narrator(event).await,
            Event::FateWeaver(event) => self.handle_fate_weaver(event).await,
            Event::System(event) => self.handle_system(event).await,
            Event::Protagonist(_) => true,
        }
    }

    async fn handle_upper_narrator(&mut self, event: UpperNarratorEvent) -> bool {
        match event {
            UpperNarratorEvent::EndingCueReceived { round, cue } => {
                self.pending_ending_cue = Some((round, cue));
            }
            UpperNarratorEvent::NarrationTaskReceived { round, task } => {
                let ending_cue = self
                    .pending_ending_cue
                    .take()
                    .filter(|(cue_round, _)| *cue_round == round)
                    .map(|(_, cue)| cue);
                if let Err(err) = self.render_round(round, task, ending_cue).await {
                    self.send_error(err).await;
                }
            }
            UpperNarratorEvent::StyleSelected { .. }
            | UpperNarratorEvent::SceneOutlined { .. }
            | UpperNarratorEvent::NarrativeChunkProduced { .. }
            | UpperNarratorEvent::NarrativeCompleted { .. }
            | UpperNarratorEvent::RevisionRequested { .. }
            | UpperNarratorEvent::FidelityIssueDetected { .. }
            | UpperNarratorEvent::ProtocolError { .. } => {}
        }

        true
    }

    async fn handle_fate_weaver(&mut self, event: FateWeaverEvent) -> bool {
        match event {
            FateWeaverEvent::StoryEnded { .. } => false,
            FateWeaverEvent::StartRequested
            | FateWeaverEvent::StopRequested { .. }
            | FateWeaverEvent::PhaseAdvanced { .. }
            | FateWeaverEvent::EventsTriggered { .. }
            | FateWeaverEvent::ConsequencesDerived { .. }
            | FateWeaverEvent::WorldStateChanged { .. }
            | FateWeaverEvent::SnapshotPrepared { .. }
            | FateWeaverEvent::NarrativeRequested { .. }
            | FateWeaverEvent::ProtagonistDecisionRequested { .. }
            | FateWeaverEvent::EndingSequenceStarted { .. }
            | FateWeaverEvent::FinalDecisionRequested { .. }
            | FateWeaverEvent::NarrativeRevisionDelivered { .. }
            | FateWeaverEvent::ConsistencyIssuesFound { .. }
            | FateWeaverEvent::NarrativeRendered { .. }
            | FateWeaverEvent::PlotSkeletonPrepared { .. }
            | FateWeaverEvent::ConditionalEventsPrepared { .. }
            | FateWeaverEvent::ProtocolError { .. }
            | FateWeaverEvent::ProtagonistActionCommitted { .. } => true,
        }
    }

    async fn handle_system(&mut self, event: SystemEvent) -> bool {
        match event {
            SystemEvent::ShutdownRequested => false,
            SystemEvent::AgentError { .. } | SystemEvent::DecisionAutoSelected { .. } => true,
        }
    }

    async fn render_round(
        &mut self,
        round: u32,
        task: NarrationTask,
        ending_cue: Option<EndingCue>,
    ) -> Result<(), String> {
        let style = NarrativeStyle::for_task(&task);
        let perspective = NarrativePerspective::reader_default();
        let scene_beats = task.scene_beats();
        let facts = serde_json::to_string_pretty(&task)
            .map_err(|err| format!("叙事者序列化事实清单失败: {err}"))?;
        let ending_payload = serde_json::to_string_pretty(&ending_cue)
            .map_err(|err| format!("叙事者序列化结局提示失败: {err}"))?;

        self.send(UpperNarratorEvent::StyleSelected {
            round,
            style: style.clone(),
            perspective: perspective.clone(),
        })
        .await;
        self.send(UpperNarratorEvent::SceneOutlined {
            round,
            beats: scene_beats.clone(),
        })
        .await;

        let prompt = format!(
            r#"你正在书写第 {round} 轮故事。
你必须只输出叙事正文本身，不要 JSON，不要标题，不要解释，不要附加说明。

叙事要求：
- 忠于 facts 中的客观事实，不新增关键结果
- 根据 pacing_instruction、tone、focus、length_hint 调整节奏与文风
- 直接从正文第一句开始写，适合被逐字流式展示
- 如果 should_end=true，要让文字具备结局收束感
- 当前建议风格：{style_name}
- 当前叙事视角：{perspective_name}

事实清单：
{facts}

场景节拍：
{scene_beats_payload}

结局提示：
{ending_payload}"#,
            style_name = style.label(),
            perspective_name = perspective.label(),
            scene_beats_payload = serde_json::to_string_pretty(&scene_beats)
                .map_err(|err| format!("叙事者序列化场景节拍失败: {err}"))?
        );

        self.agent_actor
            .context_mut()
            .add_message(Message::user(prompt));

        let channel = self.channel.clone();
        let narrative = {
            let (event_tx, mut event_rx) = mpsc::channel(128);
            let step = self.agent_actor.run_step(Some(event_tx));
            tokio::pin!(step);

            let result = loop {
                tokio::select! {
                    result = &mut step => break result,
                    maybe_event = event_rx.recv() => {
                        match maybe_event {
                            Some(AgentActorEvent::ContentChunk(content)) => {
                                channel.send(UpperNarratorEvent::NarrativeChunkProduced {
                                    round,
                                    content,
                                });
                            }
                            Some(_) => {}
                            None => {}
                        }
                    }
                }
            };

            while let Some(event) = event_rx.recv().await {
                if let AgentActorEvent::ContentChunk(content) = event {
                    channel.send(UpperNarratorEvent::NarrativeChunkProduced {
                        round,
                        content,
                    });
                }
            }

            extract_step_content(result)?
        };

        channel.send(UpperNarratorEvent::NarrativeCompleted {
            round,
            narrative: RenderedNarrative {
                text: narrative,
                style,
                perspective,
            },
        });

        Ok(())
    }

    async fn send_error(&self, message: String) {
        self.send(SystemEvent::AgentError {
            agent: "upper_narrator".to_string(),
            message,
        })
        .await;
    }

    async fn send(&self, evt: impl Into<Event>) {
        self.channel.send(evt);
    }
}
