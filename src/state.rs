//! Shared, in-memory application state.
//!
//! Everything here is ephemeral and local. The "cloud store" is a plain
//! `HashMap` in RAM; observations are a bounded ring of the last N requests so
//! you can inspect, via `GET /__/observations`, what a client actually sent.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::config::Config;

/// Maximum number of observation records retained in memory.
const MAX_OBSERVATIONS: usize = 1000;

/// Process-wide state, shared across handlers via [`SharedState`].
pub struct AppState {
    pub config: Config,
    pub requests_seen: AtomicU64,
    /// Opaque "cloud" blobs, keyed by their composed path.
    pub cloud: Mutex<HashMap<String, Vec<u8>>>,
    /// Ring buffer of recent request observations (headers/fields seen).
    pub observations: Mutex<Vec<Value>>,
}

impl AppState {
    /// Build shareable state from config.
    pub fn new(config: Config) -> SharedState {
        Arc::new(Self {
            config,
            requests_seen: AtomicU64::new(0),
            cloud: Mutex::new(HashMap::new()),
            observations: Mutex::new(Vec::new()),
        })
    }

    /// Record a single observation, incrementing the request counter and
    /// trimming the ring buffer if needed.
    pub fn record(&self, entry: Value) {
        self.requests_seen.fetch_add(1, Ordering::Relaxed);
        tracing::info!(target: "papertriangle::observe", "{entry}");
        let mut obs = self.observations.lock().unwrap();
        if obs.len() >= MAX_OBSERVATIONS {
            obs.remove(0);
        }
        obs.push(entry);
    }
}

/// Clone-cheap handle to [`AppState`] used as the axum router state.
pub type SharedState = Arc<AppState>;
