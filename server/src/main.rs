mod handlers;
mod registry;
mod state;

use anyhow::Context;
use axum::{
    Router,
    routing::{delete, get, post},
};
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, RwLock};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::handlers::{
    add_edge, add_node, get_graph, get_nodes, remove_edge, remove_node, run_flow,
    update_node_position, ws_handler,
};
use crate::registry::create_registry;
use crate::state::{AppState, GraphBlueprint};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "server=info,flow=info,nodes=info,axum=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let registry = create_registry();
    let (tx, _rx) = broadcast::channel(100);

    let app_state = AppState {
        registry: Arc::new(RwLock::new(registry)),
        blueprint: Arc::new(RwLock::new(GraphBlueprint::default())),
        tx,
        running: Arc::new(AtomicBool::new(false)),
    };

    let app = Router::new()
        .route("/api/nodes", get(get_nodes))
        .route("/api/graph", get(get_graph))
        .route("/api/graph/node", post(add_node))
        .route("/api/graph/node/:id/position", post(update_node_position))
        .route("/api/graph/node/:id", delete(remove_node))
        .route("/api/graph/edge", post(add_edge))
        .route("/api/graph/edge/:id", delete(remove_edge))
        .route("/api/run", post(run_flow))
        .route("/ws", get(ws_handler))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    info!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context(format!("Failed to bind to address {}", addr))?;
    axum::serve(listener, app)
        .await
        .context("Failed to start axum server")?;

    Ok(())
}
