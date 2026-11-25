//! Cloud storage frontend — host `<prefix>.cloud.rockstargames.com` with the
//! `/cloud/<id>/cloudservices/<name>/<mode-tail>` path builder (spec §2.4).
//!
//! GET returns a stored blob or a fabricated placeholder; PUT/POST store the
//! body in the in-memory map so round-trips are observable. Bodies are treated
//! as opaque `application/octet-stream`, matching the spec.

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{header, HeaderMap},
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde_json::json;

use crate::{security, state::SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route(
            "/cloud/{id}/cloudservices/{name}",
            get(get_file).put(set_file).post(set_file),
        )
        .route(
            "/cloud/{id}/cloudservices/{name}/{*tail}",
            get(get_file_tail).put(set_file_tail).post(set_file_tail),
        )
}

async fn get_file(
    State(st): State<SharedState>,
    Path((id, name)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    serve(&st, &format!("{id}/{name}"), &headers)
}

async fn get_file_tail(
    State(st): State<SharedState>,
    Path((id, name, tail)): Path<(String, String, String)>,
    headers: HeaderMap,
) -> Response {
    serve(&st, &format!("{id}/{name}/{tail}"), &headers)
}

async fn set_file(
    State(st): State<SharedState>,
    Path((id, name)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    store(&st, format!("{id}/{name}"), &headers, body)
}

async fn set_file_tail(
    State(st): State<SharedState>,
    Path((id, name, tail)): Path<(String, String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    store(&st, format!("{id}/{name}/{tail}"), &headers, body)
}

fn serve(st: &SharedState, key: &str, headers: &HeaderMap) -> Response {
    st.record(json!({
        "endpoint": "cloud.get",
        "key": key,
        "observed_headers": security::observe(headers),
    }));

    let store = st.cloud.lock().unwrap();
    let bytes = store.get(key).cloned().unwrap_or_else(|| {
        format!("MOCK-CLOUD-BLOB key={key}\nNo real cloud-save data is served.\n").into_bytes()
    });

    (
        [(header::CONTENT_TYPE, "application/octet-stream")],
        bytes,
    )
        .into_response()
}

fn store(st: &SharedState, key: String, headers: &HeaderMap, body: Bytes) -> Response {
    let len = body.len();
    st.record(json!({
        "endpoint": "cloud.set",
        "key": key,
        "bytes": len,
        "observed_headers": security::observe(headers),
    }));
    st.cloud.lock().unwrap().insert(key.clone(), body.to_vec());

    Json(json!({
        "_mock": true,
        "stored": true,
        "key": key,
        "bytes": len,
        "note": "Blob stored in memory for this process only."
    }))
    .into_response()
}
