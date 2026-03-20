use std::any::Any;

use serde_json::Value;
use tokio::sync::mpsc;

use super::{StepFrame, StepResultDraft};
use crate::agents::{AgentActorEvent, AgentError};

pub(crate) enum Effect {
    EmitNow(AgentActorEvent),
    SetMetadata {
        key: String,
        value: Value,
    },
    RemoveMetadata {
        key: String,
    },
    StoreScratchpad {
        key: &'static str,
        value: Box<dyn Any + Send + Sync>,
    },
    SetResult(StepResultDraft),
    Abort(AgentError),
}

impl Effect {
    pub(crate) fn store_scratchpad<T>(key: &'static str, value: T) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self::StoreScratchpad {
            key,
            value: Box::new(value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EffectSignal {
    Continue,
    ResultSet,
    Aborted,
}

pub(crate) struct EffectHandle;

impl EffectHandle {
    pub(crate) async fn apply(
        frame: &mut StepFrame,
        event_tx: Option<&mpsc::Sender<AgentActorEvent>>,
        effect: Effect,
    ) -> EffectSignal {
        match effect {
            Effect::EmitNow(event) => {
                if let Some(tx) = event_tx {
                    let _ = tx.send(event).await;
                }
                EffectSignal::Continue
            }
            Effect::SetMetadata { key, value } => {
                frame.metadata.insert(key, value);
                EffectSignal::Continue
            }
            Effect::RemoveMetadata { key } => {
                frame.metadata.remove(&key);
                EffectSignal::Continue
            }
            Effect::StoreScratchpad { key, value } => {
                frame.scratchpad.insert_box(key, value);
                EffectSignal::Continue
            }
            Effect::SetResult(result) => {
                frame.set_result(result);
                EffectSignal::ResultSet
            }
            Effect::Abort(err) => {
                frame.set_result(StepResultDraft::Error(err));
                EffectSignal::Aborted
            }
        }
    }
}
