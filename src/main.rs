//! papertriangle — binary entrypoint.
//!
//! A localhost MOCK / OBSERVATION server that mirrors the *documented* network
//! surface described in `docs/spec/network_calls.md` (itself a static-analysis
//! writeup). It performs no real authentication, implements none of the
//! proprietary request-signing crypto, and talks to no live hosts. Its only job
//! is to expose endpoint *shapes* and to log the headers/fields a client sends,
//! so the protocol surface can be studied and documented.
//!
//! See `docs/scope-and-ethics.md` for the boundaries this project deliberately
//! keeps.

use clap::Parser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "papertriangle=info".into());
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let config = papertriangle::config::Config::parse();
    papertriangle::run(config).await
}
