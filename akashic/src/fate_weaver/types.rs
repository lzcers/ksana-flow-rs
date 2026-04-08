use crate::{
    protagonist::{
        DecisionFrame, FinalDecisionFrame as ProtagonistFinalDecisionFrame, ProtagonistPerception,
    },
    upper_narrator::{FactFragment, NarrationTask},
};
#[allow(unused_imports)]
pub use payload::{
    ConsequenceFact, EmergentEnding, EndingCatalyst, FateWeaverEndingKind, FateWeaverEvent,
    FateWeaverEventKind, FateWeaverPacing, FateWeaverPhase, FinalDecisionFrame, NarrationBrief,
    NarrativeConstraints, ProtagonistDecisionFrame, ProtagonistWorldSnapshot,
    TriggeredWorldEvent, WorldStateDelta,
};

pub mod payload {
    use crate::upper_narrator::{PacingInstruction, StoryPhase};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(tag = "type", content = "payload", rename_all = "snake_case")]
    /// 命运编织者对外广播的流程事件。
    pub enum FateWeaverEvent {
        /// 请求启动整段故事流程。
        StartRequested,
        /// 记录某一轮最终叙事文本已经产出。
        NarrativeRendered {
            round: u32,
            narrative: String,
        },
        /// 广播故事阶段与整体进度已推进。
        PhaseAdvanced {
            round: u32,
            phase: FateWeaverPhase,
            progress_percent: u8,
        },
        /// 广播本轮被触发的世界事件。
        EventsTriggered {
            round: u32,
            events: Vec<TriggeredWorldEvent>,
        },
        /// 广播本轮推演出的直接后果与事实变化。
        ConsequencesDerived {
            round: u32,
            facts: Vec<ConsequenceFact>,
        },
        /// 广播本轮世界状态的增量变化。
        WorldStateChanged {
            round: u32,
            deltas: Vec<WorldStateDelta>,
        },
        /// 广播已整理好的主角视角世界快照。
        SnapshotPrepared {
            round: u32,
            snapshot: ProtagonistWorldSnapshot,
        },
        /// 请求上层叙事者开始渲染本轮叙事。
        NarrativeRequested {
            round: u32,
            brief: NarrationBrief,
        },
        /// 请求主角基于当前局势做出常规决策。
        ProtagonistDecisionRequested {
            round: u32,
            frame: ProtagonistDecisionFrame,
        },
        /// 广播故事已进入终局收束阶段。
        EndingSequenceStarted {
            round: u32,
            catalyst: EndingCatalyst,
        },
        /// 请求主角进行终局抉择。
        FinalDecisionRequested {
            round: u32,
            frame: FinalDecisionFrame,
        },
        /// 广播故事已结束以及涌现出的结局。
        StoryEnded {
            round: u32,
            ending: EmergentEnding,
        },
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum FateWeaverPhase {
        Setup,
        Rising,
        Climax,
        Falling,
        Resolution,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum FateWeaverPacing {
        Buildup,
        Tension,
        Release,
        Reflection,
        Climax,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum FateWeaverEventKind {
        Skeleton,
        Conditional,
        PlayerAction,
        Ending,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum FateWeaverEndingKind {
        Tragedy,
        Triumph,
        Bittersweet,
        Transformation,
        OpenEnded,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct TriggeredWorldEvent {
        pub event_id: String,
        pub kind: FateWeaverEventKind,
        pub summary: String,
        pub cause: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct ConsequenceFact {
        pub subject: String,
        pub change: String,
        pub horizon: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct WorldStateDelta {
        pub domain: String,
        pub summary: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct ProtagonistWorldSnapshot {
        pub location: String,
        pub surroundings: String,
        pub present_npcs: Vec<String>,
        pub available_resources: Vec<String>,
        pub active_threats: Vec<String>,
        pub recent_events: Vec<String>,
        pub emotional_context: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct NarrativeConstraints {
        pub tone: String,
        pub focus: String,
        pub length_hint: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct NarrationBrief {
        pub phase: FateWeaverPhase,
        pub pacing: FateWeaverPacing,
        pub triggered_events: Vec<TriggeredWorldEvent>,
        pub consequence_facts: Vec<ConsequenceFact>,
        pub constraints: NarrativeConstraints,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct ProtagonistDecisionFrame {
        pub situation: String,
        pub objective: String,
        pub urgency: String,
        pub stakes: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct EndingCatalyst {
        pub reason: String,
        pub unresolved_threads: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct FinalDecisionFrame {
        pub dilemma: String,
        pub stakes: String,
        pub options: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct EmergentEnding {
        pub kind: FateWeaverEndingKind,
        pub summary: String,
        pub cost: String,
    }

    impl FateWeaverPhase {
        pub fn from_raw(value: &str) -> Self {
            match value {
                "RISING" => Self::Rising,
                "CLIMAX" => Self::Climax,
                "FALLING" => Self::Falling,
                "RESOLUTION" => Self::Resolution,
                _ => Self::Setup,
            }
        }

        pub fn as_story_phase(&self) -> StoryPhase {
            match self {
                Self::Setup => StoryPhase::Setup,
                Self::Rising => StoryPhase::Rising,
                Self::Climax => StoryPhase::Climax,
                Self::Falling => StoryPhase::Falling,
                Self::Resolution => StoryPhase::Resolution,
            }
        }
    }

    impl FateWeaverPacing {
        pub fn from_raw(value: &str) -> Self {
            match value {
                "TENSION" => Self::Tension,
                "RELEASE" => Self::Release,
                "REFLECTION" => Self::Reflection,
                "CLIMAX" => Self::Climax,
                _ => Self::Buildup,
            }
        }

        pub fn as_story_pacing(&self) -> PacingInstruction {
            match self {
                Self::Buildup => PacingInstruction::Buildup,
                Self::Tension => PacingInstruction::Tension,
                Self::Release => PacingInstruction::Release,
                Self::Reflection => PacingInstruction::Reflection,
                Self::Climax => PacingInstruction::Climax,
            }
        }
    }

    impl FateWeaverEventKind {
        pub fn from_raw(value: &str) -> Self {
            match value {
                "CONDITIONAL" => Self::Conditional,
                "PLAYER_ACTION" => Self::PlayerAction,
                "ENDING" => Self::Ending,
                _ => Self::Skeleton,
            }
        }
    }
}


pub mod parse {
    use super::{
        ConsequenceFact, DecisionFrame, FactFragment, FateWeaverEndingKind, FateWeaverEventKind,
        FateWeaverPacing, FateWeaverPhase, NarrationTask, NarrativeConstraints,
        ProtagonistFinalDecisionFrame, ProtagonistPerception, ProtagonistWorldSnapshot,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct WorldFact {
        #[serde(rename = "type")]
        pub event_type: String,
        pub description: String,
        pub participants: Vec<String>,
        pub impact: String,
        pub cause: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct RoundFacts {
        pub round: u32,
        pub phase: String,
        pub progress: f32,
        pub events: Vec<WorldFact>,
        pub world_state_delta: String,
        pub character_state_delta: String,
        pub pacing_instruction: String,
        pub narrative_constraints: NarrativeConstraints,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct WorldSnapshot {
        pub current_location: String,
        pub surroundings: String,
        pub present_npcs: Vec<String>,
        pub available_resources: String,
        pub active_threats: String,
        pub recent_events: String,
        pub emotional_context: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    pub struct FateRoundEnvelope {
        pub facts: RoundFacts,
        pub world_snapshot: WorldSnapshot,
        #[serde(default)]
        pub should_end: bool,
        #[serde(default)]
        pub ending_hint: Option<String>,
    }

    impl WorldFact {
        pub fn as_triggered_event(&self) -> super::TriggeredWorldEvent {
            super::TriggeredWorldEvent {
                event_id: self.cause.clone(),
                kind: FateWeaverEventKind::from_raw(&self.event_type),
                summary: self.description.clone(),
                cause: self.cause.clone(),
            }
        }

        pub fn as_fact_fragment(&self) -> FactFragment {
            FactFragment {
                fact_id: self.cause.clone(),
                summary: self.description.clone(),
                participants: self.participants.clone(),
                impact: self.impact.clone(),
                cause: self.cause.clone(),
            }
        }
    }

    impl WorldSnapshot {
        pub fn as_protagonist_snapshot(&self) -> ProtagonistWorldSnapshot {
            ProtagonistWorldSnapshot {
                location: self.current_location.clone(),
                surroundings: self.surroundings.clone(),
                present_npcs: self.present_npcs.clone(),
                available_resources: split_summary(&self.available_resources),
                active_threats: split_summary(&self.active_threats),
                recent_events: split_summary(&self.recent_events),
                emotional_context: self.emotional_context.clone(),
            }
        }

        pub fn as_protagonist_perception(&self) -> ProtagonistPerception {
            ProtagonistPerception {
                location: self.current_location.clone(),
                surroundings: self.surroundings.clone(),
                present_npcs: self.present_npcs.clone(),
                available_resources: split_summary(&self.available_resources),
                active_threats: split_summary(&self.active_threats),
                recent_events: split_summary(&self.recent_events),
                emotional_context: self.emotional_context.clone(),
                unknowns: Vec::new(),
            }
        }
    }

    impl FateRoundEnvelope {
        pub fn phase(&self) -> FateWeaverPhase {
            FateWeaverPhase::from_raw(&self.facts.phase)
        }

        pub fn pacing(&self) -> FateWeaverPacing {
            FateWeaverPacing::from_raw(&self.facts.pacing_instruction)
        }

        pub fn progress_percent(&self) -> u8 {
            ((self.facts.progress * 100.0).round() as u32).min(100) as u8
        }

        pub fn decision_frame(&self) -> DecisionFrame {
            DecisionFrame {
                objective: self.facts.narrative_constraints.focus.clone(),
                urgency: self.facts.pacing_instruction.clone(),
                stakes: format!(
                    "{}｜{}",
                    self.facts.world_state_delta, self.facts.character_state_delta
                ),
                notable_uncertainties: split_summary(&self.world_snapshot.active_threats),
            }
        }

        pub fn final_decision_frame(&self) -> ProtagonistFinalDecisionFrame {
            ProtagonistFinalDecisionFrame {
                dilemma: self
                    .ending_hint
                    .clone()
                    .unwrap_or_else(|| "故事进入终局，主角必须做出最后抉择。".to_string()),
                stakes: format!(
                    "{}｜{}",
                    self.facts.world_state_delta, self.facts.character_state_delta
                ),
                options: self
                    .facts
                    .events
                    .iter()
                    .map(|event| event.description.clone())
                    .take(3)
                    .collect(),
            }
        }

        pub fn narration_task(&self) -> NarrationTask {
            NarrationTask {
                phase: self.phase().as_story_phase(),
                pacing: self.pacing().as_story_pacing(),
                tone: self.facts.narrative_constraints.tone.clone(),
                focus: self.facts.narrative_constraints.focus.clone(),
                length_hint: self.facts.narrative_constraints.length_hint.clone(),
                facts: self.facts.events.iter().map(WorldFact::as_fact_fragment).collect(),
                should_end: self.should_end,
            }
        }

        pub fn consequence_facts(&self) -> Vec<ConsequenceFact> {
            vec![
                ConsequenceFact {
                    subject: "world".to_string(),
                    change: self.facts.world_state_delta.clone(),
                    horizon: "immediate".to_string(),
                },
                ConsequenceFact {
                    subject: "protagonist".to_string(),
                    change: self.facts.character_state_delta.clone(),
                    horizon: "immediate".to_string(),
                },
            ]
        }

        pub fn infer_ending_kind(&self) -> FateWeaverEndingKind {
            let hint = self.ending_hint.as_deref().unwrap_or_default();
            if hint.contains("悲") || hint.contains("败") {
                FateWeaverEndingKind::Tragedy
            } else if hint.contains("凯旋") || hint.contains("胜") {
                FateWeaverEndingKind::Triumph
            } else if hint.contains("蜕变") || hint.contains("成长") {
                FateWeaverEndingKind::Transformation
            } else if hint.contains("开放") {
                FateWeaverEndingKind::OpenEnded
            } else {
                FateWeaverEndingKind::Bittersweet
            }
        }
    }

    fn split_summary(value: &str) -> Vec<String> {
        value
            .split(['、', '，', ',', '；', ';'])
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned)
            .collect()
    }
}

#[allow(unused_imports)]
pub use parse::{FateRoundEnvelope, RoundFacts, WorldFact, WorldSnapshot};



pub enum FateWeaverMessage {
    /// 请求启动整段故事流程。
    StartRequested,
    
}