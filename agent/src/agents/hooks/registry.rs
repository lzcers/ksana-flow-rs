use std::collections::HashMap;

use serde_json::Value;

use super::{ExecutionPolicy, MetricsHook, RuntimeHook, StreamingEventHook};
use crate::agents::AgentState;

pub(crate) struct RuntimeHookRegistry {
    hooks: Vec<Box<dyn RuntimeHook>>,
}

impl RuntimeHookRegistry {
    pub(crate) fn empty() -> Self {
        Self { hooks: Vec::new() }
    }

    pub(crate) fn register<H>(mut self, hook: H) -> Self
    where
        H: RuntimeHook + 'static,
    {
        self.hooks.push(Box::new(hook));
        self
    }

    pub(crate) fn push<H>(&mut self, hook: H)
    where
        H: RuntimeHook + 'static,
    {
        self.hooks.push(Box::new(hook));
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &(dyn RuntimeHook + '_)> {
        self.hooks.iter().map(|hook| hook.as_ref())
    }

    pub(crate) fn execution_policy(&self, state: &AgentState) -> ExecutionPolicy {
        let mut policy = ExecutionPolicy::default();
        for hook in self.iter() {
            hook.configure_execution_policy(state, &mut policy);
        }
        policy
    }

    pub(crate) fn snapshot(&self, name: &str) -> Option<Value> {
        self.iter()
            .find(|hook| hook.name() == name)
            .and_then(|hook| hook.snapshot())
    }

    pub(crate) fn snapshots(&self) -> HashMap<String, Value> {
        self.iter()
            .filter_map(|hook| {
                hook.snapshot()
                    .map(|snapshot| (hook.name().to_string(), snapshot))
            })
            .collect()
    }
}

impl Default for RuntimeHookRegistry {
    fn default() -> Self {
        Self::empty()
            .register(MetricsHook::default())
            .register(StreamingEventHook)
    }
}
