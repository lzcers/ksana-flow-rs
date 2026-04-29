use std::time::Duration;

use akashic_ecs::{AkashicRuntime, RuntimeSummary, resources::turn_state::TurnPhase};

const MAX_TICKS: usize = 100000;
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
    let mut runtime = AkashicRuntime::new();
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
        runtime.tick();
        let turn_index = runtime.turn_index();
        let active_turn_id = runtime.active_turn_id();
        let phase = runtime.phase();

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
            runtime.restart_turn();

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
                    print_final_summary(runtime.summary());
                    println!("{message}");
                    return;
                }
                Err(message) => {
                    print_final_summary(runtime.summary());
                    eprintln!("{message}");
                    std::process::exit(1);
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(TICK_DELAY_MS)).await;
    }

    eprintln!("在 {} 个 tick 内未完成目标回合，停止运行。", MAX_TICKS);
    print_final_summary(runtime.summary());
    std::process::exit(1);
}

fn demo_target_turns() -> Option<u64> {
    std::env::var("AKASHIC_DEMO_TURNS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|turns| *turns > 0)
}

fn print_final_summary(summary: RuntimeSummary) {
    println!("最终 phase={:?}", summary.phase);
    println!("已完成 turn_index={}", summary.turn_index);

    let latest_history = summary.latest_history.trim();
    if latest_history != "暂无最新世界变化" {
        println!("最新世界变化：\n{latest_history}");
    }

    if !summary.latest_protagonist_action.is_empty() {
        println!("主角行动：{}", summary.latest_protagonist_action);
    }

    if !summary.latest_broadcast_summary.is_empty() {
        println!("最新广播摘要：\n{}", summary.latest_broadcast_summary);
    }
}
