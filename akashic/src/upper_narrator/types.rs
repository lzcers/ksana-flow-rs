#![allow(dead_code)]

pub mod payload {
    use super::parse::{EndingCue, NarrationTask, RevisionFeedback};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    #[serde(tag = "type", content = "payload", rename_all = "snake_case")]
    pub enum UpperNarratorEvent {
        NarrationTaskReceived {
            round: u32,
            task: NarrationTask,
        },
        RevisionRequested {
            round: u32,
            feedback: RevisionFeedback,
        },
        EndingCueReceived {
            round: u32,
            cue: EndingCue,
        },
        StyleSelected {
            round: u32,
            style: NarrativeStyle,
            perspective: NarrativePerspective,
        },
        SceneOutlined {
            round: u32,
            beats: Vec<SceneBeat>,
        },
        NarrativeChunkProduced {
            round: u32,
            content: String,
        },
        NarrativeCompleted {
            round: u32,
            narrative: RenderedNarrative,
        },
        FidelityIssueDetected {
            round: u32,
            issue: FidelityIssue,
        },
        ProtocolError {
            round: Option<u32>,
            message: String,
        },
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum StoryPhase {
        Setup,
        Rising,
        Climax,
        Falling,
        Resolution,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum PacingInstruction {
        Buildup,
        Tension,
        Release,
        Reflection,
        Climax,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum NarrativeStyle {
        Literary,
        Suspense,
        Epic,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    #[serde(rename_all = "snake_case")]
    pub enum NarrativePerspective {
        FirstPerson,
        SecondPerson,
        ThirdPerson,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct SceneBeat {
        pub beat: String,
        pub sensory_focus: String,
        pub dialogue_purpose: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct FidelityIssue {
        pub scope: String,
        pub detail: String,
        pub fix_hint: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct RenderedNarrative {
        pub text: String,
        pub style: NarrativeStyle,
        pub perspective: NarrativePerspective,
    }

    impl NarrativeStyle {
        pub fn for_task(task: &NarrationTask) -> Self {
            match task.pacing {
                PacingInstruction::Climax | PacingInstruction::Release => Self::Epic,
                PacingInstruction::Tension => Self::Suspense,
                _ => Self::Literary,
            }
        }

        pub fn label(&self) -> &'static str {
            match self {
                Self::Literary => "文学风",
                Self::Suspense => "悬疑风",
                Self::Epic => "史诗风",
            }
        }
    }

    impl NarrativePerspective {
        pub fn reader_default() -> Self {
            Self::ThirdPerson
        }

        pub fn label(&self) -> &'static str {
            match self {
                Self::FirstPerson => "第一人称",
                Self::SecondPerson => "第二人称",
                Self::ThirdPerson => "第三人称",
            }
        }
    }
}

#[allow(unused_imports)]
pub use payload::{
    FidelityIssue, NarrativePerspective, NarrativeStyle, PacingInstruction, RenderedNarrative,
    SceneBeat, StoryPhase, UpperNarratorEvent,
};

pub mod parse {
    use super::{FidelityIssue, PacingInstruction, SceneBeat, StoryPhase};
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct FactFragment {
        pub fact_id: String,
        pub summary: String,
        pub participants: Vec<String>,
        pub impact: String,
        pub cause: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct NarrationTask {
        pub phase: StoryPhase,
        pub pacing: PacingInstruction,
        pub tone: String,
        pub focus: String,
        pub length_hint: String,
        pub facts: Vec<FactFragment>,
        pub should_end: bool,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct RevisionFeedback {
        pub issues: Vec<FidelityIssue>,
        pub preserve: Vec<String>,
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
    pub struct EndingCue {
        pub ending_kind: String,
        pub unresolved_threads: Vec<String>,
        pub emotional_aftertaste: String,
    }

    impl NarrationTask {
        pub fn scene_beats(&self) -> Vec<SceneBeat> {
            self.facts
                .iter()
                .map(|fact| SceneBeat {
                    beat: fact.summary.clone(),
                    sensory_focus: self.tone.clone(),
                    dialogue_purpose: self.focus.clone(),
                })
                .collect()
        }
    }
}

#[allow(unused_imports)]
pub use parse::{EndingCue, FactFragment, NarrationTask, RevisionFeedback};
