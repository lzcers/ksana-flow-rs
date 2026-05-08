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
    utils::build_chat_model,
};
use bevy_ecs::{prelude::*, schedule::Schedule};
use std::{env, time::Duration};
use tokio::time::sleep;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    dotenv::dotenv().ok();

    if env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("请先设置 DEEPSEEK_API_KEY，再运行 `cargo run -p akashic-ecs` 体验最小 demo。");
        return;
    }

    // build world
    let mut world = World::new();

    // 资源初始化
    world.insert_resource(TurnState::default());
    world.insert_resource(ProtagonistAction::default());
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
    loop {
        schedule.run(&mut world);
        print_frame_status(&world, frame);

        let (phase, turn_index) = {
            let turn_state = world.resource::<TurnState>();
            (turn_state.phase, turn_state.turn_index)
        };

        if phase == TurnPhase::Failed {
            eprintln!("demo 在第 {frame} 帧进入失败态。");
            return;
        }
        if turn_index >= 1 {
            print_result(&world);
            return;
        }

        sleep(Duration::from_millis(200)).await;
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

fn print_result(world: &World) {
    let selected_action = world.resource::<ProtagonistAction>().0.clone();
    let world_snapshot = world.resource::<WorldSnapshot>().clone();

    println!();
    println!("主角最终选择：{selected_action}");
    println!();
    println!("{}", world_snapshot.to_ledger());
}
