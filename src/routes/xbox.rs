//! Microsoft Xbox Live platform surface (spec §3). These are standard platform
//! calls for any online Xbox title; here they are mirrored as stubs.
//!
//! - Session directory (MPSD): `/connections/...` (spec §3.1)
//! - Title stats: `/scids/<scid>/stats/users/<xuid>` (spec §3.3)
//! - Presence / recent-players: `/users/...` (spec §3.3)

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::{any, get},
    Json, Router,
};
use serde_json::{json, Value};

use crate::{security, state::SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/connections/{*rest}", any(session_directory))
        .route("/scids/{scid}/stats/users/{xuid}", get(stats).post(stats))
        .route("/users/{*rest}", any(users))
}

async fn session_directory(
    State(st): State<SharedState>,
    Path(rest): Path<String>,
    headers: HeaderMap,
) -> Json<Value> {
    st.record(json!({
        "endpoint": "xbox.sessiondirectory",
        "path": format!("/connections/{rest}"),
        "observed_headers": security::observe(&headers),
    }));
    Json(json!({
        "_mock": true,
        "service": "sessiondirectory.xboxlive.com (MPSD)",
        "path": format!("/connections/{rest}"),
        "note": "Multiplayer session directory stub."
    }))
}

async fn stats(
    State(st): State<SharedState>,
    Path((scid, xuid)): Path<(String, String)>,
    headers: HeaderMap,
) -> Json<Value> {
    st.record(json!({
        "endpoint": "xbox.titlestats",
        "scid": scid,
        "xuid": xuid,
        "observed_headers": security::observe(&headers),
    }));
    Json(json!({
        "_mock": true,
        "service": "title stats",
        "scid": scid,
        "xuid": xuid,
        "statlistscollection": [{ "stats": {} }],
        "note": "Fabricated empty stats."
    }))
}

async fn users(
    State(st): State<SharedState>,
    Path(rest): Path<String>,
    headers: HeaderMap,
) -> Json<Value> {
    let kind = if rest.contains("recentplayers") {
        "recent-players"
    } else if rest.contains("titles/current") {
        "presence/title"
    } else {
        "users"
    };
    st.record(json!({
        "endpoint": "xbox.users",
        "kind": kind,
        "path": format!("/users/{rest}"),
        "observed_headers": security::observe(&headers),
    }));
    Json(json!({
        "_mock": true,
        "service": kind,
        "path": format!("/users/{rest}"),
        "note": "Xbox Live users/presence/social stub."
    }))
}
