use bevy_ecs::{message::MessageReader, system::ResMut};

use crate::{
    resources::{
        manual_control::ManualControlState,
        turn_state::{TurnPhase, TurnState},
    },
    turn_messages::{TurnControl, TurnEvent},
};

// 统一管理故事轮次推进，只处理 phase 与消息，不直接解析模型结果。
pub fn turn_orchestrator_system(
    mut control_reader: MessageReader<TurnControl>,
    mut event_reader: MessageReader<TurnEvent>,
    mut manual_control: ResMut<ManualControlState>,
    mut turn_state: ResMut<TurnState>,
) {
    let mut should_start_turn = turn_state.phase == TurnPhase::Idle;

    for control in control_reader.read() {
        if handle_control(control, &mut turn_state, &mut manual_control) {
            should_start_turn = true;
        }
    }

    for event in event_reader.read() {
        if handle_event(event, &mut turn_state) {
            should_start_turn = true;
        }
    }

    maybe_start_turn(&mut turn_state, should_start_turn);
}

fn handle_control(
    control: &TurnControl,
    turn_state: &mut TurnState,
    manual_control: &mut ManualControlState,
) -> bool {
    match control {
        TurnControl::StartTurn { turn_id } if turn_state.phase == TurnPhase::Idle => {
            turn_state.active_turn_id = *turn_id;
            true
        }
        TurnControl::StartTurn { .. } => false,
        TurnControl::ResetTurn { next_turn_id } => {
            manual_control.pending_protagonist_override_turn = None;
            turn_state.reset(*next_turn_id);
            true
        }
        TurnControl::SubmitProtagonistAction {
            turn_id,
            action_text,
        } if *turn_id == turn_state.active_turn_id
            && turn_state.phase == TurnPhase::AwaitingProtagonist =>
        {
            manual_control.pending_protagonist_override_turn = None;
            apply_protagonist_action(turn_state, action_text);
            false
        }
        TurnControl::SubmitProtagonistAction { .. } => false,
    }
}

fn handle_event(event: &TurnEvent, turn_state: &mut TurnState) -> bool {
    match event {
        TurnEvent::SceneFateGenerated {
            turn_id,
            scene_facts,
        } if *turn_id == turn_state.active_turn_id => {
            turn_state.latest_broadcast_summary = scene_facts.clone();
            turn_state.phase = TurnPhase::NarratorScene;
            false
        }
        TurnEvent::SceneNarrationGenerated {
            turn_id,
            scene_text: _scene_text,
        } if *turn_id == turn_state.active_turn_id => {
            turn_state.phase = TurnPhase::AwaitingProtagonist;
            false
        }
        TurnEvent::ProtagonistActionGenerated {
            turn_id,
            action_text,
        } if *turn_id == turn_state.active_turn_id
            && turn_state.phase == TurnPhase::AwaitingProtagonist =>
        {
            apply_protagonist_action(turn_state, action_text);
            false
        }
        TurnEvent::ConsequenceFateGenerated {
            turn_id,
            consequence_facts,
        } if *turn_id == turn_state.active_turn_id => {
            turn_state.latest_broadcast_summary = consequence_facts.clone();
            turn_state.phase = TurnPhase::NarratorStory;
            false
        }
        TurnEvent::StoryNarrationGenerated { turn_id, .. }
            if *turn_id == turn_state.active_turn_id =>
        {
            turn_state.finish_turn();
            true
        }
        TurnEvent::TaskFailed {
            turn_id,
            stage: _stage,
            entity: _entity,
            message: _message,
        } if *turn_id == turn_state.active_turn_id => {
            turn_state.phase = TurnPhase::Failed;
            false
        }
        _ => false,
    }
}

fn maybe_start_turn(turn_state: &mut TurnState, should_start_turn: bool) {
    if should_start_turn && turn_state.phase == TurnPhase::Idle {
        let turn_id = turn_state.active_turn_id;
        turn_state.start_turn(turn_id);
    }
}

fn apply_protagonist_action(turn_state: &mut TurnState, action_text: &str) {
    turn_state.latest_protagonist_action = action_text.trim().to_string();
    turn_state.phase = TurnPhase::FateConsequence;
}
