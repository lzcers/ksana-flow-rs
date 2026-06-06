use std::{env, error::Error, fs::OpenOptions, io::Write, time::Duration};

use story_engine::{
    AkashicEngine, SessionArchiveState,
    resources::{
        protagonist_action::{PlayerActionInput, PlayerActionType},
        turn_state::TurnPhase,
    },
};

const FATE_CONTEXT_OUTPUT_FILE_PATH: &str = "story_engine_fate_weaver_context.txt";
const NARRATOR_CONTEXT_OUTPUT_FILE_PATH: &str = "story_engine_upper_narrator_context.txt";
const PROTAGONIST_CONTEXT_OUTPUT_FILE_PATH: &str = "story_engine_protagonist_context.txt";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    if env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("请先设置 DEEPSEEK_API_KEY，再运行 `cargo run -p story_engine`");
        return Ok(());
    }

    let max_turns = env_usize("AKASHIC_STORY_ENGINE_MAX_TURNS", 1);
    let poll_interval = Duration::from_millis(env_u64("AKASHIC_STORY_ENGINE_POLL_MS", 100));

    let engine = AkashicEngine::new();
    let session = engine.create_default_session("story-engine-main").await?;
    session.start_next_turn()?;

    println!("== Akashic Story Engine ==");
    println!(
        "[api] story_engine main loop started, max_turns={}, poll_ms={}",
        max_turns,
        poll_interval.as_millis()
    );

    let mut frame = 0;
    let mut completed_turns = 0;
    let mut last_reported_phase = None;
    let mut last_completed_turn = None;

    loop {
        let snapshot = session.get_game_session();
        if last_reported_phase != Some(snapshot.phase) {
            print_frame_status(frame, &snapshot);
            last_reported_phase = Some(snapshot.phase);
        }

        match snapshot.phase {
            TurnPhase::Failed => {
                print_frame_status(frame, &snapshot);
                return Err(format!("故事引擎在第 {} 轮失败", snapshot.active_turn_id).into());
            }
            TurnPhase::Ended => {
                print_frame_status(frame, &snapshot);
                print_result(&session.export_archive_state().await?);
                println!(
                    "[api] story ended: turn_index={} active_turn_id={}",
                    snapshot.turn_index, snapshot.active_turn_id
                );
                return Ok(());
            }
            TurnPhase::AwaitingPlayer => {
                print_result(&session.export_archive_state().await?);
                let Some(choice) = snapshot.choices.first() else {
                    return Err("故事引擎进入 AwaitingPlayer，但没有可选行动".into());
                };
                println!(
                    "[api] auto selected choice: {} -> {}",
                    choice.id, choice.option.action
                );
                session.submit_player_action(PlayerActionInput {
                    r#type: PlayerActionType::SelectedOption,
                    action: choice.option.action.clone(),
                })?;
            }
            TurnPhase::TurnCompleted if last_completed_turn != Some(snapshot.turn_index) => {
                completed_turns += 1;
                last_completed_turn = Some(snapshot.turn_index);
                print_result(&session.export_archive_state().await?);
                println!(
                    "[api] turn complete: turn_index={} completed_turns={}/{}",
                    snapshot.turn_index, completed_turns, max_turns
                );
                if completed_turns >= max_turns {
                    println!("[api] reached max turns, exiting");
                    return Ok(());
                }
                session.start_next_turn()?;
            }
            TurnPhase::Idle => {
                session.start_next_turn()?;
            }
            _ => {}
        }

        frame += 1;
        tokio::time::sleep(poll_interval).await;
    }
}

fn print_frame_status(frame: usize, snapshot: &story_engine::Session) {
    println!(
        "[frame {frame:03}] phase={:?} turn_index={} active_turn_id={} narration_chars={} choices={}",
        snapshot.phase,
        snapshot.turn_index,
        snapshot.active_turn_id,
        snapshot.latest_narration.chars().count(),
        snapshot.choices.len()
    );
}

fn print_result(archive: &SessionArchiveState) {
    println!(
        "[api] output result: turn_index={} phase={:?} narrator_context_chars={} protagonist_action_chars={}",
        archive.turn_index,
        archive.phase,
        archive.upper_narrator_context.print().chars().count(),
        archive.committed_action.chars().count()
    );

    output_context_to_file(archive);
}

fn output_context_to_file(archive: &SessionArchiveState) {
    write_to_file(
        FATE_CONTEXT_OUTPUT_FILE_PATH,
        &format!(
            "===== turn {} =====\n{}\n",
            archive.turn_index,
            archive.fate_weaver_context.print()
        ),
    );
    write_to_file(
        NARRATOR_CONTEXT_OUTPUT_FILE_PATH,
        &format!(
            "===== turn {} =====\n{}\n",
            archive.turn_index,
            archive.upper_narrator_context.print()
        ),
    );
    write_to_file(
        PROTAGONIST_CONTEXT_OUTPUT_FILE_PATH,
        &format!(
            "===== turn {} =====\n{}\n",
            archive.turn_index,
            archive.protagonist_context.print()
        ),
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

fn env_usize(key: &str, default: usize) -> usize {
    env::var(key)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}

fn env_u64(key: &str, default: u64) -> u64 {
    env::var(key)
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
