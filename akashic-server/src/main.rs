mod api;
mod db;
mod error;
mod state;

use std::{net::SocketAddr, path::PathBuf};

use anyhow::Context;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{api::build_router, db::ArchiveRepository, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let archive_repo = ArchiveRepository::new(default_archive_db_path());
    let state = AppState::new(archive_repo)
        .map_err(|err| anyhow::anyhow!("failed to initialize app state: {:?}", err))?;

    let app = build_router(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    info!("akashic-server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind to {}", addr))?;

    axum::serve(listener, app)
        .await
        .context("failed to start akashic-server")?;

    Ok(())
}

fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "akashic_server=info,axum=info,tower_http=info".into());

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(false)
                .compact(),
        )
        .init();
}

fn default_archive_db_path() -> PathBuf {
    PathBuf::from("akashic-server-data/archive.sqlite3")
}
