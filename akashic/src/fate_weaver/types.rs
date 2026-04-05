
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ==================== 输入定义 ====================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InitializationInput {
    pub world_setting: String,
    pub protagonist_profile: ProtagonistProfile,
    pub story_theme: StoryTheme,
    pub skeleton_events: Vec<SkeletonEventSeed>,
    pub conditional_events: Vec<ConditionalEventTemplate>,
    pub narrative_params: NarrativeParams,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProtagonistProfile {
    pub name: String,
    pub background: String,
    pub abilities: Vec<String>,
    pub personality: String,
    pub initial_goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoryTheme {
    pub core_conflict: String,
    pub narrative_style: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkeletonEventSeed {
    pub event_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConditionalEventTemplate {
    pub id: String,
    pub event_type: String,
    pub preconditions: Vec<String>,
    pub probability: f64,
    pub cooldown: u32,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NarrativeParams {
    pub max_rounds: u32,
    pub convergence_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeInput {
    pub round: u32,
    pub timestamp: String,
    pub protagonist_action: Option<ProtagonistAction>,
    pub npc_actions: Vec<NpcAction>,
    pub user_directive: Option<UserDirective>,
    pub previous_world_state: Option<WorldState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProtagonistAction {
    pub actor: String,
    pub action: String,
    pub location: String,
    pub resources_consumed: Vec<String>,
    pub intent: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NpcAction {
    pub actor: String,
    pub action: String,
    pub location: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum UserDirective {
    JumpTime { to_time: String },
    ForceEvent { event_id: String },
    RequestEnd,
}

// ==================== 输出定义 ====================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Output {
    pub round: u32,
    pub output_type: OutputType,
    pub action_verdict: Option<ActionVerdict>,
    pub world_state_update: WorldStateUpdate,
    pub event_trigger: Option<EventTrigger>,
    pub convergence: Option<ConvergenceOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OutputType {
    NormalRound,
    Convergence,
    Ending,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionVerdict {
    pub accepted: bool,
    pub action: String,
    pub rule_checks: RuleChecks,
    pub modifications: Option<Vec<String>>,
    pub rejection_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuleChecks {
    pub timeline: String,
    pub world_rules: String,
    pub causality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldStateUpdate {
    pub timestamp: String,
    pub elapsed_time: String,
    pub weather: Option<String>,
    pub protagonist: ProtagonistState,
    pub faction_relations: HashMap<String, i32>,
    pub global_events: Vec<String>,
    pub story_progress: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProtagonistState {
    pub status: String,
    pub resources: HashMap<String, u32>,
    pub reputation: HashMap<String, i32>,
    pub psychology: String,
    pub clues_unlocked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorldState {
    pub timestamp: String,
    pub weather: String,
    pub protagonist: ProtagonistState,
    pub faction_relations: HashMap<String, i32>,
    pub active_global_events: Vec<String>,
    pub story_progress: f64,
    pub cooldown_events: Vec<CooldownEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CooldownEvent {
    pub event_id: String,
    pub remaining_rounds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EventTrigger {
    pub event_id: String,
    pub event_type: EventType,
    pub name: String,
    pub trigger_reason: String,
    pub fact_list: Vec<String>,
    pub causal_chain: String,
    pub newly_available_events: Vec<String>,
    pub cooldown_events: Vec<CooldownEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    Skeleton,
    Conditional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConvergenceOutput {
    pub convergence_signal: ConvergenceSignal,
    pub final_event: Option<EventTrigger>,
    pub clue_resolution: Vec<ClueResolution>,
    pub final_choice: FinalChoice,
    pub ending_suggestion: EndingType,
    pub ending_report: EndingReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConvergenceSignal {
    pub triggered: bool,
    pub trigger_condition: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClueResolution {
    pub clue_id: String,
    pub final_state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FinalChoice {
    pub description: String,
    pub options: Vec<ChoiceOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChoiceOption {
    pub id: String,
    pub description: String,
    pub consequences: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EndingType {
    Tragedy,
    Triumph,
    Bittersweet,
    Transformation,
    OpenEnded,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EndingReport {
    pub scores: HashMap<String, f64>,
    pub justification: String,
}

// ==================== 主输入/输出 Enum ====================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FateWeaverInput {
    Initialization(InitializationInput),
    Runtime(RuntimeInput),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FateWeaverOutput {
    Normal(Output),
    Error { message: String },
}

// ==================== Builder 方法实现 ====================

impl InitializationInput {
    pub fn new(
        world_setting: String,
        protagonist_profile: ProtagonistProfile,
        story_theme: StoryTheme,
        skeleton_events: Vec<SkeletonEventSeed>,
        conditional_events: Vec<ConditionalEventTemplate>,
        narrative_params: NarrativeParams,
    ) -> Self {
        Self {
            world_setting,
            protagonist_profile,
            story_theme,
            skeleton_events,
            conditional_events,
            narrative_params,
        }
    }
}

impl ProtagonistProfile {
    pub fn new(
        name: String,
        background: String,
        abilities: Vec<String>,
        personality: String,
        initial_goal: String,
    ) -> Self {
        Self {
            name,
            background,
            abilities,
            personality,
            initial_goal,
        }
    }
}

impl StoryTheme {
    pub fn new(core_conflict: String, narrative_style: String) -> Self {
        Self {
            core_conflict,
            narrative_style,
        }
    }
}

impl SkeletonEventSeed {
    pub fn new(event_type: String, description: String) -> Self {
        Self {
            event_type,
            description,
        }
    }
}

impl ConditionalEventTemplate {
    pub fn new(
        id: String,
        event_type: String,
        preconditions: Vec<String>,
        probability: f64,
        cooldown: u32,
        description: String,
    ) -> Self {
        Self {
            id,
            event_type,
            preconditions,
            probability,
            cooldown,
            description,
        }
    }
}

impl NarrativeParams {
    pub fn new(max_rounds: u32, convergence_threshold: f64) -> Self {
        Self {
            max_rounds,
            convergence_threshold,
        }
    }
}

impl RuntimeInput {
    pub fn new(
        round: u32,
        timestamp: String,
        protagonist_action: Option<ProtagonistAction>,
        npc_actions: Vec<NpcAction>,
        user_directive: Option<UserDirective>,
        previous_world_state: Option<WorldState>,
    ) -> Self {
        Self {
            round,
            timestamp,
            protagonist_action,
            npc_actions,
            user_directive,
            previous_world_state,
        }
    }
}

impl ProtagonistAction {
    pub fn new(
        actor: String,
        action: String,
        location: String,
        resources_consumed: Vec<String>,
        intent: String,
    ) -> Self {
        Self {
            actor,
            action,
            location,
            resources_consumed,
            intent,
        }
    }
}

impl NpcAction {
    pub fn new(actor: String, action: String, location: String) -> Self {
        Self {
            actor,
            action,
            location,
        }
    }
}

impl Output {
    pub fn normal_round(
        round: u32,
        action_verdict: Option<ActionVerdict>,
        world_state_update: WorldStateUpdate,
        event_trigger: Option<EventTrigger>,
    ) -> Self {
        Self {
            round,
            output_type: OutputType::NormalRound,
            action_verdict,
            world_state_update,
            event_trigger,
            convergence: None,
        }
    }

    pub fn convergence(
        round: u32,
        world_state_update: WorldStateUpdate,
        convergence: ConvergenceOutput,
    ) -> Self {
        Self {
            round,
            output_type: OutputType::Convergence,
            action_verdict: None,
            world_state_update,
            event_trigger: None,
            convergence: Some(convergence),
        }
    }
}

impl ActionVerdict {
    pub fn accepted(action: String) -> Self {
        Self {
            accepted: true,
            action,
            rule_checks: RuleChecks::passed(),
            modifications: None,
            rejection_reason: None,
        }
    }

    pub fn rejected(action: String, reason: String) -> Self {
        Self {
            accepted: false,
            action,
            rule_checks: RuleChecks::default(),
            modifications: None,
            rejection_reason: Some(reason),
        }
    }

    pub fn modified(action: String, modifications: Vec<String>) -> Self {
        Self {
            accepted: true,
            action,
            rule_checks: RuleChecks::default(),
            modifications: Some(modifications),
            rejection_reason: None,
        }
    }
}

impl RuleChecks {
    pub fn passed() -> Self {
        Self {
            timeline: "通过".to_string(),
            world_rules: "通过".to_string(),
            causality: "通过".to_string(),
        }
    }
}

impl Default for RuleChecks {
    fn default() -> Self {
        Self {
            timeline: "未检查".to_string(),
            world_rules: "未检查".to_string(),
            causality: "未检查".to_string(),
        }
    }
}

impl WorldStateUpdate {
    pub fn new(
        timestamp: String,
        elapsed_time: String,
        weather: Option<String>,
        protagonist: ProtagonistState,
        story_progress: f64,
    ) -> Self {
        Self {
            timestamp,
            elapsed_time,
            weather,
            protagonist,
            faction_relations: HashMap::new(),
            global_events: Vec::new(),
            story_progress,
        }
    }
}

impl ProtagonistState {
    pub fn new(status: String, psychology: String) -> Self {
        Self {
            status,
            resources: HashMap::new(),
            reputation: HashMap::new(),
            psychology,
            clues_unlocked: Vec::new(),
        }
    }
}

impl EventTrigger {
    pub fn skeleton(
        event_id: String,
        name: String,
        trigger_reason: String,
        fact_list: Vec<String>,
        causal_chain: String,
    ) -> Self {
        Self {
            event_id,
            event_type: EventType::Skeleton,
            name,
            trigger_reason,
            fact_list,
            causal_chain,
            newly_available_events: Vec::new(),
            cooldown_events: Vec::new(),
        }
    }

    pub fn conditional(
        event_id: String,
        name: String,
        trigger_reason: String,
        fact_list: Vec<String>,
        causal_chain: String,
        newly_available_events: Vec<String>,
        cooldown_events: Vec<CooldownEvent>,
    ) -> Self {
        Self {
            event_id,
            event_type: EventType::Conditional,
            name,
            trigger_reason,
            fact_list,
            causal_chain,
            newly_available_events,
            cooldown_events,
        }
    }
}

impl CooldownEvent {
    pub fn new(event_id: String, remaining_rounds: u32) -> Self {
        Self {
            event_id,
            remaining_rounds,
        }
    }
}

impl ConvergenceOutput {
    pub fn new(
        convergence_signal: ConvergenceSignal,
        clue_resolution: Vec<ClueResolution>,
        final_choice: FinalChoice,
        ending_suggestion: EndingType,
        ending_report: EndingReport,
    ) -> Self {
        Self {
            convergence_signal,
            final_event: None,
            clue_resolution,
            final_choice,
            ending_suggestion,
            ending_report,
        }
    }
}

impl FinalChoice {
    pub fn new(description: String, options: Vec<ChoiceOption>) -> Self {
        Self {
            description,
            options,
        }
    }
}

impl ChoiceOption {
    pub fn new(id: String, description: String, consequences: String) -> Self {
        Self {
            id,
            description,
            consequences,
        }
    }
}

impl EndingReport {
    pub fn new(scores: HashMap<String, f64>, justification: String) -> Self {
        Self {
            scores,
            justification,
        }
    }
}

impl FateWeaverInput {
    pub fn initialization(input: InitializationInput) -> Self {
        Self::Initialization(input)
    }

    pub fn runtime(input: RuntimeInput) -> Self {
        Self::Runtime(input)
    }
}

impl FateWeaverOutput {
    pub fn normal(output: Output) -> Self {
        Self::Normal(output)
    }

    pub fn error(message: String) -> Self {
        Self::Error { message }
    }
}

