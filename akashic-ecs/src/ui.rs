use std::collections::HashMap;

use bevy_ecs::{
    entity::Entity,
    message::MessageReader,
    resource::Resource,
    system::{Res, ResMut},
};

use crate::{
    resources::{
        task_manager::{TaskKind, TaskManager},
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::TurnEvent,
};

#[derive(Debug, Clone)]
pub enum UiEvent {
    Status {
        turn_id: u64,
        text: String,
    },
    FateChunk {
        turn_id: u64,
        chunk: String,
    },
    Narration {
        turn_id: u64,
        title: String,
        content: String,
    },
    ProtagonistAction {
        turn_id: u64,
        action_text: String,
    },
    Error {
        turn_id: u64,
        text: String,
    },
}

#[derive(Resource, Default)]
pub struct UiEventBuffer {
    pending: Vec<UiEvent>,
    chunk_offsets: HashMap<Entity, usize>,
    last_phase: Option<TurnPhase>,
    last_turn_id: Option<u64>,
}

impl UiEventBuffer {
    pub fn drain(&mut self) -> Vec<UiEvent> {
        std::mem::take(&mut self.pending)
    }

    fn push(&mut self, event: UiEvent) {
        self.pending.push(event);
    }
}

pub fn ui_bridge_system(
    turn_state: Res<TurnState>,
    task_manager: Res<TaskManager>,
    mut event_reader: MessageReader<TurnEvent>,
    mut buffer: ResMut<UiEventBuffer>,
) {
    buffer.push_phase_status(&turn_state);
    buffer.push_fate_chunks(turn_state.active_turn_id, &task_manager);

    for event in event_reader.read() {
        match event {
            TurnEvent::SceneNarrationGenerated {
                turn_id,
                scene_text,
            } => buffer.push(UiEvent::Narration {
                turn_id: *turn_id,
                title: "场景叙事".to_string(),
                content: scene_text.clone(),
            }),
            TurnEvent::StoryNarrationGenerated {
                turn_id,
                story_text,
            } => buffer.push(UiEvent::Narration {
                turn_id: *turn_id,
                title: "故事叙事".to_string(),
                content: story_text.clone(),
            }),
            TurnEvent::ProtagonistActionGenerated {
                turn_id,
                action_text,
            } => buffer.push(UiEvent::ProtagonistAction {
                turn_id: *turn_id,
                action_text: action_text.clone(),
            }),
            TurnEvent::TaskFailed {
                turn_id,
                stage,
                message,
                ..
            } => buffer.push(UiEvent::Error {
                turn_id: *turn_id,
                text: format!(
                    "回合 {} 在 {} 失败: {}",
                    turn_id,
                    phase_label(*stage),
                    message
                ),
            }),
            TurnEvent::SceneFateGenerated { turn_id, .. } => buffer.push(UiEvent::Status {
                turn_id: *turn_id,
                text: format!("第 {} 轮进入场景叙事", turn_id),
            }),
            TurnEvent::ConsequenceFateGenerated { turn_id, .. } => buffer.push(UiEvent::Status {
                turn_id: *turn_id,
                text: format!("第 {} 轮进入故事叙事", turn_id),
            }),
        }
    }
}

impl UiEventBuffer {
    fn push_phase_status(&mut self, turn_state: &TurnState) {
        if self.last_phase == Some(turn_state.phase)
            && self.last_turn_id == Some(turn_state.active_turn_id)
        {
            return;
        }

        self.last_phase = Some(turn_state.phase);
        self.last_turn_id = Some(turn_state.active_turn_id);
        self.push(UiEvent::Status {
            turn_id: turn_state.active_turn_id,
            text: format!(
                "第 {} 轮: {}",
                turn_state.active_turn_id,
                phase_label(turn_state.phase)
            ),
        });
    }

    fn push_fate_chunks(&mut self, turn_id: u64, task_manager: &TaskManager) {
        let tasks = task_manager.task_results_snapshot();
        let known_offsets = self.chunk_offsets.clone();
        let mut active_entities = Vec::with_capacity(tasks.len());
        let mut pending_events = Vec::new();
        let mut next_offsets = Vec::new();

        for (entity, result) in tasks {
            active_entities.push(entity);
            if !matches!(
                result.kind,
                TaskKind::FateScenePlanning | TaskKind::FateConsequence
            ) {
                continue;
            }

            let offset = known_offsets.get(&entity).copied().unwrap_or(0);
            if offset >= result.chunks.len() {
                continue;
            }

            pending_events.extend(
                result.chunks[offset..]
                    .iter()
                    .cloned()
                    .map(|chunk| UiEvent::FateChunk { turn_id, chunk }),
            );
            next_offsets.push((entity, result.chunks.len()));
        }

        self.pending.extend(pending_events);
        for (entity, offset) in next_offsets {
            self.chunk_offsets.insert(entity, offset);
        }
        self.chunk_offsets
            .retain(|entity, _| active_entities.contains(entity));
    }
}

fn phase_label(phase: TurnPhase) -> &'static str {
    match phase {
        TurnPhase::Idle => "等待新回合",
        TurnPhase::FateWeaving => "命运编排中",
        TurnPhase::NarratorScene => "场景叙事中",
        TurnPhase::AwaitingProtagonist => "主角决策中",
        TurnPhase::FateConsequence => "后果推演中",
        TurnPhase::NarratorStory => "故事叙事中",
        TurnPhase::Failed => "执行失败",
    }
}
