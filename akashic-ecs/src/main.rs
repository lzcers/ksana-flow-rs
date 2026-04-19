mod components;
mod profile;
mod prompts;
mod resources;
mod systems;
mod utils;
use crate::{
    components::fate_weaver::{FateLine, FateWeaver},
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
};
use bevy_ecs::{schedule::Schedule, world::World};

fn main() {
    let mut world = World::new();
    let fate_weaver = world.spawn((
        FateWeaver::new(DEFAULT_WORLD_PROFILE, DEFAULT_PROTAGONIST_PROFILE),
        FateLine::default(),
    ));
    let mut schedule = Schedule::default();

    println!("Hello, world!");
}
