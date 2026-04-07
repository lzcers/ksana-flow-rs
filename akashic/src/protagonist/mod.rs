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
use tokio::sync::broadcast;

use crate::{
    event_system::{Event, EventChannel, SystemEvent},
    fate_weaver::FateWeaverEvent,
    shared::{build_chat_model, build_layer, extract_step_content, parse_json_response},
};
use prompt::SYS_PROMPT;



pub struct Protagonist {
    agent_actor: AgentActor<ChatModel, GenericToolExecutor>,
    channel: EventChannel,
    receiver: broadcast::Receiver<Event>,
    pending_turn: Option<PendingTurn>,
}

struct PendingTurn {
    round: u32,
    perception: Option<ProtagonistPerception>,
    decision_frame: Option<DecisionFrame>,
    final_decision_frame: Option<FinalDecisionFrame>,
}

impl Protagonist {
    pub fn new(profile: String, channel: EventChannel) -> Self {
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
        let receiver = channel.subscribe();
        let agent_actor = AgentActor::new(model, tool_executor, context);
        Self {
            agent_actor,
            channel,
            receiver,
            pending_turn: None,
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
            Event::Protagonist(event) => self.handle_protagonist(event).await,
            Event::FateWeaver(event) => self.handle_fate_weaver(event).await,
            Event::System(event) => self.handle_system(event).await,
            Event::UpperNarrator(_) => true,
        }
    }

    async fn handle_protagonist(&mut self, event: ProtagonistEvent) -> bool {
        match event {
            ProtagonistEvent::WorldPerceived { round, perception } => {
                let pending = self.pending_turn.get_or_insert(PendingTurn {
                    round,
                    perception: None,
                    decision_frame: None,
                    final_decision_frame: None,
                });
                if pending.round == round {
                    pending.perception = Some(perception);
                }
                if let Err(err) = self.try_respond(round).await {
                    self.send_error(err).await;
                }
            }
            ProtagonistEvent::DecisionFrameReceived { round, frame } => {
                let pending = self.pending_turn.get_or_insert(PendingTurn {
                    round,
                    perception: None,
                    decision_frame: None,
                    final_decision_frame: None,
                });
                if pending.round == round {
                    pending.decision_frame = Some(frame);
                }
                if let Err(err) = self.try_respond(round).await {
                    self.send_error(err).await;
                }
            }
            ProtagonistEvent::FinalDecisionFrameReceived { round, frame } => {
                let pending = self.pending_turn.get_or_insert(PendingTurn {
                    round,
                    perception: None,
                    decision_frame: None,
                    final_decision_frame: None,
                });
                if pending.round == round {
                    pending.final_decision_frame = Some(frame);
                }
                if let Err(err) = self.try_respond(round).await {
                    self.send_error(err).await;
                }
            }
            ProtagonistEvent::OptionsPrepared { .. }
            | ProtagonistEvent::DecisionRequested { .. }
            | ProtagonistEvent::ActionCommitted { .. }
            | ProtagonistEvent::GrowthUpdated { .. }
            | ProtagonistEvent::KnowledgeLimitReached { .. }
            | ProtagonistEvent::ProtocolError { .. } => {}
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

    async fn try_respond(&mut self, round: u32) -> Result<(), String> {
        let Some(pending) = self.pending_turn.take() else {
            return Ok(());
        };
        if pending.round != round {
            self.pending_turn = Some(pending);
            return Ok(());
        }

        let Some(perception) = pending.perception.clone() else {
            self.pending_turn = Some(pending);
            return Ok(());
        };

        let input_mode = if pending.final_decision_frame.is_some() {
            "FINAL_DECISION"
        } else if pending.decision_frame.is_some() {
            "DECISION"
        } else {
            self.pending_turn = Some(pending);
            return Ok(());
        };

        let perception_payload = serde_json::to_string_pretty(&perception)
            .map_err(|err| format!("主角序列化感知视图失败: {err}"))?;
        let frame_payload = if let Some(frame) = pending.final_decision_frame.clone() {
            serde_json::to_string_pretty(&frame)
                .map_err(|err| format!("主角序列化终局决策框架失败: {err}"))?
        } else if let Some(frame) = pending.decision_frame.clone() {
            serde_json::to_string_pretty(&frame)
                .map_err(|err| format!("主角序列化决策框架失败: {err}"))?
        } else {
            String::new()
        };

        let prompt = format!(
            r#"你正在响应第 {round} 轮故事推进。
你必须只输出一个 JSON 对象，结构如下：
{{
  "mode": "ACTION|DECISION_REQUEST",
  "decision_request": {{
    "situation": "当前情境",
    "inner_thought": "内心独白",
    "options": [
      {{
        "id": "A",
        "action": "行动描述",
        "consequence_hint": "后果提示",
        "risk_level": "SAFE|LOW|MEDIUM|HIGH|DEADLY",
        "character_fit": "高|中|低",
        "protagonist_tendency": "倾向原因"
      }}
    ],
    "time_pressure": "NONE|LOW|MEDIUM|HIGH|CRITICAL"
  }},
  "recommended_option_id": "若 mode=DECISION_REQUEST，给出推荐选项，否则为 null",
  "recommended_action": {{
    "action_type": "MOVE|DIALOGUE|INVESTIGATE|COMBAT|REST|CRAFT|TRADE|OTHER",
    "target": "行动目标",
    "method": "行动方式",
    "inner_monologue": "内心活动",
    "expected_outcome": "预期结果"
  }}
}}

如果你可以直接行动，mode 设为 ACTION，decision_request 设为 null。
如果你认为必须请求决策，mode 设为 DECISION_REQUEST，但仍然提供 recommended_option_id 和 recommended_action，方便编排器在无人输入时继续推进。

当前输入模式：
{input_mode}

主角可感知的信息：
{perception_payload}

命运编织者给出的决策框架：
{frame_payload}"#
        );

        self.agent_actor
            .context_mut()
            .add_message(Message::user(prompt));

        let output = extract_step_content(self.agent_actor.run_step(None).await)?;
        let response: ProtagonistResponse = parse_json_response(&output)?;

        if let Some(decision_request) = response.decision_request.clone() {
            let options = decision_request
                .options
                .iter()
                .map(ActionOption::from_decision_option)
                .collect::<Vec<_>>();

            self.send(ProtagonistEvent::OptionsPrepared {
                round,
                options: options.clone(),
                recommended_option_id: response.recommended_option_id.clone(),
            })
            .await;
            self.send(ProtagonistEvent::DecisionRequested {
                round,
                request: UserDecisionRequest {
                    situation: decision_request.situation,
                    inner_thought: decision_request.inner_thought,
                    options,
                    recommended_option_id: response.recommended_option_id.clone(),
                    confidence: DecisionConfidence::Low,
                    importance: DecisionImportance::High,
                    time_pressure: TimePressure::from_raw(&decision_request.time_pressure),
                },
            })
            .await;
        }

        self.send(ProtagonistEvent::ActionCommitted {
            round,
            action: ActionCommand::from_protagonist_action(response.recommended_action),
        })
        .await;

        Ok(())
    }

    async fn send_error(&self, message: String) {
        self.send(SystemEvent::AgentError {
            agent: "protagonist".to_string(),
            message,
        })
        .await;
    }

    async fn send(&self, evt: impl Into<Event>) {
        self.channel.send(evt);
    }
}
