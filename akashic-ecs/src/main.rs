mod components;
mod profile;
mod prompts;
mod resources;
mod systems;
mod turn_messages;
mod utils;

use std::time::Duration;

use crate::{
    components::fate_weaver::FateWeaver,
    components::protagonist::Protagonist,
    components::upper_narrator::UpperNarrator,
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{
        task_manager::TaskManager,
        turn_state::{TurnPhase, TurnState},
        world_state::WorldState,
    },
    systems::fate_weaver_sys::{fate_result_apply_system, fate_weaver_system},
    systems::protagonist_sys::{protagonist_result_apply_system, protagonist_system},
    systems::task_sys::task_system,
    systems::turn_orchestrator_sys::turn_orchestrator_system,
    systems::upper_narrator_sys::{upper_narrator_result_apply_system, upper_narrator_system},
    turn_messages::{TurnEvent, register_turn_messages},
    utils::build_chat_model,
};
use bevy_ecs::{
    message::{MessageReader, message_update_system},
    schedule::{IntoScheduleConfigs, Schedule},
    world::World,
};

const MAX_TICKS: usize = 2000;
const TICK_DELAY_MS: u64 = 100;
const MAX_FAILURES: usize = 5;

fn main() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("创建 Tokio runtime 失败");
    runtime.block_on(async_main());
}

async fn async_main() {
    let mut world = World::new();
    register_turn_messages(&mut world);
    world.insert_resource(TaskManager::new(build_chat_model()));
    world.insert_resource(TurnState::default());
    world.insert_resource(WorldState::default());

    let _fate_weaver = world.spawn((FateWeaver::new(
        DEFAULT_WORLD_PROFILE,
        DEFAULT_PROTAGONIST_PROFILE,
    ),));
    let _protagonist = world.spawn((Protagonist::new(DEFAULT_PROTAGONIST_PROFILE),));
    let _upper_narrator = world.spawn((UpperNarrator::new(),));

    world.resource_mut::<TurnState>().start_turn(1);

    let mut schedule = build_schedule();
    let target_turns = demo_target_turns();
    println!("启动 akashic-ecs demo。");
    match target_turns {
        Some(turns) => println!("目标完成 {} 个回合后退出。", turns),
        None => println!("默认持续运行；可通过 AKASHIC_DEMO_TURNS 设置退出回合数。"),
    }

    let mut last_phase = None;
    let mut last_turn = None;
    let mut failure_count = 0usize;

    for tick in 0..MAX_TICKS {
        schedule.run(&mut world);

        let (turn_index, active_turn_id, phase) = {
            let turn_state = world.resource::<TurnState>();
            (
                turn_state.turn_index,
                turn_state.active_turn_id,
                turn_state.phase,
            )
        };

        if last_turn != Some(turn_index) || last_phase != Some(phase) {
            println!(
                "[tick {:03}] turn_index={} active_turn={} phase={:?}",
                tick, turn_index, active_turn_id, phase
            );
            last_turn = Some(turn_index);
            last_phase = Some(phase);
        }

        let outcome = if target_turns.is_some_and(|target| turn_index >= target) {
            Some(Ok("已完成目标回合数"))
        } else if phase == TurnPhase::Failed {
            failure_count += 1;
            println!("检测到失败，自动重置并继续下一轮演示。");
            restart_demo_turn(&mut world);

            if failure_count >= MAX_FAILURES {
                Some(Err("连续失败次数过多，停止 demo"))
            } else {
                None
            }
        } else {
            failure_count = 0;
            None
        };

        if let Some(result) = outcome {
            match result {
                Ok(message) => {
                    print_final_summary(&world);
                    println!("{message}");
                    return;
                }
                Err(message) => {
                    print_final_summary(&world);
                    eprintln!("{message}");
                    std::process::exit(1);
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(TICK_DELAY_MS)).await;
    }

    eprintln!("在 {} 个 tick 内未完成目标回合，停止运行。", MAX_TICKS);
    print_final_summary(&world);
    std::process::exit(1);
}

fn build_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            message_update_system,
            fate_weaver_system,
            protagonist_system,
            upper_narrator_system,
            task_system,
            fate_result_apply_system,
            protagonist_result_apply_system,
            upper_narrator_result_apply_system,
            demo_event_logger_system,
            turn_orchestrator_system,
        )
            .chain(),
    );
    schedule
}

fn demo_target_turns() -> Option<u64> {
    std::env::var("AKASHIC_DEMO_TURNS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|turns| *turns > 0)
}

fn restart_demo_turn(world: &mut World) {
    let next_turn_id = {
        let turn_state = world.resource::<TurnState>();
        turn_state.active_turn_id + 1
    };

    let mut turn_state = world.resource_mut::<TurnState>();
    turn_state.reset(Some(next_turn_id));
    turn_state.start_turn(next_turn_id);
}

fn demo_event_logger_system(mut event_reader: MessageReader<TurnEvent>) {
    for event in event_reader.read() {
        match event {
            TurnEvent::SceneFateGenerated {
                turn_id,
                scene_facts,
            } => {
                println!("\n=== Turn {} / Fate Scene ===\n{}\n", turn_id, scene_facts);
            }
            TurnEvent::SceneNarrationGenerated {
                turn_id,
                scene_text,
            } => {
                println!(
                    "\n=== Turn {} / Narrator Scene ===\n{}\n",
                    turn_id, scene_text
                );
            }
            TurnEvent::ProtagonistActionGenerated {
                turn_id,
                action_text,
            } => {
                println!(
                    "\n=== Turn {} / Protagonist ===\n{}\n",
                    turn_id, action_text
                );
            }
            TurnEvent::ConsequenceFateGenerated {
                turn_id,
                consequence_facts,
            } => {
                println!(
                    "\n=== Turn {} / Fate Consequence ===\n{}\n",
                    turn_id, consequence_facts
                );
            }
            TurnEvent::StoryNarrationGenerated {
                turn_id,
                story_text,
            } => {
                println!(
                    "\n=== Turn {} / Narrator Story ===\n{}\n",
                    turn_id, story_text
                );
            }
            TurnEvent::TaskFailed {
                turn_id,
                stage,
                entity,
                message,
            } => {
                eprintln!(
                    "\n=== Turn {} / Task Failed ===\nstage={:?} entity={:?}\n{}\n",
                    turn_id, stage, entity, message
                );
            }
        }
    }
}

fn print_final_summary(world: &World) {
    let turn_state = world.resource::<TurnState>();
    let world_state = world.resource::<WorldState>();

    println!("最终 phase={:?}", turn_state.phase);
    println!("已完成 turn_index={}", turn_state.turn_index);

    let latest_history = world_state.latest_history_entry_text();
    if latest_history != "暂无最新世界变化" {
        println!("最新世界变化：\n{}", latest_history);
    }

    if !turn_state.latest_protagonist_action.trim().is_empty() {
        println!("主角行动：{}", turn_state.latest_protagonist_action.trim());
    }

    if !turn_state.latest_broadcast_summary.trim().is_empty() {
        println!(
            "最新广播摘要：\n{}",
            turn_state.latest_broadcast_summary.trim()
        );
    }
}
