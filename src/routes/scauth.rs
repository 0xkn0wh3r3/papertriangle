//! Social Club authentication frontend — host `%s.scauth.rockstargames.com`
//! (spec §1 #1, §2.1). Mirrors the ticket-creation entry point and returns a
//! fabricated session object. Performs no authentication.

use axum::{extract::State, http::HeaderMap, routing::post, Json, Router};
use serde_json::Value;

use crate::{mockdata, security, state::SharedState};

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/scauth", post(create_ticket).get(create_ticket))
        .route("/scauth/{*op}", post(create_ticket).get(create_ticket))
        // GTA V-era shape where auth lived under gameservices as `auth.asmx`.
        .route("/auth.asmx/{*op}", post(create_ticket).get(create_ticket))
}

/// Stub for `CreateTicketScAuthToken2`-style calls. Records the security headers
/// the client presented, then returns a fabricated session.
async fn create_ticket(State(st): State<SharedState>, headers: HeaderMap) -> Json<Value> {
    st.record(serde_json::json!({
        "endpoint": "scauth.create_ticket",
        "observed_headers": security::observe(&headers),
    }));
    Json(mockdata::mock_session(&st.config))
}
