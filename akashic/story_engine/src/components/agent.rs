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
    pub name: String,
    pub sys_prompt: String,
    pub output_type: AgentOutputType,
    pub kind: AgentKind,
    pub context: Context,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SessionOwner(pub Entity);

pub type OwnedAgentMut<'a> = (Entity, &'a mut Agent, &'a SessionOwner);
pub type ReadyAgentFilter = (Without<PendingReasoning>, Without<RunningReasoning>);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentKind {
    Simulator,
    Applicator,
    Player,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentOutputType {
    #[serde(alias = "world_snapshot", alias = "protagonist_options")]
    Json,
    #[serde(alias = "simulation_text", alias = "narration")]
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PipelinePhase {
    Simulation,
    Application,
    ExternalInput,
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
        Self::new(
            AgentKind::Simulator,
            AgentOutputType::Json,
            "FateWeaver",
            system_prompt,
        )
    }

    pub fn new_upper_narrator(world_profile: &str, protagonist_profile: &str) -> Self {
        let system_prompt = UPPER_NARRATOR_PROMPT
            .replace("{world_profile}", world_profile)
            .replace("{protagonist_profile}", protagonist_profile);
        Self::new(
            AgentKind::Applicator,
            AgentOutputType::Text,
            "UpperNarrator",
            system_prompt,
        )
    }

    pub fn new_protagonist(world_profile: &str, protagonist_profile: &str) -> Self {
        let system_prompt = PROTAGONIST_PROMPT
            .replace("{world_profile}", world_profile)
            .replace("{protagonist_profile}", protagonist_profile);
        Self::new(
            AgentKind::Applicator,
            AgentOutputType::Json,
            "Protagonist",
            system_prompt,
        )
    }

    pub fn from_context(
        kind: AgentKind,
        output_type: AgentOutputType,
        name: impl Into<String>,
        sys_prompt: impl Into<String>,
        context: Context,
    ) -> Self {
        Self {
            kind,
            output_type,
            name: name.into(),
            sys_prompt: sys_prompt.into(),
            context,
        }
    }

    pub fn new(
        kind: AgentKind,
        output_type: AgentOutputType,
        name: impl Into<String>,
        system_prompt: String,
    ) -> Self {
        let mut context = Context::new();
        context.add_message(Message::system(&system_prompt));
        Self {
            kind,
            output_type,
            name: name.into(),
            sys_prompt: system_prompt,
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

impl AgentKind {
    pub fn pipeline_phase(self) -> PipelinePhase {
        match self {
            Self::Simulator => PipelinePhase::Simulation,
            Self::Applicator => PipelinePhase::Application,
            Self::Player => PipelinePhase::ExternalInput,
        }
    }
}
