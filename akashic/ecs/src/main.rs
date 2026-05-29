use akashic_ecs::{
    components::{
        fate_weaver::FateWeaver, protagonist::Protagonist, upper_narrator::UpperNarrator,
    },
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{
        export::ExportState,
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::ProtagonistDecisionState,
        task_manager::TaskManager,
        turn_state::{TurnPhase, TurnState},
        world_snapshot::WorldSnapshot,
    },
    systems::{
        export_sys::export_system,
        fate_weaver_sys::{fate_weaver_apply_system, fate_weaver_system},
        player_input_sys::player_input_system,
        protagonist_sys::{protagonist_apply_system, protagonist_system},
        task_sys::task_system,
        turn_orchestrator_sys::turn_orchestrator_system,
        upper_narrator_sys::{upper_narrator_apply_system, upper_narrator_system},
    },
    turn_messages::{TurnControl, TurnEvent},
    utils::build_chat_model,
};
use bevy_ecs::{
    message::{Messages, message_update_system},
    prelude::*,
    schedule::Schedule,
};
use std::{env, fs::OpenOptions, io::Write};

const FATE_CONTEXT_OUTPUT_FILE_PATH: &str = "fate_weaver_context.txt";
const NARRATOR_CONTEXT_OUTPUT_FILE_PATH: &str = "upper_narrator_context.txt";
const PROTAGONIST_CONTEXT_OUTPUT_FILE_PATH: &str = "protagonist_context.txt";

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    if env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("请先设置 DEEPSEEK_API_KEY，再运行 `cargo run -p akashic-ecs` ");
        return;
    }

    // Build world
    let mut world = World::new();

    // 资源初始化
    world.init_resource::<WorldSnapshot>();
    world.init_resource::<TurnState>();
    world.init_resource::<PlayerInbox>();
    world.insert_resource(PlayerInputConfig::auto_select_first());
    world.init_resource::<ProtagonistDecisionState>();
    world.init_resource::<Messages<TurnEvent>>();
    world.init_resource::<Messages<TurnControl>>();
    world.insert_resource(TaskManager::new(build_chat_model()));
    world.insert_resource(ExportState::new());

    // 实体初始化
    // 命运编织者，负责故事世界维护
    world.spawn(FateWeaver::new(
        DEFAULT_WORLD_PROFILE,
        DEFAULT_PROTAGONIST_PROFILE,
    ));
    // 上层叙事者，负责故事世界的文学化描述
    world.spawn(UpperNarrator::new(
        DEFAULT_WORLD_PROFILE,
        DEFAULT_PROTAGONIST_PROFILE,
    ));
    // 主角，负责执行任务
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
            player_input_system,
            turn_orchestrator_system,
            fate_weaver_system,
            fate_weaver_apply_system,
            upper_narrator_system,
            upper_narrator_apply_system,
            protagonist_system,
            protagonist_apply_system,
            export_system,
        )
            .chain(),
    );

    println!("== Akashic ECS ==");
    println!("[api] akashic-ecs main loop started");

    let mut frame = 0;
    let mut last_reported_completed_turn = None;
    loop {
        let (turn_index, phase) = {
            let turn_state = world.resource::<TurnState>();
            (turn_state.turn_index, turn_state.phase)
        };

        if phase == TurnPhase::Failed {
            eprintln!("在 {} 轮发生错误，失败", turn_index);
            return;
        }
        if phase == TurnPhase::Idle {
            println!("");
            print_frame_status(&world, frame);
            println!("");
            print_result(&mut world);
            println!("------------------------------------------------------------------------");
            world
                .resource_mut::<Messages<TurnControl>>()
                .write(TurnControl::StartNextTurn);
        }
        if phase == TurnPhase::TurnComplete && last_reported_completed_turn != Some(turn_index) {
            println!("");
            print_frame_status(&world, frame);
            println!("");
            print_result(&mut world);
            println!(
                "[api] continue turn requested: turn_id={} phase={:?}",
                turn_index, phase
            );
            world
                .resource_mut::<Messages<TurnControl>>()
                .write(TurnControl::StartNextTurn);
            last_reported_completed_turn = Some(turn_index);
        }
        if phase == TurnPhase::StoryEnded {
            println!("");
            print_frame_status(&world, frame);
            println!("");
            print_result(&mut world);
            println!("[api] story ended: turn_id={} phase={:?}", turn_index, phase);
            return;
        }
        schedule.run(&mut world);
        frame += 1;
    }
}

fn print_frame_status(world: &World, frame: usize) {
    let turn_state = world.resource::<TurnState>();
    let task_results = world
        .resource::<TaskManager>()
        .results
        .iter()
        .map(|(entity, result)| (*entity, result.clone()))
        .collect::<Vec<_>>();
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
    let turn_index = world.resource::<TurnState>().turn_index;
    let selected_action = world
        .resource::<ProtagonistDecisionState>()
        .committed_action()
        .to_string();
    let narrator_latest_msg = {
        let mut query = world.query::<&UpperNarrator>();
        query
            .iter(world)
            .next()
            .and_then(|narrator| narrator.context().print_latest_msg())
            .unwrap_or_default()
    };

    println!(
        "[api] output result: turn_index={} narrator_chars={} action_chars={}",
        turn_index,
        narrator_latest_msg.chars().count(),
        selected_action.chars().count()
    );

    output_context_to_file(world, turn_index);
}

fn output_context_to_file(world: &mut World, turn_index: u64) {
    let fate_context = {
        let mut query = world.query::<&FateWeaver>();
        query
            .iter(world)
            .next()
            .map(|fate_weaver| fate_weaver.context().print())
            .unwrap_or_default()
    };
    let narrator_context = {
        let mut query = world.query::<&UpperNarrator>();
        query
            .iter(world)
            .next()
            .map(|upper_narrator| upper_narrator.context().print())
            .unwrap_or_default()
    };
    let protagonist_context = {
        let mut query = world.query::<&Protagonist>();
        query
            .iter(world)
            .next()
            .map(|protagonist| protagonist.context().print())
            .unwrap_or_default()
    };

    write_to_file(
        FATE_CONTEXT_OUTPUT_FILE_PATH,
        &format!("===== turn {} =====\n{}\n", turn_index, fate_context),
    );
    write_to_file(
        NARRATOR_CONTEXT_OUTPUT_FILE_PATH,
        &format!("===== turn {} =====\n{}\n", turn_index, narrator_context),
    );
    write_to_file(
        PROTAGONIST_CONTEXT_OUTPUT_FILE_PATH,
        &format!("===== turn {} =====\n{}\n", turn_index, protagonist_context),
    );
}

fn write_to_file(path: &str, content: &str) {
    let write_result = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .and_then(|mut file| file.write_all(content.as_bytes()));

    if let Err(err) = write_result {
        eprintln!("写入输出文件失败 ({}): {err}", path);
    }
}
