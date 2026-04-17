use std::{collections::HashMap, pin::pin, time::Instant};

use agent::{
    agent::{Context, call_model},
    models::ChatModel,
};
use bevy_ecs::{entity::Entity, resource::Resource};
use bevy_tasks::{AsyncComputeTaskPool, Task};
use chrono::Duration;

use crate::shared::build_chat_model;

#[derive(Resource)]
pub struct ModelResource {
    model: ChatModel,
}

impl ModelResource {
    pub fn new() -> Self {
        let model = build_chat_model();
        Self { model }
    }
    pub fn get_model(&self) -> ChatModel {
        self.model.clone()
    }
}

// 统一管理所有进行中的异步任务
#[derive(Resource)]
pub struct ModelTaskManager {
    model: ChatModel,
    /// 进行中的任务映射：Entity -> 任务元数据
    tasks: HashMap<Entity, LlmTaskMetadata>,
    /// 任务超时时间（超过此时长的任务将被清理）
    timeout: Duration,
}

/// 单个任务的元数据
pub struct LlmTaskMetadata {
    /// 异步任务句柄
    task: Task<LlmResponse>,
    /// 任务创建时间（用于超时处理）
    pub created_at: Instant,
    /// 关联的上下文数据（如对话目标实体）
    pub context: Context,
    // 中间结果
    pub chunks: Vec<String>,
    // 最终结果
    pub result: Option<LlmResponse>,
}

type LlmResponse = Result<String, String>;

impl ModelTaskManager {
    /// 创建一个新的任务管理器
    pub fn new(model: ChatModel) -> Self {
        Self {
            model,
            tasks: HashMap::new(),
            timeout: Duration::seconds(180), // 默认180分钟超时
        }
    }

    /// 自定义超时时间
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// 为指定实体创建一个 LLM 任务（发起异步调用）
    ///
    /// # 参数
    /// - `entity`: 发起调用的 Agent 实体
    /// - `context`: 任务附加上下文
    pub fn spawn_task(&mut self, entity: Entity, context: &Context) {
        let model = self.model.clone();
        // 获取后台线程池并提交异步任务
        let msgs = context.to_messages();
        let pool = AsyncComputeTaskPool::get();
        let stream = pin!(call_model(&model, &msgs, None));

        // 存储任务元数据
    }

    /// 轮询所有任务，收集已完成或超时的结果
    /// 注意：已完成或超时的任务会从管理器中移除。
    /// 解析 stream 的 event，更新 TextChunk -> chunks
    pub fn poll_tasks(&mut self) {}

    // 获取任务元信息
    // 可从中取 result 和 chunks 中间结果
    pub fn get_task(&self, entity: Entity) -> LlmTaskMetadata {
        todo!()
    }

    /// 删除实体的任务
    pub fn remove_tasks(&mut self, entity: Entity) {
        self.tasks.remove(&entity);
    }

    /// 检查指定实体是否有进行中的任务
    pub fn has_pending(&self, entity: Entity) -> bool {
        self.tasks.contains_key(&entity)
    }

    /// 获取当前待处理任务的数量
    pub fn pending_count(&self) -> usize {
        self.tasks.len()
    }
}
