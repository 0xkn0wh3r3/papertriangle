//! Console ↔ Social Club account-link flow — host `#5`, spec §2.8.
//!
//! `GET /console/` begins the flow with the documented query template; a
//! completion POST follows. We echo the observed query parameters (notably the
//! per-device `machineId`, which the spec flags as a device identifier on the
//! wire) so the flow's inputs are visible for study.

use axum::{
    extract::{Query, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::{mockdata, security, state::SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/console", get(start))
        .route("/console/", get(start))
        .route("/console/complete", get(complete).post(complete))
        .route("/console/complete/error", get(error))
}

/// Begin the link flow. Query template (spec §2.8):
/// `?cid=&scAuthLinkToken=&playerTicket=&machineId=&lang=&mode=`
async fn start(
    State(st): State<SharedState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Json<Value> {
    st.record(json!({
        "endpoint": "console.start",
        "query": params,
        "observed_headers": security::observe(&headers),
        "privacy_note": "machineId is a per-device identifier (spec §2.8 / §6)."
    }));

    Json(json!({
        "_mock": true,
        "flow": "console-account-link",
        "received_params": params,
        "next": "POST /console/complete",
        "note": "Stub link page. No account is linked and no token is validated."
    }))
}

async fn complete(
    State(st): State<SharedState>,
    Query(params): Query<HashMap<String, String>>,
    headers: HeaderMap,
) -> Json<Value> {
    st.record(json!({
        "endpoint": "console.complete",
        "query": params,
        "observed_headers": security::observe(&headers),
    }));

    Json(json!({
        "_mock": true,
        "flow": "console-account-link",
        "status": "completed",
        "session": mockdata::mock_session(&st.config),
        "note": "Fabricated completion."
    }))
}

async fn error(State(st): State<SharedState>) -> Json<Value> {
    st.record(json!({ "endpoint": "console.complete.error" }));
    Json(json!({
        "_mock": true,
        "flow": "console-account-link",
        "status": "error",
        "note": "Stub error path (spec §2.8: /console/complete/error)."
    }))
}
