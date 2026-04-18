use std::{
    collections::HashMap,
    pin::Pin,
    sync::Mutex,
    task::{Context as PollContext, Poll},
    time::Instant,
};

use crate::shared::build_chat_model;
use agent::{
    agent::{CallModelEvent, Context, call_model},
    models::ChatModel,
};
use async_stream::stream;
use bevy_ecs::{entity::Entity, resource::Resource};
use chrono::Duration;
use futures::{Stream, StreamExt, task::noop_waker_ref};

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
    /// 已完成或已超时的任务快照，保留直到显式删除
    completed_tasks: HashMap<Entity, LlmTaskSnapshot>,
    /// 任务超时时间（超过此时长的任务将被清理）
    timeout: Duration,
}

/// 单个任务的元数据
pub struct LlmTaskMetadata {
    /// 未完成的模型事件流，由 poll_tasks 推进消费
    stream: Mutex<LlmEventStream>,
    /// 任务创建时间（用于超时处理）
    pub created_at: Instant,
    /// 关联的上下文数据（如对话目标实体）
    pub context: Context,
    // 中间结果
    pub chunks: Vec<String>,
    // 最终结果
    pub result: Option<LlmResponse>,
    // 是否已完成
    finished: bool,
}

/// 对外暴露的任务快照
pub struct LlmTaskSnapshot {
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
type LlmEventStream = Pin<Box<dyn Stream<Item = CallModelEvent> + Send>>;
impl LlmTaskMetadata {
    fn new(stream: LlmEventStream, context: Context) -> Self {
        Self {
            stream: Mutex::new(stream),
            created_at: Instant::now(),
            context,
            chunks: Vec::new(),
            result: None,
            finished: false,
        }
    }

    fn snapshot(&self) -> LlmTaskSnapshot {
        LlmTaskSnapshot {
            created_at: self.created_at,
            context: self.context.clone(),
            chunks: self.chunks.clone(),
            result: self.result.clone(),
        }
    }

    fn poll_stream(&mut self) {
        let mut stream = self.stream.lock().expect("llm task stream poisoned");
        let waker = noop_waker_ref();
        let mut cx = PollContext::from_waker(waker);

        loop {
            match stream.as_mut().poll_next(&mut cx) {
                Poll::Ready(Some(CallModelEvent::TextChunk(content))) => {
                    self.chunks.push(content);
                }
                Poll::Ready(Some(CallModelEvent::Completed { content, .. })) => {
                    self.result = Some(Ok(content));
                    self.finished = true;
                    return;
                }
                Poll::Ready(Some(CallModelEvent::Error(error))) => {
                    self.result = Some(Err(error));
                    self.finished = true;
                    return;
                }
                Poll::Ready(Some(CallModelEvent::ReasoningChunk(_))) => {}
                Poll::Ready(None) => {
                    if self.result.is_none() {
                        self.result = Some(Err("LLM task ended without completion".to_string()));
                    }
                    self.finished = true;
                    return;
                }
                Poll::Pending => return,
            }
        }
    }

    fn mark_timeout(&mut self) {
        if self.result.is_none() {
            self.result = Some(Err("LLM task timed out".to_string()));
        }
        self.finished = true;
    }
}

impl ModelTaskManager {
    /// 创建一个新的任务管理器
    pub fn new(model: ChatModel) -> Self {
        Self {
            model,
            tasks: HashMap::new(),
            completed_tasks: HashMap::new(),
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
        let msgs = context.to_messages();
        let stream = Box::pin(stream! {
            let mut inner_stream = std::pin::pin!(call_model(&model, &msgs, None));
            while let Some(event) = inner_stream.next().await {
                yield event;
            }
        });

        self.completed_tasks.remove(&entity);
        self.tasks
            .insert(entity, LlmTaskMetadata::new(stream, context.clone()));
    }

    /// 轮询所有任务，收集已完成或超时的结果
    /// 注意：已完成或超时的任务会从 pending 集合中移出，但会保留在完成快照中，直到显式删除。
    /// 解析 stream 的 event，更新 TextChunk -> chunks
    pub fn poll_tasks(&mut self) {
        let mut finished_entities = Vec::new();
        let timeout = self.timeout.to_std().unwrap_or_default();

        for (entity, metadata) in self.tasks.iter_mut() {
            if metadata.created_at.elapsed() >= timeout {
                metadata.mark_timeout();
                finished_entities.push(*entity);
                continue;
            }

            metadata.poll_stream();

            if metadata.finished {
                finished_entities.push(*entity);
            }
        }

        for entity in finished_entities {
            if let Some(metadata) = self.tasks.remove(&entity) {
                self.completed_tasks.insert(entity, metadata.snapshot());
            }
        }
    }

    // 获取任务元信息
    // 可从中取进行中的中间结果，或已完成任务的最终快照
    pub fn get_task(&self, entity: Entity) -> Option<LlmTaskSnapshot> {
        if let Some(metadata) = self.tasks.get(&entity) {
            return Some(metadata.snapshot());
        }

        if let Some(snapshot) = self.completed_tasks.get(&entity) {
            return Some(LlmTaskSnapshot {
                created_at: snapshot.created_at,
                context: snapshot.context.clone(),
                chunks: snapshot.chunks.clone(),
                result: snapshot.result.clone(),
            });
        }

        None
    }

    /// 删除实体的任务
    pub fn remove_tasks(&mut self, entity: Entity) {
        self.tasks.remove(&entity);
        self.completed_tasks.remove(&entity);
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
