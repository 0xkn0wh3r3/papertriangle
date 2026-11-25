//! Crash reporting — Breakpad → Backtrace minidump upload (spec §4).
//!
//! The real destination URL is runtime-supplied, so there is no fixed path to
//! mirror; we accept `multipart/form-data` on a stub path and report which
//! parts/attributes were sent. Uploaded bytes are counted and discarded.

use axum::{extract::{Multipart, State}, routing::post, Json, Router};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::{error::AppError, state::SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/crash/upload", post(upload))
        .route("/post", post(upload))
}

/// Accept a multipart minidump upload, summarize its parts, and discard it.
async fn upload(
    State(st): State<SharedState>,
    mut multipart: Multipart,
) -> Result<Json<Value>, AppError> {
    let mut parts = Vec::new();
    let mut total = 0usize;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or_default().to_string();
        let filename = field.file_name().map(|s| s.to_string());
        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::BadRequest(e.to_string()))?;
        total += data.len();
        parts.push(json!({ "name": name, "filename": filename, "bytes": data.len() }));
    }

    let report_id = Uuid::new_v4();
    st.record(json!({
        "endpoint": "crash.upload",
        "report_id": report_id,
        "parts": parts,
        "total_bytes": total,
    }));

    Ok(Json(json!({
        "_mock": true,
        "accepted": true,
        "report_id": report_id,
        "parts": parts,
        "total_bytes": total,
        "note": "Minidump bytes were counted and discarded; nothing is stored or forwarded."
    })))
}
