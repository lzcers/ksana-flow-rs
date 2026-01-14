mod db;
mod handlers;
mod registry;
mod state;

use anyhow::Context;
use axum::{
    Router,
    routing::{get, post},
};
use std::net::SocketAddr;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::db::Db;
use crate::handlers::{
    create_workflow, delete_workflow, get_nodes, get_workflow, list_workflows, run_node,
    run_workflow, update_workflow, ws_handler,
};
use crate::registry::create_registry;
use crate::state::AppState;

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
    let db = Db::new("ksana.db").context("Failed to initialize database")?;

    let app_state = AppState {
        registry: Arc::new(RwLock::new(registry)),
        running: Arc::new(AtomicBool::new(false)),
        tx,
        db: Arc::new(Mutex::new(db)),
    };

    let app = Router::new()
        .route("/api/workflows", get(list_workflows).post(create_workflow))
        .route(
            "/api/workflows/:id",
            get(get_workflow)
                .put(update_workflow)
                .delete(delete_workflow),
        )
        .route("/api/workflow/run", post(run_workflow))
        .route("/api/workflow/run_node", post(run_node))
        .route("/api/nodes", get(get_nodes))
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
