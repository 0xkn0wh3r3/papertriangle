//! Security-header *observation* (not enforcement).
//!
//! The source spec (`docs/spec/network_calls.md` §2.6) lists the header NAMES
//! ROS uses to sign requests. This module knows those names only so it can
//! *report which ones a client sent*. It does **not** validate, recompute, or
//! forge any signature — the algorithms and keys are not in the spec and are
//! deliberately out of scope (see `docs/scope-and-ethics.md`).

use axum::http::HeaderMap;
use serde_json::{json, Map, Value};

/// Header names documented on the ROS/Xbox surface. Used purely to surface what
/// a client presents, so the request shape can be studied.
pub const SECURITY_HEADERS: &[&str] = &[
    // ROS application-layer security headers (names only; see spec §2.6).
    "ros-SessionTicket",
    "Scs-Ticket",
    "ScAuthToken",
    "ros-Challenge",
    "ros-HeadersHmac",
    "ros-ContentHash",
    "ros-ContentSignature",
    "ros-SecurityFlags",
    "SCS-RequestId",
    "Authorization",
    // Platform / standard headers worth noting.
    "x-xbl-contract-version",
    "User-Agent",
    "Content-Type",
    "Accept-Language",
];

/// Return a map of the documented headers that are present on this request,
/// with their raw values. Absent headers are simply omitted.
pub fn observe(headers: &HeaderMap) -> Map<String, Value> {
    let mut seen = Map::new();
    for name in SECURITY_HEADERS {
        if let Some(value) = headers.get(*name) {
            let rendered = value.to_str().unwrap_or("<non-utf8>");
            seen.insert((*name).to_string(), json!(rendered));
        }
    }
    seen
}
