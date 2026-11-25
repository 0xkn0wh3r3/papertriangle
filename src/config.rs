//! Runtime configuration (CLI flags / environment variables).

use clap::Parser;

/// Configuration for the mock/observation server.
#[derive(Debug, Clone, Parser)]
#[command(
    name = "papertriangle",
    about = "Mock/observation server mirroring the documented ROS-style network surface. \
             Research scaffold only — not a functioning backend, no real crypto, no live hosts."
)]
pub struct Config {
    /// Address to bind, e.g. `127.0.0.1:8080`. Defaults to loopback on purpose.
    #[arg(long, env = "PT_BIND", default_value = "127.0.0.1:8080")]
    pub bind: String,

    /// Environment token that fills the `<env>` slot in generated host strings
    /// (`<env>-<service>.ros.rockstargames.com`). Cosmetic only.
    #[arg(long, env = "PT_ENV", default_value = "prod")]
    pub environment: String,

    /// Title token used in the `/<title>/<version>/gameservices/...` URL shape.
    #[arg(long, env = "PT_TITLE", default_value = "gta6")]
    pub title: String,
}
