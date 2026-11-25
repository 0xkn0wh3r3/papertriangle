//! papertriangle — a mock/observation server for the documented ROS-style
//! network surface.
//!
//! # What this is
//!
//! A research scaffold. It stands up an HTTP(S-less) server on localhost whose
//! routes echo the URL shapes, header *names*, and field *names* recorded in
//! [`docs/spec/network_calls.md`]. Responses are obviously-fabricated stubs.
//!
//! # What this is NOT
//!
//! - It is **not** a reimplementation of Rockstar Online Services.
//! - It does **not** implement the `ros-HeadersHmac` / `ros-ContentSignature`
//!   signing scheme, session-ticket crypto, or any authentication logic. (Those
//!   values are not present in the source spec and are intentionally out of
//!   scope — see `docs/scope-and-ethics.md`.)
//! - It never contacts `*.rockstargames.com`, `*.xboxlive.com`, or any other
//!   live service.
//!
//! The purpose is *documentation and understanding* of the observed surface.

pub mod config;
pub mod error;
pub mod mockdata;
pub mod routes;
pub mod security;
pub mod state;

use anyhow::Context;

/// Build the application and serve it until the process is stopped.
pub async fn run(config: config::Config) -> anyhow::Result<()> {
    let bind = config.bind.clone();
    let state = state::AppState::new(config);
    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .with_context(|| format!("failed to bind {bind}"))?;

    tracing::info!("papertriangle mock/observation server listening on http://{bind}");
    tracing::info!("this is a stub server — responses are fabricated and no real service is contacted");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
