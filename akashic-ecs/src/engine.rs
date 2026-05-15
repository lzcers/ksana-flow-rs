use bevy_ecs::{
    message::{Messages, message_update_system},
    prelude::*,
    schedule::Schedule,
};
use serde::Serialize;
use tokio::sync::broadcast;

use crate::{
    components::{
        fate_weaver::FateWeaver, protagonist::Protagonist, upper_narrator::UpperNarrator,
    },
    profile::{DEFAULT_PROTAGONIST_PROFILE, DEFAULT_WORLD_PROFILE},
    resources::{
        export::{ExportHandle, ExportState, TaskEvent, TaskView},
        player_input::{PlayerInbox, PlayerInputConfig},
        protagonist_action::{PendingProtagonistChoice, ProtagonistDecisionState},
        task_manager::{TaskManager, TaskStatus},
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
    turn_messages::{PlayerCommand, TurnControl, TurnEvent},
    utils::build_chat_model,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub phase: TurnPhase,                       // 阶段
    pub turn_index: u64,                        // 轮次
    pub active_turn_id: u64,                    // 当前轮次ID
    pub current_task: Option<TaskView>,         // 当前任务
    pub tasks: Vec<TaskView>,                   // 任务列表
    pub world_snapshot: WorldSnapshot,          // 世界快照
    pub latest_narration: String,               // 最新叙事
    pub current_protagonist_action: String,     // 当前 protagonist 动作
    pub choices: Vec<PendingProtagonistChoice>, // 待处理选择
}

pub struct AkashicSessionEngine {
    world: World,
    schedule: Schedule,
    export_handle: ExportHandle,
}

impl AkashicSessionEngine {
    // 创建实例
    pub fn new() -> Self {
        Self::new_with_profiles(DEFAULT_WORLD_PROFILE, DEFAULT_PROTAGONIST_PROFILE)
    }

    pub fn new_with_profiles(world_profile: &str, protagonist_profile: &str) -> Self {
        let (world, export_handle) = build_world(world_profile, protagonist_profile);
        Self {
            world,
            schedule: build_schedule(),
            export_handle,
        }
    }

    // 获取当前会话状态
    pub fn get_game_session(&mut self) -> Session {
        let export_snapshot = self.export_handle.current_snapshot();
        let decision_state = self.world.resource::<ProtagonistDecisionState>().clone();
        let latest_narration = self.latest_narration();
        let current_task = select_current_task(&export_snapshot.tasks);

        Session {
            phase: export_snapshot.phase,
            turn_index: export_snapshot.turn_index,
            active_turn_id: export_snapshot.active_turn_id,
            tasks: export_snapshot.tasks,
            world_snapshot: export_snapshot.world,
            current_protagonist_action: decision_state.committed_action().to_string(),
            choices: decision_state.choices().to_vec(),
            latest_narration,
            current_task,
        }
    }

    // 订阅当前会话事件
    pub fn subscribe_events(&self) -> broadcast::Receiver<TaskEvent> {
        self.export_handle.subscribe_events()
    }

    // 继续轮次
    pub async fn continue_turn(&mut self) -> Result<Session, String> {
        match self.world.resource::<TurnState>().phase {
            TurnPhase::Idle => {
                self.send_turn_control_command(TurnControl::StartTurn {
                    turn_id: self.world.resource::<TurnState>().turn_index + 1,
                });
            }
            TurnPhase::TurnFinished => {
                let finished_turn_id = self.world.resource::<TurnState>().active_turn_id;
                self.send_turn_control_command(TurnControl::ContinueTurn {
                    turn_id: finished_turn_id,
                });
            }
            _ => {}
        }

        self.drive_until(|snapshot| snapshot.phase == TurnPhase::AwaitingPlayerChoice)
            .await
    }

    // 提交选择
    pub async fn submit_choice(&mut self, choice_id: &str) -> Result<Session, String> {
        let turn_id = self.world.resource::<TurnState>().active_turn_id;
        self.world
            .resource_mut::<PlayerInbox>()
            .push(PlayerCommand::SubmitChoice {
                turn_id,
                choice_id: choice_id.to_string(),
            });

        self.drive_until(|snapshot| snapshot.phase == TurnPhase::TurnFinished)
            .await
    }

    async fn drive_until<P>(&mut self, should_stop: P) -> Result<Session, String>
    where
        P: Fn(&Session) -> bool,
    {
        loop {
            self.run_one_frame()?;
            let snapshot = self.get_game_session();

            if should_stop(&snapshot) {
                return Ok(snapshot);
            }
        }
    }

    fn run_one_frame(&mut self) -> Result<(), String> {
        self.schedule.run(&mut self.world);
        self.ensure_not_failed()
    }

    fn ensure_not_failed(&self) -> Result<(), String> {
        let turn_state = self.world.resource::<TurnState>();
        if turn_state.phase == TurnPhase::Failed {
            return Err("游戏流程推进失败，请检查任务流或模型输出".to_string());
        }
        Ok(())
    }

    fn latest_narration(&mut self) -> String {
        let mut query = self.world.query::<&UpperNarrator>();
        query
            .iter(&self.world)
            .next()
            .and_then(|narrator| narrator.context().print_latest_msg())
            .unwrap_or_default()
    }

    fn send_turn_control_command(&mut self, control: TurnControl) {
        self.world
            .resource_mut::<Messages<TurnControl>>()
            .write(control);
    }
}

fn build_world(world_profile: &str, protagonist_profile: &str) -> (World, ExportHandle) {
    let mut world = World::new();

    world.init_resource::<WorldSnapshot>();
    world.init_resource::<TurnState>();
    world.init_resource::<PlayerInbox>();
    world.insert_resource(PlayerInputConfig::wait_for_user());
    world.init_resource::<ProtagonistDecisionState>();
    world.init_resource::<Messages<TurnEvent>>();
    world.init_resource::<Messages<TurnControl>>();
    world.insert_resource(TaskManager::new(build_chat_model()));
    let (export_state, export_handle) = ExportState::new_with_handle();
    world.insert_resource(export_state);

    world.spawn(FateWeaver::new(world_profile, protagonist_profile));
    world.spawn(UpperNarrator::new(world_profile, protagonist_profile));
    world.spawn(Protagonist::new(world_profile, protagonist_profile));

    (world, export_handle)
}

fn build_schedule() -> Schedule {
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
    schedule
}

fn select_current_task(tasks: &[TaskView]) -> Option<TaskView> {
    tasks
        .iter()
        .find(|task| matches!(task.status, TaskStatus::Running))
        .or_else(|| {
            tasks
                .iter()
                .find(|task| matches!(task.status, TaskStatus::Pending))
        })
        .cloned()
}
