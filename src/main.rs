use std::error::Error;
use std::time::Duration;

use keyloop_challenge::config::Config;
use keyloop_challenge::domain::SlotGrid;
use keyloop_challenge::infra::{EventBus, Locks};
use keyloop_challenge::state::AppState;
use keyloop_challenge::{api, telemetry};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tokio::signal::unix::{SignalKind, signal};
use tokio_util::sync::CancellationToken;

const CONNECT_ATTEMPTS: u32 = 10;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::from_env()?;
    telemetry::init(config.log_format);

    let pool = connect_pool(&config).await?;
    let listener = TcpListener::bind(config.bind_addr).await?;
    tracing::info!(addr = %config.bind_addr, "listening");

    let shutdown = CancellationToken::new();
    let state = AppState {
        pool,
        locks: Locks::new(Duration::from_millis(config.lock_timeout_ms)),
        grid: SlotGrid::new(config.slot_step_minutes),
        bus: EventBus::new(),
        shutdown: shutdown.clone(),
    };
    let app = api::router(state);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            shutdown.cancel();
        })
        .await?;

    tracing::info!("stopped");
    Ok(())
}

async fn connect_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    let options = PgPoolOptions::new().max_connections(config.db_pool_max);
    let mut attempt = 1;
    loop {
        match options.clone().connect(&config.database_url).await {
            Ok(pool) => return Ok(pool),
            Err(err) if attempt < CONNECT_ATTEMPTS => {
                tracing::warn!(attempt, error = %err, "database not ready, retrying");
                attempt += 1;
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
            Err(err) => return Err(err),
        }
    }
}

async fn shutdown_signal() {
    let mut term = signal(SignalKind::terminate()).expect("install SIGTERM handler");
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        _ = term.recv() => {}
    }
    tracing::info!("shutdown signal received");
}
