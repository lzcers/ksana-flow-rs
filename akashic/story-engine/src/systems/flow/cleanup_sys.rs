use bevy_ecs::{
    entity::Entity,
    hierarchy::ChildOf,
    system::{Commands, Query},
};

use crate::components::{
    flow::{ApplicationCompleted, SimulationCompleted},
    outcome::{NarrationOutcome, SimulationOutcome},
    turn_flow::{TurnFlow, TurnStage},
};

// 在轮次收尾后清理 Agent 上的本轮结果组件；history/export 必须先读取它们。
#[allow(clippy::type_complexity)]
pub fn cleanup_previous_turn_outcomes_system(
    mut commands: Commands,
    sessions: Query<(Entity, &TurnFlow)>,
    outcomes: Query<(
        Entity,
        &ChildOf,
        Option<&SimulationOutcome>,
        Option<&NarrationOutcome>,
        Option<&SimulationCompleted>,
        Option<&ApplicationCompleted>,
    )>,
) {
    for (session_entity, turn_flow) in sessions
        .iter()
        .filter(|(_, flow)| matches!(flow.stage, TurnStage::TurnCompleted | TurnStage::Ended))
    {
        for (agent_entity, _, simulation, narration, simulation_completed, application_completed) in
            outcomes
                .iter()
                .filter(|(_, owner, ..)| owner.parent() == session_entity)
        {
            if simulation.is_some_and(|outcome| outcome.turn_id == turn_flow.active_turn_id) {
                commands.entity(agent_entity).remove::<SimulationOutcome>();
            }
            if narration.is_some_and(|outcome| outcome.turn_id == turn_flow.active_turn_id) {
                commands.entity(agent_entity).remove::<NarrationOutcome>();
            }
            if simulation_completed
                .is_some_and(|completed| completed.turn_id == turn_flow.active_turn_id)
            {
                commands
                    .entity(agent_entity)
                    .remove::<SimulationCompleted>();
            }
            if application_completed
                .is_some_and(|completed| completed.turn_id == turn_flow.active_turn_id)
            {
                commands
                    .entity(agent_entity)
                    .remove::<ApplicationCompleted>();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_ecs::{hierarchy::ChildOf, prelude::*, schedule::Schedule};

    use super::*;

    #[test]
    fn cleans_finished_turn_outcomes_during_finalize() {
        let mut world = World::new();
        let session = world
            .spawn(TurnFlow {
                turn_index: 1,
                active_turn_id: 1,
                stage: TurnStage::TurnCompleted,
            })
            .id();
        let agent = world
            .spawn((
                ChildOf(session),
                SimulationOutcome {
                    turn_id: 1,
                    content: "world".to_string(),
                },
                NarrationOutcome {
                    turn_id: 1,
                    content: "narration".to_string(),
                },
                SimulationCompleted { turn_id: 1 },
                ApplicationCompleted { turn_id: 1 },
            ))
            .id();
        let mut schedule = Schedule::default();
        schedule.add_systems(cleanup_previous_turn_outcomes_system);

        schedule.run(&mut world);
        world.flush();

        assert!(world.get::<SimulationOutcome>(agent).is_none());
        assert!(world.get::<NarrationOutcome>(agent).is_none());
        assert!(world.get::<SimulationCompleted>(agent).is_none());
        assert!(world.get::<ApplicationCompleted>(agent).is_none());
    }
}
