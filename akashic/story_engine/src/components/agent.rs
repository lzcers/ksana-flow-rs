use agent::{agent::Context, core::Message};
use bevy_ecs::{component::Component, entity::Entity, query::Without};
use serde::{Deserialize, Serialize};

use crate::prompts::{
    fate_weaver_prompt::{FATE_BASE_SYSTEM_PROMPT, OUTPUT_SCHEMA},
    protagonist_prompt::PROTAGONIST_PROMPT,
    upper_narrator_prompt::UPPER_NARRATOR_PROMPT,
};
#[derive(Component, Debug, Clone)]
pub struct Agent {
    pub output_kind: AgentOutputKind,
    pub name: String,
    pub context: Context,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SessionOwner(pub Entity);

pub type OwnedAgentMut<'a> = (Entity, &'a mut Agent, &'a SessionOwner);
pub type ReadyAgentFilter = (Without<PendingReasoning>, Without<RunningReasoning>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentOutputKind {
    WorldSnapshot,
    SimulationText,
    Narration,
    ProtagonistOptions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelinePhase {
    Simulation,
    Application,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct PendingReasoning;

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RunningReasoning;

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct NarrationOutcome {
    pub turn_id: u64,
    pub content: String,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SimulationOutcome {
    pub turn_id: u64,
    pub content: String,
}

impl Agent {
    pub fn new_fate_weaver(
        world_profile: &str,
        protagonist_profile: &str,
        key_story_beats: &str,
    ) -> Self {
        let system_prompt = FATE_BASE_SYSTEM_PROMPT
            .replace("{world_profile}", world_profile)
            .replace("{protagonist_profile}", protagonist_profile)
            .replace("{key_story_beats}", key_story_beats)
            .replace("{output_schema}", OUTPUT_SCHEMA);
        Self::new_custom(AgentOutputKind::WorldSnapshot, "FateWeaver", system_prompt)
    }

    pub fn new_upper_narrator(world_profile: &str, protagonist_profile: &str) -> Self {
        let system_prompt = UPPER_NARRATOR_PROMPT
            .replace("{world_profile}", world_profile)
            .replace("{protagonist_profile}", protagonist_profile);
        Self::new_custom(AgentOutputKind::Narration, "UpperNarrator", system_prompt)
    }

    pub fn new_protagonist(world_profile: &str, protagonist_profile: &str) -> Self {
        let system_prompt = PROTAGONIST_PROMPT
            .replace("{world_profile}", world_profile)
            .replace("{protagonist_profile}", protagonist_profile);
        Self::new_custom(
            AgentOutputKind::ProtagonistOptions,
            "Protagonist",
            system_prompt,
        )
    }

    pub fn from_context(
        output_kind: AgentOutputKind,
        name: impl Into<String>,
        context: Context,
    ) -> Self {
        Self {
            output_kind,
            name: name.into(),
            context,
        }
    }

    pub fn new_custom(
        output_kind: AgentOutputKind,
        name: impl Into<String>,
        system_prompt: String,
    ) -> Self {
        let mut context = Context::new();
        context.add_message(Message::system(&system_prompt));
        Self {
            output_kind,
            name: name.into(),
            context,
        }
    }

    pub fn append_user_message(&mut self, content: &str) {
        self.context.add_message(Message::user(content));
    }

    pub fn append_assistant_message(&mut self, content: &str) {
        self.context.add_message(Message::assistant(content));
    }

    pub fn revert(&mut self) {
        self.context.rollback_latest_input();
    }
}

impl AgentOutputKind {
    pub fn is_simulation(self) -> bool {
        matches!(self, Self::WorldSnapshot | Self::SimulationText)
    }

    pub fn pipeline_phase(self) -> PipelinePhase {
        match self {
            Self::WorldSnapshot | Self::SimulationText => PipelinePhase::Simulation,
            Self::Narration | Self::ProtagonistOptions => PipelinePhase::Application,
        }
    }
}
