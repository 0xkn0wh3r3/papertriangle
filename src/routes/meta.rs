//! Meta endpoints that belong to the mock itself (not the spec): a landing
//! page, a health check, and an observation dump.

use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

use crate::state::SharedState;

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/", get(index))
        .route("/healthz", get(healthz))
        .route("/__/observations", get(observations))
}

async fn healthz() -> &'static str {
    "ok"
}

/// Human-oriented description of what this server is and the routes it exposes.
async fn index(State(st): State<SharedState>) -> Json<Value> {
    Json(json!({
        "name": "papertriangle",
        "kind": "mock / observation server",
        "disclaimer": "Fabricated responses. No real authentication, no proprietary crypto, \
                       no live hosts are ever contacted. Research/documentation scaffold.",
        "source_spec": "docs/spec/network_calls.md",
        "config": {
            "environment": st.config.environment,
            "title": st.config.title,
        },
        "requests_seen": st.requests_seen.load(Ordering::Relaxed),
        "endpoints": {
            "scauth": "POST /scauth/{*op}  and  POST /auth.asmx/{*op}",
            "gameservices": "GET|POST /{title}/{version}/gameservices/{service}.asmx/{Operation}",
            "cloud": "GET|PUT|POST /cloud/{id}/cloudservices/{name}[/{tail}]",
            "console_link": "GET /console/ , POST /console/complete , GET /console/complete/error",
            "xbox_session_directory": "ANY /connections/{*rest}",
            "xbox_stats": "GET|POST /scids/{scid}/stats/users/{xuid}",
            "xbox_presence_social": "ANY /users/{*rest}",
            "rta_websocket": "GET /connect (ws)  and  GET /ros/pubsub (ws)",
            "crash_upload": "POST /crash/upload  and  POST /post  (multipart)",
            "observations": "GET /__/observations"
        }
    }))
}

/// Dump the recent request observations captured in memory.
async fn observations(State(st): State<SharedState>) -> Json<Value> {
    let obs = st.observations.lock().unwrap().clone();
    Json(json!({
        "requests_seen": st.requests_seen.load(Ordering::Relaxed),
        "retained": obs.len(),
        "observations": obs,
    }))
}
