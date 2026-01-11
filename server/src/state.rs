use crate::registry::NodeRegistry;
use flow::{FlowEvent, Graph};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;

use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub struct AppState {
    pub registry: Arc<RwLock<NodeRegistry>>,
    pub blueprint: Arc<RwLock<GraphBlueprint>>,
    pub tx: broadcast::Sender<FlowEvent>,
    pub running: Arc<AtomicBool>,
}

impl AppState {
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn set_running(&self, running: bool) {
        self.running.store(running, Ordering::SeqCst);
    }
}

#[derive(Serialize, Deserialize, Clone, Default, Debug)]
pub struct GraphBlueprint {
    pub nodes: Vec<NodeConfig>,
    pub edges: Vec<EdgeConfig>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct NodeConfig {
    pub id: String,
    pub type_name: String,
    pub config: Value,
    pub position: Option<Position>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Position {
    pub x: f64,
    pub y: f64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EdgeConfig {
    pub id: String,
    pub source: String,
    pub target: String,
}
