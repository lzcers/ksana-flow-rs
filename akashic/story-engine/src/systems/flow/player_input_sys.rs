use bevy_ecs::{
    entity::Entity,
    system::{Commands, Query},
};

use crate::components::{
    flow::PlayerInputCompleted,
    turn_flow::{TurnFlow, TurnStage},
};

pub fn player_input_progress_system(
    mut commands: Commands,
    mut sessions: Query<(Entity, &mut TurnFlow, &PlayerInputCompleted)>,
) {
    for (entity, mut flow, completed) in sessions.iter_mut() {
        if flow.stage != TurnStage::AwaitingPlayer || completed.turn_id != flow.active_turn_id {
            continue;
        }

        commands.entity(entity).remove::<PlayerInputCompleted>();
        flow.finish_turn();
    }
}
