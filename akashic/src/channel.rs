use agent::agent::AgentActorEvent;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;
use chrono::{DateTime, Utc};



// 事件用于描述世界发生了什么，包括世界变化、NPC 行为、主角相关事件等。
// 叙事者主要根据事件列表来展开故事。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AkashicEvent {
    // 世界变化
    WorldStateChanged,      // 全球变量变化（如经济崩溃、天灾降临）
    TimeAdvanced,           // 时间推进到新阶段
    
    // NPC 行为
    NpcAction,              // NPC 自主行动（如离开城镇、暗中谋划）
    NpcInteraction,         // NPC 与主角互动（如对话、交易、冲突）
    
    // 主角相关
    ProtagonistChoice,      // 主角做出了一个选择
    ProtagonistAction,      // 主角的非选择行为（如移动、使用物品）
    EncounterTriggered,     // 触发了一个新的遭遇（向主角呈现选项前）
    
    // 叙事元事件
    SceneTransition,        // 场景切换
    MilestoneReached,       // 剧情里程碑（如“初遇反派”）

    // AgentEvent
    AgentActor(AgentActorEvent),
}

#[derive(Clone)]
struct EventChannel {
    channel: broadcast::Sender<AkashicEvent>,
}

impl EventChannel {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<AkashicEvent>(64);
        Self { channel: tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<AkashicEvent> {
        self.channel.subscribe()
    }

    pub fn send(&self, event: impl Into<AkashicEvent>) {
        let _ = self.channel.send(event.into());
    }
}



// Message 是 Agent 之间通信的消息类型，包含了消息的元数据和实际内容。

/// 消息来源/目标标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentId {
    Weaver,       // 命运编织者
    Protagonist,  // 主角
    Narrator,     // 上层叙事者
}

/// 消息元数据（所有消息共有的头部）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMeta {
    pub message_id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub source: AgentId,
    pub target: AgentId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FateWeaverMessage {
    Start,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtagonistMessage {
    Action
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UpperNarratorMessage {
    DraftScene,
}

pub struct AgentInboxes {
    pub fate_weaver: mpsc::Receiver<FateWeaverMessage>,
    pub protagonist: mpsc::Receiver<ProtagonistMessage>,
    pub upper_narrator: mpsc::Receiver<UpperNarratorMessage>,
}


#[derive(Clone)]
pub struct AgentChannel {
    event: EventChannel,
    fate_weaver_tx: mpsc::Sender<FateWeaverMessage>,
    protagonist_tx: mpsc::Sender<ProtagonistMessage>,
    upper_narrator_tx: mpsc::Sender<UpperNarratorMessage>,
}

impl AgentChannel {
    pub fn new() -> (Self, AgentInboxes) {
        let (fate_weaver_tx, fate_weaver_rx) = mpsc::channel(64);
        let (protagonist_tx, protagonist_rx) = mpsc::channel(64);
        let (upper_narrator_tx, upper_narrator_rx) = mpsc::channel(64);
        (
            Self {
                event: EventChannel::new(),
                fate_weaver_tx,
                protagonist_tx,
                upper_narrator_tx,
            },
            AgentInboxes {
                fate_weaver: fate_weaver_rx,
                protagonist: protagonist_rx,
                upper_narrator: upper_narrator_rx,
            },
        )
    }

    pub fn subscribe_event(&self) -> broadcast::Receiver<AkashicEvent> {
        self.event.subscribe()
    }

    pub fn send_event(&self, event: impl Into<AkashicEvent>) {
        self.event.send(event);
    }

    pub async fn send_fate_weaver(
        &self,
        msg: FateWeaverMessage,
    ) -> Result<(), mpsc::error::SendError<FateWeaverMessage>> {
        self.fate_weaver_tx.send(msg).await
    }

    pub async fn send_protagonist(
        &self,
        msg: ProtagonistMessage,
    ) -> Result<(), mpsc::error::SendError<ProtagonistMessage>> {
        self.protagonist_tx.send(msg).await
    }

    pub async fn send_upper_narrator(
        &self,
        msg: UpperNarratorMessage,
    ) -> Result<(), mpsc::error::SendError<UpperNarratorMessage>> {
        self.upper_narrator_tx.send(msg).await
    }

    pub fn get_evt_sender(&self) -> broadcast::Sender<AkashicEvent> {
        self.event.channel.clone()
    }
}
