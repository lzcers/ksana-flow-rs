mod components;
mod profile;
mod prompts;
mod resources;
mod systems;
mod utils;
use crate::{
    components::fate_weaver::{FateLine, FateWeaver},
    components::protagonist::Protagonist,
    components::upper_narrator::UpperNarrator,
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{task_manager::TaskManager, turn_state::TurnState},
    systems::fate_weaver_sys::fate_weaver_system,
    systems::protagonist_sys::protagonist_system,
    systems::task_sys::task_system,
    systems::turn_transition_sys::turn_transition_system,
    systems::upper_narrator_sys::upper_narrator_system,
};
use bevy_ecs::{
    schedule::{IntoScheduleConfigs, Schedule},
    world::World,
};

fn main() {
    let mut world = World::new();
    world.insert_resource(TaskManager::default());
    world.insert_resource(TurnState::default());

    let _fate_weaver = world.spawn((
        FateWeaver::new(DEFAULT_WORLD_PROFILE, DEFAULT_PROTAGONIST_PROFILE),
        FateLine::default(),
    ));
    let _protagonist = world.spawn((Protagonist::new(DEFAULT_PROTAGONIST_PROFILE),));
    let _upper_narrator = world.spawn((UpperNarrator::new(),));
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            fate_weaver_system,
            protagonist_system,
            upper_narrator_system,
            task_system,
            turn_transition_system,
        )
            .chain(),
    );

    println!("Hello, world!");
}
