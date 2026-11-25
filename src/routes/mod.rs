//! Router assembly.
//!
//! Each submodule owns one slice of the documented surface and exposes a
//! `routes() -> Router<SharedState>`. Paths are kept faithful to the spec where
//! they don't collide; the mapping from real hostnames to these local paths is
//! documented in `docs/api-surface.md`.

use axum::Router;

use crate::state::SharedState;

mod cloud;
mod console;
mod crash;
mod gameservices;
mod meta;
mod rta;
mod scauth;
mod xbox;

/// Build the full application router with shared state attached.
pub fn router(state: SharedState) -> Router {
    Router::new()
        .merge(meta::routes())
        .merge(scauth::routes())
        .merge(gameservices::routes())
        .merge(cloud::routes())
        .merge(console::routes())
        .merge(xbox::routes())
        .merge(rta::routes())
        .merge(crash::routes())
        .with_state(state)
}
