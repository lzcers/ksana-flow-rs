mod api;
mod error;
mod state;

use std::net::SocketAddr;

use anyhow::Context;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::{api::build_router, state::AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let app = build_router(AppState::default())
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
