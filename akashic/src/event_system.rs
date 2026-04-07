use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

use crate::{
    fate_weaver::FateWeaverEvent, protagonist::ProtagonistEvent,
    upper_narrator::UpperNarratorEvent,
};

// Agent 之间通信的消息通道
#[derive(Clone)]
pub struct EventChannel {
    channel: broadcast::Sender<Event>,
}

impl EventChannel {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel::<Event>(64);
        Self { channel: tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.channel.subscribe()
    }

    pub fn send(&self, event: impl Into<Event>) {
        let _ = self.channel.send(event.into());
    }
}



#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum SystemEvent {
    DecisionAutoSelected {
        round: u32,
        option_id: String,
        reason: String,
    },
    AgentError {
        agent: String,
        message: String,
    },
    ShutdownRequested,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "scope", content = "event", rename_all = "snake_case")]
pub enum Event {
    FateWeaver(FateWeaverEvent),
    Protagonist(ProtagonistEvent),
    UpperNarrator(UpperNarratorEvent),
    System(SystemEvent),
}

impl From<FateWeaverEvent> for Event {
    fn from(event: FateWeaverEvent) -> Self {
        Self::FateWeaver(event)
    }
}

impl From<ProtagonistEvent> for Event {
    fn from(event: ProtagonistEvent) -> Self {
        Self::Protagonist(event)
    }
}

impl From<UpperNarratorEvent> for Event {
    fn from(event: UpperNarratorEvent) -> Self {
        Self::UpperNarrator(event)
    }
}

impl From<SystemEvent> for Event {
    fn from(event: SystemEvent) -> Self {
        Self::System(event)
    }
}
