//! ROS game services — the `/<title>/<version>/gameservices/<service>.asmx/<Op>`
//! URL shape (spec §7.2). A single generic dispatcher mirrors the shape for any
//! service; a few well-known services return a more specific stub.

use axum::{
    extract::{Path, State},
    http::HeaderMap,
    routing::get,
    Json, Router,
};
use serde_json::{json, Value};

use crate::{mockdata, security, state::SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new().route(
        "/{title}/{version}/gameservices/{*op}",
        get(dispatch).post(dispatch),
    )
}

/// Generic gameservices dispatcher.
///
/// `op` captures `"<service>.asmx/<Operation>"` (or `.svc`). We split it into a
/// service and operation and return a shape-appropriate stub.
async fn dispatch(
    State(st): State<SharedState>,
    Path((title, version, op)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Json<Value> {
    let (service_file, operation) = op.split_once('/').unwrap_or((op.as_str(), ""));
    let service = service_file
        .trim_end_matches(".asmx")
        .trim_end_matches(".svc")
        .to_lowercase();

    let observed = security::observe(&headers);
    st.record(json!({
        "endpoint": "gameservices",
        "service": service,
        "operation": operation,
        "title": title,
        "version": version,
        "observed_headers": observed,
    }));

    let body = match service.as_str() {
        "auth" | "scauth" => mockdata::mock_session(&st.config),
        "telemetry" => json!({
            "_mock": true,
            "service": "Telemetry",
            "operation": operation,
            "accepted": true,
            "stored": false,
            "note": "SubmitRealTime stub — the telemetry batch is acknowledged and discarded."
        }),
        "cloudsave" => json!({
            "_mock": true,
            "service": "cloudsave",
            "operation": operation,
            "note": "Cloud-save calls also have a dedicated REST shape; see /cloud/... routes."
        }),
        _ => json!({
            "_mock": true,
            "service": service,
            "operation": operation,
            "title": title,
            "version": version,
            "note": "Generic stub mirroring /<title>/<version>/gameservices/<service>.asmx/<Operation>."
        }),
    };

    Json(body)
}
