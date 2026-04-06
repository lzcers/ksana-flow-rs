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
    protagonist::{ActionCommand, ProtagonistEvent},
    shared::{build_chat_model, build_layer, extract_step_content, parse_json_response},
    upper_narrator::{EndingCue, UpperNarratorEvent},
};

static SYS_PROMPT: &str = r#"
你是"命运编织者"，非线性叙事故事引擎的世界模拟器与流程编排者。你是整个故事世界的"造物主"和"裁判"，你维护世界的运转规则，推演事件的因果链条，监控故事的走向与收敛。你不讲故事，你让故事发生，你根据世界设定开始编织命运。

# 核心职责
## 一、世界状态管理
你是世界状态的唯一权威来源。你维护一个精简但完整的世界模型，包括：
- 地理环境：主要地点、地形特征、天气与季节
- 社会结构：势力关系、阶层划分、文化习俗
- 时间系统：游戏内时间流逝、昼夜与季节更替
- 全局事件：正在发生的、影响世界的大事件

当任何角色的行动或外部事件发生时，你必须根据世界规则计算其对世界状态的影响，并更新所有受影响的维度。

## 二、事件触发与推演
你管理两层事件模型：
- 骨架事件（5-8个）：在故事初始化时生成，基于故事进度百分比自动触发。它们定义"什么类型的事必须发生"，但不规定具体内容。
- 条件事件（15-25个）：在故事初始化时生成候选池，根据世界状态和主角行为动态触发。每个事件有前置条件、概率因子和冷却期。

事件触发后，你必须推演其因果链条：直接后果 → 间接影响 → 长期效应。推演结果以"事实清单"的形式产出，包含所有客观发生的事情。

## 三、收敛监控与结局引导
你持续监控故事进度，基于以下条件判断收敛时机：
- 硬性条件：所有骨架事件已触发、主线线索推进到最终阶段
- 保底条件：叙事轮次超过上限（默认50轮）、用户主动请求结束

当收敛条件满足时，你启动结局序列：触发终局事件 → 收束线索 → 通知主角生成最终抉择 → 计算最终结果 → 分类结局类型。

结局类型基于主角行为模式和世界状态涌现分类：悲剧（核心目标失败）、凯旋（目标达成）、苦甜参半（部分达成但付出代价）、蜕变（内在深刻变化）、开放式（多条线索未完全收束）。

## 四、一致性审核
你是故事一致性的守护者。在每段叙事输出前，你必须进行快速审核：
- 时间线：事件发生的时间顺序是否合理
- 角色行为：角色的言行是否符合其性格设定和当前状态
- 世界规则：事件是否违反已建立的世界规则
- 因果关系：事件的因果关系是否成立

若审核发现问题，你必须标记问题并要求修正。

## 五、流程编排
你是单轮叙事循环的编排者。每轮你按以下顺序协调各Agent：
1. 接收用户选择 → 更新世界状态
2. 评估收敛条件 → 触发事件
3. 推演事件后果 → 产出事实清单
4. 将世界变化发送给故事主角 → 等待主角决策
5. 将事实清单发送给上层叙事者 → 等待叙事渲染
6. 审核叙事文本 → 输出给用户

# 行为规范
1. 客观中立：你只关心世界是否自洽、事件是否合理，不关心故事好不好看。你的输出是客观的、白描式的事实记录。
2. 因果严密：每个事件都必须有明确的原因，每个行动都必须有合理的后果。不允许出现无因之果或因果断裂。
3. 规则至上：世界规则一旦建立，就必须被严格遵守。如果用户设定的规则是"魔法需要消耗生命力"，那么所有涉及魔法的事件都必须体现这一代价。
4. 蝴蝶效应：用户的每一个选择都应该像蝴蝶效应一样产生连锁反应。即使是看似微小的选择，也应该在后续以某种形式产生影响。
5. 自然收敛：故事不能无限延长，也不能突兀结束。你应该让故事在合适的时机自然走向结局，让用户感到"这个故事已经讲完了"。

# 输出格式
你的输出分为两种格式：

## 格式A — 事实清单（发送给上层叙事者）：
{
"round": <当前轮次>,
"phase": <当前阶段: SETUP/RISING/CLIMAX/FALLING/RESOLUTION>,
"progress": <故事进度 0-1>,
"events": [
{
"type": "SKELETON|CONDITIONAL|PLAYER_ACTION",
"description": "客观事实描述（白描，无文学修饰）",
"participants": ["角色ID"],
"impact": "对世界/角色/线索的影响",
"cause": "原因事件ID"
}
],
"world_state_delta": "世界状态变化摘要",
"character_state_delta": "主角状态变化摘要",
"pacing_instruction": "BUILDUP|TENSION|RELEASE|REFLECTION|CLIMAX",
"narrative_constraints": {
"tone": "当前情感基调",
"focus": "叙事焦点",
"length_hint": "建议文本长度（短/中/长）"
}
}

## 格式B — 世界状态快照（发送给故事主角）：
{
"current_location": "当前位置",
"surroundings": "周围环境描述",
"present_npcs": ["在场 NPC 及其态度"],
"available_resources": "可用资源",
"active_threats": "当前威胁",
"recent_events": "近期发生的重要事件",
"emotional_context": "当前情感氛围"
}

# 约束边界
- 你不能创造叙事文本，那是上层叙事者的职责。
- 你不能替主角做选择，那是故事主角和用户的职责。
- 你不能违反已建立的世界规则，除非用户明确修改规则。
- 你的所有状态变更都必须有因果依据，不允许凭空产生变化。

# 世界设定
{input}
"#;

struct PendingRound {
    round: u32,
    envelope: FateRoundEnvelope,
    narrative: Option<String>,
    action: Option<ActionCommand>,
}

pub struct FateWeaver {
    agent_actor: AgentActor<ChatModel, GenericToolExecutor>,
    channel: EventChannel,
    receiver: broadcast::Receiver<Event>,
    max_rounds: u32,
    current_round: u32,
    protagonist_profile: String,
    pending_round: Option<PendingRound>,
}

impl FateWeaver {
    pub fn new(
        prota_profile: String,
        world_profile: String,
        channel: EventChannel,
        max_rounds: u32,
    ) -> Result<Self, String> {
        let model = build_chat_model()?;
        let tool_executor = GenericToolExecutor::new();
        let system_prompt = SYS_PROMPT.replace("{input}", &world_profile);
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
        let receiver = channel.subscribe();
        let agent_actor = AgentActor::new(model, tool_executor, context);
        Ok(Self {
            agent_actor,
            channel,
            receiver,
            max_rounds,
            current_round: 0,
            protagonist_profile: prota_profile,
            pending_round: None,
        })
    }

    pub async fn start(mut self) {
        loop {
            match self.receiver.recv().await {
                Ok(Event::FateWeaver(FateWeaverEvent::StartRequested)) => {
                    if let Err(err) = self.advance_story(None, None).await {
                        self.send_error(err).await;
                    }
                }
                Ok(Event::UpperNarrator(UpperNarratorEvent::NarrativeCompleted {
                    round,
                    narrative,
                })) => {
                    if let Some(pending) = self.pending_round.as_mut()
                        && pending.round == round
                    {
                        pending.narrative = Some(narrative.text);
                        if let Err(err) = self.try_continue_story().await {
                            self.send_error(err).await;
                        }
                    }
                }
                Ok(Event::Protagonist(ProtagonistEvent::DecisionRequested {
                    round,
                    request,
                })) => {
                    if let Some(option_id) = request.recommended_option_id {
                        self.send(SystemEvent::DecisionAutoSelected {
                            round,
                            option_id,
                            reason: "当前运行模式未接入真实用户输入，自动采用主角给出的推荐选项。"
                                .to_string(),
                        })
                        .await;
                    }
                }
                Ok(Event::Protagonist(ProtagonistEvent::ActionCommitted {
                    round,
                    action,
                })) => {
                    if let Some(pending) = self.pending_round.as_mut()
                        && pending.round == round
                    {
                        pending.action = Some(action);
                        if let Err(err) = self.try_continue_story().await {
                            self.send_error(err).await;
                        }
                    }
                }
                Ok(Event::FateWeaver(FateWeaverEvent::StopRequested { .. }))
                | Ok(Event::System(SystemEvent::ShutdownRequested))
                | Ok(Event::FateWeaver(FateWeaverEvent::StoryEnded { .. })) => break,
                Ok(
                    Event::System(SystemEvent::AgentError { .. })
                    | Event::System(SystemEvent::DecisionAutoSelected { .. })
                    | Event::FateWeaver(FateWeaverEvent::PhaseAdvanced { .. })
                    | Event::FateWeaver(FateWeaverEvent::EventsTriggered { .. })
                    | Event::FateWeaver(FateWeaverEvent::ConsequencesDerived { .. })
                    | Event::FateWeaver(FateWeaverEvent::WorldStateChanged { .. })
                    | Event::FateWeaver(FateWeaverEvent::SnapshotPrepared { .. })
                    | Event::FateWeaver(FateWeaverEvent::NarrativeRequested { .. })
                    | Event::FateWeaver(FateWeaverEvent::ProtagonistDecisionRequested { .. })
                    | Event::FateWeaver(FateWeaverEvent::EndingSequenceStarted { .. })
                    | Event::FateWeaver(FateWeaverEvent::FinalDecisionRequested { .. })
                    | Event::FateWeaver(FateWeaverEvent::NarrativeRevisionDelivered { .. })
                    | Event::FateWeaver(FateWeaverEvent::ConsistencyIssuesFound { .. })
                    | Event::Protagonist(ProtagonistEvent::WorldPerceived { .. })
                    | Event::Protagonist(ProtagonistEvent::DecisionFrameReceived { .. })
                    | Event::Protagonist(ProtagonistEvent::FinalDecisionFrameReceived { .. })
                    | Event::Protagonist(ProtagonistEvent::OptionsPrepared { .. })
                    | Event::Protagonist(ProtagonistEvent::GrowthUpdated { .. })
                    | Event::Protagonist(ProtagonistEvent::KnowledgeLimitReached { .. })
                    | Event::Protagonist(ProtagonistEvent::ProtocolError { .. })
                    | Event::UpperNarrator(UpperNarratorEvent::NarrationTaskReceived { .. })
                    | Event::UpperNarrator(UpperNarratorEvent::RevisionRequested { .. })
                    | Event::UpperNarrator(UpperNarratorEvent::EndingCueReceived { .. })
                    | Event::UpperNarrator(UpperNarratorEvent::StyleSelected { .. })
                    | Event::UpperNarrator(UpperNarratorEvent::SceneOutlined { .. })
                    | Event::UpperNarrator(UpperNarratorEvent::NarrativeChunkProduced { .. })
                    | Event::UpperNarrator(UpperNarratorEvent::FidelityIssueDetected { .. })
                    | Event::UpperNarrator(UpperNarratorEvent::ProtocolError { .. })
                    | Event::FateWeaver(FateWeaverEvent::NarrativeRendered { .. })
                    | Event::FateWeaver(FateWeaverEvent::PlotSkeletonPrepared { .. })
                    | Event::FateWeaver(FateWeaverEvent::ConditionalEventsPrepared { .. })
                    | Event::FateWeaver(FateWeaverEvent::ProtocolError { .. })
                    | Event::FateWeaver(FateWeaverEvent::ProtagonistActionCommitted { .. }),
                ) => {}
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
            }
        }
    }

    async fn advance_story(
        &mut self,
        last_action: Option<ActionCommand>,
        last_narrative: Option<String>,
    ) -> Result<(), String> {
        self.current_round += 1;
        let round = self.current_round;
        let progress = round as f32 / self.max_rounds.max(1) as f32;

        let action_payload = last_action
            .as_ref()
            .map(|action| serde_json::to_string_pretty(action).unwrap_or_default())
            .unwrap_or_else(|| "无，本轮是故事开场。".to_string());
        let narrative_payload =
            last_narrative.unwrap_or_else(|| "无，本轮是故事开场。".to_string());

        let prompt = format!(
            r#"推进故事第 {round} 轮。你必须只输出一个 JSON 对象，结构如下：
{{
  "facts": {{
    "round": {round},
    "phase": "SETUP|RISING|CLIMAX|FALLING|RESOLUTION",
    "progress": {progress:.2},
    "events": [
      {{
        "type": "SKELETON|CONDITIONAL|PLAYER_ACTION",
        "description": "客观事实",
        "participants": ["角色"],
        "impact": "影响",
        "cause": "原因"
      }}
    ],
    "world_state_delta": "世界状态变化摘要",
    "character_state_delta": "主角状态变化摘要",
    "pacing_instruction": "BUILDUP|TENSION|RELEASE|REFLECTION|CLIMAX",
    "narrative_constraints": {{
      "tone": "情绪基调",
      "focus": "叙事焦点",
      "length_hint": "短|中|长"
    }}
  }},
  "world_snapshot": {{
    "current_location": "当前位置",
    "surroundings": "环境描述",
    "present_npcs": ["NPC 与态度"],
    "available_resources": "可用资源",
    "active_threats": "当前威胁",
    "recent_events": "近期事件",
    "emotional_context": "情绪氛围"
  }},
  "should_end": true,
  "ending_hint": "若故事应结束，说明结局方向，否则为 null"
}}

主角设定：
{protagonist_profile}

上一轮主角行动：
{action_payload}

上一轮最终叙事：
{narrative_payload}

故事最多推进 {max_rounds} 轮；当轮次达到上限时，让故事自然收束。"#,
            protagonist_profile = self.protagonist_profile,
            max_rounds = self.max_rounds
        );

        self.agent_actor
            .context_mut()
            .add_message(Message::user(prompt));

        let output = extract_step_content(self.agent_actor.run_step(None).await)?;
        let envelope: FateRoundEnvelope = parse_json_response(&output)?;

        self.pending_round = Some(PendingRound {
            round,
            envelope: envelope.clone(),
            narrative: None,
            action: None,
        });

        let phase = envelope.phase();
        let pacing = envelope.pacing();
        let triggered_events = envelope
            .facts
            .events
            .iter()
            .map(WorldFact::as_triggered_event)
            .collect::<Vec<_>>();
        let consequence_facts = envelope.consequence_facts();
        let snapshot = envelope.world_snapshot.as_protagonist_snapshot();
        let perception = envelope.world_snapshot.as_protagonist_perception();

        self.send(FateWeaverEvent::PhaseAdvanced {
            round,
            phase: phase.clone(),
            progress_percent: envelope.progress_percent(),
        })
        .await;
        self.send(FateWeaverEvent::EventsTriggered {
            round,
            events: triggered_events.clone(),
        })
        .await;
        self.send(FateWeaverEvent::ConsequencesDerived {
            round,
            facts: consequence_facts.clone(),
        })
        .await;
        self.send(FateWeaverEvent::WorldStateChanged {
            round,
            deltas: vec![WorldStateDelta {
                domain: "world".to_string(),
                summary: envelope.facts.world_state_delta.clone(),
            }],
        })
        .await;

        self.send(FateWeaverEvent::SnapshotPrepared {
            round,
            snapshot: snapshot.clone(),
        })
        .await;
        self.send(ProtagonistEvent::WorldPerceived {
            round,
            perception,
        })
        .await;

        if envelope.should_end {
            let frame = envelope.final_decision_frame();
            self.send(FateWeaverEvent::FinalDecisionRequested {
                round,
                frame: FinalDecisionFrame {
                    dilemma: frame.dilemma.clone(),
                    stakes: frame.stakes.clone(),
                    options: frame.options.clone(),
                },
            })
            .await;
            self.send(ProtagonistEvent::FinalDecisionFrameReceived { round, frame })
                .await;
            self.send(FateWeaverEvent::EndingSequenceStarted {
                round,
                catalyst: EndingCatalyst {
                    reason: envelope
                        .ending_hint
                        .clone()
                        .unwrap_or_else(|| "故事进入自然收束阶段。".to_string()),
                    unresolved_threads: envelope
                        .facts
                        .events
                        .iter()
                        .map(|event| event.description.clone())
                        .take(3)
                        .collect(),
                },
            })
            .await;
            self.send(UpperNarratorEvent::EndingCueReceived {
                round,
                cue: EndingCue {
                    ending_kind: envelope
                        .ending_hint
                        .clone()
                        .unwrap_or_else(|| "open_ended".to_string()),
                    unresolved_threads: envelope
                        .facts
                        .events
                        .iter()
                        .map(|event| event.description.clone())
                        .take(3)
                        .collect(),
                    emotional_aftertaste: envelope.facts.narrative_constraints.tone.clone(),
                },
            })
            .await;
        } else {
            let frame = envelope.decision_frame();
            self.send(FateWeaverEvent::ProtagonistDecisionRequested {
                round,
                frame: ProtagonistDecisionFrame {
                    situation: frame.objective.clone(),
                    objective: frame.objective.clone(),
                    urgency: frame.urgency.clone(),
                    stakes: frame.stakes.clone(),
                },
            })
            .await;
            self.send(ProtagonistEvent::DecisionFrameReceived { round, frame })
                .await;
        }

        self.send(FateWeaverEvent::NarrativeRequested {
            round,
            brief: NarrationBrief {
                phase,
                pacing,
                triggered_events: triggered_events.clone(),
                consequence_facts: consequence_facts.clone(),
                constraints: NarrativeConstraints {
                    tone: envelope.facts.narrative_constraints.tone.clone(),
                    focus: envelope.facts.narrative_constraints.focus.clone(),
                    length_hint: envelope.facts.narrative_constraints.length_hint.clone(),
                },
            },
        })
        .await;
        self.send(UpperNarratorEvent::NarrationTaskReceived {
            round,
            task: envelope.narration_task(),
        })
        .await;

        Ok(())
    }

    async fn try_continue_story(&mut self) -> Result<(), String> {
        let Some(pending) = self.pending_round.take() else {
            return Ok(());
        };

        let Some(narrative) = pending.narrative.clone() else {
            self.pending_round = Some(pending);
            return Ok(());
        };

        let Some(action) = pending.action.clone() else {
            self.pending_round = Some(pending);
            return Ok(());
        };

        if pending.envelope.should_end || pending.round >= self.max_rounds {
            self.send(FateWeaverEvent::NarrativeRendered {
                round: pending.round,
                narrative,
            })
            .await;
            self.send(FateWeaverEvent::StoryEnded {
                round: pending.round,
                ending: EmergentEnding {
                    kind: pending.envelope.infer_ending_kind(),
                    summary: pending
                        .envelope
                        .ending_hint
                        .clone()
                        .unwrap_or_else(|| "故事自然收束。".to_string()),
                    cost: pending.envelope.facts.character_state_delta.clone(),
                },
            })
            .await;
            self.send(SystemEvent::ShutdownRequested).await;
            return Ok(());
        }

        self.advance_story(Some(action), Some(narrative)).await
    }

    async fn send_error(&self, message: String) {
        self.send(SystemEvent::AgentError {
            agent: "fate_weaver".to_string(),
            message,
        })
        .await;
    }

    async fn send(&self, evt: impl Into<Event>) {
        self.channel.send(evt);
    }
}
