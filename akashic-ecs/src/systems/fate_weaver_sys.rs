use bevy_ecs::{
    entity::Entity,
    message::MessageReader,
    system::{Query, ResMut},
};

use crate::{
    components::fate_weaver::FateWeaver,
    resources::task_manager::{TaskKind, TaskManager},
    turn_messages::TurnCommand,
};

// FateWeaver 只消费调度消息并发起任务。
pub fn fate_weaver_system(
    query: Query<(Entity, &FateWeaver)>,
    mut command_reader: MessageReader<TurnCommand>,
    mut task_manager: ResMut<TaskManager>,
) {
    let Ok((entity, fate_weaver)) = query.single() else {
        return;
    };

    for command in command_reader.read() {
        if let TurnCommand::RequestFate { .. } = command {
            if task_manager.task_status(entity).is_none() {
                task_manager.spawn_task(entity, TaskKind::FateWeaving, fate_weaver.get_context());
            }
        }
    }
}
