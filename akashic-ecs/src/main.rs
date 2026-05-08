use akashic_ecs::{
    components::{
        fate_weaver::FateWeaver, protagonist::Protagonist, upper_narrator::UpperNarrator,
    },
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{
        protagonist_action::ProtagonistAction,
        task_manager::TaskManager,
        turn_state::{TurnPhase, TurnState},
        world_snapshot::WorldSnapshot,
    },
    systems::{
        fate_weaver_sys::{fate_weaver_apply_system, fate_weaver_system},
        protagonist_sys::{protagonist_apply_system, protagonist_system},
        task_sys::task_system,
        turn_orchestrator_sys::turn_orchestrator_system,
        upper_narrator_sys::{upper_narrator_apply_system, upper_narrator_system},
    },
    turn_messages::TurnEvent,
    utils::build_chat_model,
};
use bevy_ecs::{
    message::{Messages, message_update_system},
    prelude::*,
    schedule::Schedule,
};
use std::{collections::HashMap, env};

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    if env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("请先设置 DEEPSEEK_API_KEY，再运行 `cargo run -p akashic-ecs` 体验最小 demo。");
        return;
    }

    // build world
    let mut world = World::new();

    // 资源初始化
    world.init_resource::<WorldSnapshot>();
    world.init_resource::<TurnState>();
    world.init_resource::<ProtagonistAction>();
    world.init_resource::<Messages<TurnEvent>>();
    world.insert_resource(TaskManager::new(build_chat_model()));

    // 实体初始化
    world.spawn(FateWeaver::new(
        DEFAULT_WORLD_PROFILE,
        DEFAULT_PROTAGONIST_PROFILE,
    ));
    world.spawn(UpperNarrator::new(
        DEFAULT_WORLD_PROFILE,
        DEFAULT_PROTAGONIST_PROFILE,
    ));
    world.spawn(Protagonist::new(
        DEFAULT_WORLD_PROFILE,
        DEFAULT_PROTAGONIST_PROFILE,
    ));

    // 调度器初始化
    let mut schedule = Schedule::default();

    schedule.add_systems(
        (
            task_system,
            message_update_system,
            turn_orchestrator_system,
            fate_weaver_system,
            fate_weaver_apply_system,
            upper_narrator_system,
            upper_narrator_apply_system,
            protagonist_system,
            protagonist_apply_system,
        )
            .chain(),
    );

    println!("== Akashic ECS ==");

    let mut frame = 0;
    let mut last_phase = None;
    loop {
        let (turn_index, phase) = {
            let turn_state = world.resource::<TurnState>();
            (turn_state.turn_index, turn_state.phase)
        };

        if phase == TurnPhase::Failed {
            eprintln!("demo 在 {} 轮, {:?}阶段失败", turn_index, phase);
            return;
        }
        if last_phase != Some(phase) {
            println!("");
            print_frame_status(&world, frame);
            println!("");
            print_result(&mut world);
            println!("------------------------------------------------------------------------");
            last_phase = Some(phase);
        }
        schedule.run(&mut world);

        frame += 1;
    }
}

fn print_frame_status(world: &World, frame: usize) {
    let turn_state = world.resource::<TurnState>();
    let task_results = world.resource::<TaskManager>().task_results_snapshot();
    let task_summary = if task_results.is_empty() {
        "无活动任务".to_string()
    } else {
        task_results
            .into_iter()
            .map(|(entity, result)| format!("{entity:?}:{:?}/{:?}", result.kind, result.status))
            .collect::<Vec<_>>()
            .join(", ")
    };

    println!(
        "[frame {frame:03}] phase={:?} turn_index={} active_turn_id={} tasks={}",
        turn_state.phase, turn_state.turn_index, turn_state.active_turn_id, task_summary
    );
}

fn print_result(world: &mut World) {
    let selected_action = world.resource::<ProtagonistAction>().0.clone();
    let world_snapshot = world.resource::<WorldSnapshot>().clone();
    let narrator_latest_msg = {
        let mut query = world.query::<&UpperNarrator>();
        query
            .iter(world)
            .next()
            .and_then(|narrator| narrator.context().print_latest_msg())
            .unwrap_or_default()
    };

    println!("world snapshot:");
    println!("{}", world_snapshot.to_ledger());
    println!("");
    println!("narrator latest msg:");
    println!("{}", narrator_latest_msg);
    println!("");
    println!("protagonist action:");
    println!("{selected_action}");
    println!("");
}

fn print_task_chunks(world: &World, task_chunk_offsets: &mut HashMap<Entity, usize>) {
    let mut task_results = world.resource::<TaskManager>().task_results_snapshot();
    if task_results.is_empty() {
        task_chunk_offsets.clear();
        return;
    }

    task_results.sort_by_key(|(entity, _)| entity.index());

    let active_entities = task_results
        .iter()
        .map(|(entity, _)| *entity)
        .collect::<Vec<_>>();
    task_chunk_offsets.retain(|entity, _| active_entities.contains(entity));

    for (entity, task_result) in task_results {
        let printed_count = task_chunk_offsets.get(&entity).copied().unwrap_or(0);
        if printed_count >= task_result.chunks.len() {
            continue;
        }

        for chunk in &task_result.chunks[printed_count..] {
            println!("[task {:?} {:?} chunk] {}", entity, task_result.kind, chunk);
        }

        task_chunk_offsets.insert(entity, task_result.chunks.len());
    }
}
