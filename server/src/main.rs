mod db;
mod handlers;
mod registry;
mod state;
mod utils;

use anyhow::Context;
use axum::{
    Router,
    routing::{get, post},
};
use serde_json::Value;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex, RwLock};
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tracing::info;

use crate::db::Db;
use crate::handlers::{
    create_workflow, delete_workflow, get_ai_media, get_file, get_nodes, get_workflow,
    get_workflow_status, list_workflows, pause_workflow, resume_workflow, run_node, run_workflow,
    stop_workflow, update_workflow, upload_file, ws_handler,
};
use crate::registry::create_registry;
use crate::state::AppState;

use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 根据环境变量选择日志格式
    // LOG_FORMAT=json 使用 JSON 格式（生产环境）
    // 默认使用紧凑格式（开发环境）
    let log_format = std::env::var("LOG_FORMAT").unwrap_or_default();
    let is_json = log_format.eq_ignore_ascii_case("json");

    // 构建基础环境过滤器
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "server=info,flow=info,nodes=info,axum=info".into());

    // 根据格式初始化 subscriber
    if is_json {
        // JSON 格式 - 适合生产环境，便于日志收集系统处理
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .json()
                    .with_timer(tracing_subscriber::fmt::time::UtcTime::rfc_3339())
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true)
                    .with_target(true)
                    .with_current_span(true)
                    .with_span_list(true),
            )
            .init();
    } else {
        // 紧凑格式 - 适合开发环境，便于阅读
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                tracing_subscriber::fmt::layer()
                    .with_timer(tracing_subscriber::fmt::time::LocalTime::rfc_3339())
                    .with_thread_ids(true)
                    .with_thread_names(true)
                    .with_file(true)
                    .with_line_number(true)
                    .with_target(false) // 隐藏模块路径，更简洁
                    .compact(), // 使用紧凑格式
            )
            .init();
    }

    let registry = create_registry();
    let (tx, _rx) = broadcast::channel::<(String, String, Value)>(100);
    let db = Db::new("ksana.db").context("Failed to initialize database")?;

    let app_state = AppState {
        registry: Arc::new(registry),
        executions: Arc::new(RwLock::new(std::collections::HashMap::new())),
        tx,
        db: Arc::new(Mutex::new(db)),
    };

    let app = Router::new()
        .route("/api/workflows", get(list_workflows).post(create_workflow))
        .route(
            "/api/workflows/{id}",
            get(get_workflow)
                .put(update_workflow)
                .delete(delete_workflow),
        )
        .route("/api/workflow/run", post(run_workflow))
        .route("/api/workflow/run_node", post(run_node))
        .route("/api/workflow/{id}/status", get(get_workflow_status))
        .route("/api/workflow/{id}/pause", post(pause_workflow))
        .route("/api/workflow/{id}/resume", post(resume_workflow))
        .route("/api/workflow/{id}/stop", post(stop_workflow))
        .route("/api/nodes", get(get_nodes))
        .route("/api/upload", post(upload_file))
        .route("/api/files/{id}", get(get_file))
        .route("/api/ai_media/{id}", get(get_ai_media))
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
