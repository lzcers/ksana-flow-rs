use std::pin::pin;

use crate::{
    agent_component::FateWeaver,
    fate_weaver::component::{FateLine, FateWeaverContext},
    model_resource::{ModelResource, ModelTaskManager},
};
use agent::{agent::call_model, models::ChatCapability};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, Query, Res, ResMut},
};
use bevy_tasks::AsyncComputeTaskPool;

#[derive(Component)]
struct ChatStreamTask;

fn task_system() {}

fn parse_fate_node() {}
// 命运编织系统
fn fate_Weaving_system(
    mut commands: Commands,
    mut query: Query<(Entity, &FateWeaver, &mut FateLine, &mut FateWeaverContext)>,
    mut task_manager: ResMut<ModelTaskManager>,
) {
    for (entity, fate_weaver, fate_line, fate_weaver_context) in query.iter_mut() {
        if fate_line.current_round >= fate_line.max_rounds {
            println!("FateWeaver reached max rounds: {}", fate_line.max_rounds);
            return;
        }
        // todo:
        // 1. 如果当前没有进行中的 call model 任务（推演下一轮）
        // 那么提交 call model 任务进行下一轮剧情推演
        if !task_manager.has_pending(entity) {
            let ctx = fate_weaver_context.get_context();
            task_manager.spawn_task(entity, ctx);
            return;
        }

        // 2. 如果当前有进行中的 call model 任务（推演下一轮）
        // 那么等待任务完成，拿到完成后的结果，
        // 如果有 choice，发送 choice 消息给主角系统，请求用户输入
        // 监听主角发送的答复，进行下一轮推演
        // 如果没有 choice，发送消息给叙事者系统，请求生成事实清单
    }
}

// 主角系统
// todo:
// 1.监听 fate_weaver 系统发送的 choice 消息
// 并根据 choice 消息，生成对应的 ProtagonistAction 消息
// 2. 更新并记录主角做出的决策与状态
fn protagonist_system(model: Res<ModelResource>) {}

// 叙事者系统
// 查询命运编织者和主角的状态，编写故事
// 如果有故事编写中，那么等待任务完成，拿到完成后的结果
// 如果没有故事生成中，判断判断是否存在未生成故事的轮次， 如果存在，那么提交 call model 任务进行下一轮剧情推演
// 如果不存在，那么不做任何行动
fn narration_system(query: Query<(&FateLine, &FateWeaverContext)>, model: Res<ModelResource>) {}
