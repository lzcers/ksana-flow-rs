use bevy_ecs::{message::MessageReader, system::Query, system::ResMut};

use crate::{
    components::fate_weaver::{
        FateWeaver, format_fate_resolution_context, write_shared_fate_context,
    },
    resources::turn_state::{TurnPhase, TurnState},
    turn_messages::{TurnControl, TurnEvent},
};

// 统一管理故事轮次推进，只处理 phase 与消息，不直接解析模型结果。
pub fn turn_orchestrator_system(
    mut fate_weaver_query: Query<&mut FateWeaver>,
    mut control_reader: MessageReader<TurnControl>,
    mut event_reader: MessageReader<TurnEvent>,
    mut turn_state: ResMut<TurnState>,
) {
    let mut should_start_turn = turn_state.phase == TurnPhase::Idle;

    for control in control_reader.read() {
        if handle_control(control, &mut fate_weaver_query, &mut turn_state) {
            should_start_turn = true;
        }
    }

    for event in event_reader.read() {
        if handle_event(event, &mut fate_weaver_query, &mut turn_state) {
            should_start_turn = true;
        }
    }

    maybe_start_turn(&mut turn_state, should_start_turn);
}

fn handle_control(
    control: &TurnControl,
    fate_weaver_query: &mut Query<&mut FateWeaver>,
    turn_state: &mut TurnState,
) -> bool {
    match control {
        TurnControl::StartTurn { turn_id } if turn_state.phase == TurnPhase::Idle => {
            turn_state.active_turn_id = *turn_id;
            true
        }
        TurnControl::StartTurn { .. } => false,
        TurnControl::ResetTurn { next_turn_id } => {
            turn_state.reset(*next_turn_id);
            true
        }
        TurnControl::SubmitProtagonistAction {
            turn_id,
            action_text,
        } if *turn_id == turn_state.active_turn_id
            && turn_state.phase == TurnPhase::AwaitingProtagonist =>
        {
            apply_protagonist_action(fate_weaver_query, turn_state, action_text);
            false
        }
        TurnControl::SubmitProtagonistAction { .. } => false,
    }
}

fn handle_event(
    event: &TurnEvent,
    fate_weaver_query: &mut Query<&mut FateWeaver>,
    turn_state: &mut TurnState,
) -> bool {
    match event {
        TurnEvent::SceneFateGenerated {
            turn_id,
            scene_facts,
        } if *turn_id == turn_state.active_turn_id => {
            turn_state.latest_fate_summary = scene_facts.clone();
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
            apply_protagonist_action(fate_weaver_query, turn_state, action_text);
            false
        }
        TurnEvent::ConsequenceFateGenerated {
            turn_id,
            consequence_facts,
        } if *turn_id == turn_state.active_turn_id => {
            turn_state.latest_fate_summary = consequence_facts.clone();
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

fn apply_protagonist_action(
    fate_weaver_query: &mut Query<&mut FateWeaver>,
    turn_state: &mut TurnState,
    action_text: &str,
) {
    if let Ok(mut fate_weaver) = fate_weaver_query.single_mut() {
        let context = format_fate_resolution_context(&turn_state.latest_fate_summary, action_text);
        if !context.is_empty() {
            write_shared_fate_context(fate_weaver.get_context_mut(), &context);
        }
    }
    turn_state.phase = TurnPhase::FateConsequence;
}
