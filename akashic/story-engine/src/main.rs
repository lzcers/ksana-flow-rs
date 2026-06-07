use std::{
    collections::{HashMap, VecDeque},
    env,
    error::Error,
    io::{self, Write},
    time::Duration,
};

use story_engine::{
    AkashicEngine,
    resources::{
        agent_task::{TaskChunkKind, TaskKind, TaskStatus, TaskUpdate},
        export::TaskEvent,
        protagonist_action::{PlayerActionInput, PlayerActionType},
        turn_state::TurnPhase,
    },
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::dotenv().ok();

    if env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("请先设置 DEEPSEEK_API_KEY，再运行 `cargo run -p story_engine`");
        return Ok(());
    }

    let max_turns = env_usize("AKASHIC_STORY_ENGINE_MAX_TURNS", 200);
    let poll_interval = Duration::from_millis(env_u64("AKASHIC_STORY_ENGINE_POLL_MS", 100));

    let engine = AkashicEngine::new();
    let session = engine.create_default_session("story-engine-main").await?;
    let mut task_events = session.subscribe_events();
    tokio::spawn(async move {
        let mut streams = OrderedTaskStreams::default();

        while let Ok(TaskEvent::TaskUpdated { update }) = task_events.recv().await {
            streams.handle(update);
        }
    });
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
                print_failure(&snapshot);
                return Err(format!("故事引擎在第 {} 轮失败", snapshot.active_turn_id).into());
            }
            TurnPhase::Ended => {
                println!(
                    "[api] story ended: turn_index={} active_turn_id={}",
                    snapshot.turn_index, snapshot.active_turn_id
                );
                return Ok(());
            }
            TurnPhase::AwaitingPlayer => {
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

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TaskStreamKey {
    entity: String,
    kind: TaskKind,
}

#[derive(Default)]
struct BufferedTaskStream {
    chunks: VecDeque<(TaskChunkKind, String)>,
    printed_kind: Option<TaskChunkKind>,
    completed: bool,
}

#[derive(Default)]
struct OrderedTaskStreams {
    order: VecDeque<TaskStreamKey>,
    streams: HashMap<TaskStreamKey, BufferedTaskStream>,
}

impl OrderedTaskStreams {
    fn handle(&mut self, update: TaskUpdate) {
        let key = TaskStreamKey {
            entity: update.entity,
            kind: update.kind,
        };

        if !self.streams.contains_key(&key) {
            self.order.push_back(key.clone());
            self.streams
                .insert(key.clone(), BufferedTaskStream::default());
        }

        let stream = self
            .streams
            .get_mut(&key)
            .expect("task stream must exist after insertion");
        if let (Some(kind), Some(chunk)) = (update.chunk_kind, update.chunk) {
            stream.chunks.push_back((kind, chunk));
        }
        stream.completed = matches!(update.status, TaskStatus::Done | TaskStatus::Error);

        self.flush();
    }

    fn flush(&mut self) {
        while let Some(key) = self.order.front().cloned() {
            let Some(stream) = self.streams.get_mut(&key) else {
                self.order.pop_front();
                continue;
            };

            while let Some((chunk_kind, chunk)) = stream.chunks.pop_front() {
                if stream.printed_kind != Some(chunk_kind) {
                    if stream.printed_kind.is_some() {
                        println!();
                    }
                    println!("\n[stream {:?} {} {:?}]", key.kind, key.entity, chunk_kind);
                    stream.printed_kind = Some(chunk_kind);
                }
                print!("{chunk}");
                let _ = io::stdout().flush();
            }

            if !stream.completed {
                break;
            }

            if stream.printed_kind.is_some() {
                println!();
            }
            self.streams.remove(&key);
            self.order.pop_front();
        }
    }
}

fn print_frame_status(frame: usize, snapshot: &story_engine::Session) {
    println!(
        "[frame {frame:03}] phase={:?} turn_index={} active_turn_id={}",
        snapshot.phase, snapshot.turn_index, snapshot.active_turn_id
    );
}

fn print_failure(snapshot: &story_engine::Session) {
    for task in snapshot
        .tasks
        .iter()
        .filter(|task| task.status == story_engine::resources::agent_task::TaskStatus::Error)
    {
        eprintln!(
            "[error] agent task {:?} ({}) failed: {}",
            task.kind,
            task.entity,
            task.error.as_deref().unwrap_or("unknown error")
        );
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
