//! Real-time channels.
//!
//! - `GET /connect` — Xbox Live Real-Time Activity (`wss://rta.xboxlive.com/connect`,
//!   spec §3.2). On connect it sends a sample presence frame, then echoes.
//! - `GET /ros/pubsub` — a demo of the ROS `ros.*` topic namespace (spec §2.9):
//!   on connect it emits one sample frame per documented topic, then echoes.
//!
//! These are illustrative: the real protocols' framing/crypto are not
//! reproduced.

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use serde_json::json;

use crate::state::SharedState;

/// ROS real-time topics documented in spec §2.9 (names only).
const ROS_TOPICS: &[&str] = &[
    "ros.publish",
    "ros.msgs",
    "ros.ticket.renew",
    "ros.ticket.invalidate",
    "ros.account.link",
    "ros.account.unlink",
    "ros.entitlement.changed",
    "ros.sc.membership.changed",
    "ros.cloud.file.changed",
    "ros.attrs.refresh",
];

pub fn routes() -> Router<SharedState> {
    Router::new()
        .route("/connect", get(rta_upgrade))
        .route("/ros/pubsub", get(ros_upgrade))
}

async fn rta_upgrade(State(st): State<SharedState>, ws: WebSocketUpgrade) -> Response {
    st.record(json!({ "endpoint": "rta.connect", "transport": "websocket" }));
    ws.on_upgrade(rta_socket)
}

async fn rta_socket(mut socket: WebSocket) {
    let hello = json!({
        "_mock": true,
        "service": "rta.xboxlive.com",
        "event": "presence",
        "note": "Sample RTA frame. Echoes anything you send."
    })
    .to_string();
    let _ = socket.send(Message::Text(hello.into())).await;

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(t) => {
                let reply = format!("echo: {}", t.as_str());
                if socket.send(Message::Text(reply.into())).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

async fn ros_upgrade(State(st): State<SharedState>, ws: WebSocketUpgrade) -> Response {
    st.record(json!({ "endpoint": "ros.pubsub", "transport": "websocket" }));
    ws.on_upgrade(ros_socket)
}

async fn ros_socket(mut socket: WebSocket) {
    for topic in ROS_TOPICS {
        let frame = json!({
            "_mock": true,
            "topic": topic,
            "note": "Sample ros.* control-plane frame (spec §2.9)."
        })
        .to_string();
        if socket.send(Message::Text(frame.into())).await.is_err() {
            return;
        }
    }

    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(t) => {
                let reply = format!("echo: {}", t.as_str());
                if socket.send(Message::Text(reply.into())).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
