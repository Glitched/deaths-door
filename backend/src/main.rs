//! Death's Door server entry point.

use std::net::SocketAddr;
use std::time::Duration;

use deaths_door::app::{build_router, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    if let Err(err) = run().await {
        // Clean exit with a logged reason instead of a panic backtrace.
        tracing::error!("fatal: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let db_path = std::env::var("DATABASE_PATH").unwrap_or_else(|_| "games.db".to_string());
    let state = AppState::new(&db_path)?;
    let app = build_router(state);

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8000);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("Death's Door listening on http://{addr}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Resolve when the process receives Ctrl-C or (on Unix) SIGTERM.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        if let Ok(mut sig) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        {
            sig.recv().await;
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
    tracing::info!("shutdown signal received, stopping (force-exit in 2s)");

    // Long-lived SSE streams (`/game/stream`) never close on their own, so a
    // pure graceful drain would hang waiting for them. Force-exit shortly after
    // the signal as a backstop. Quick in-flight requests still finish first.
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_secs(2)).await;
        tracing::warn!("forcing exit (open streams did not drain in time)");
        std::process::exit(0);
    });
}
