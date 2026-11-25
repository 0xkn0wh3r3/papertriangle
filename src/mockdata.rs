//! Fabricated response data for the mock server.
//!
//! These builders mirror the *field names* documented in the spec so the mock's
//! responses have the right shape. Every value is random or constant — none is
//! real, and nothing here is cryptographically meaningful.
//!
//! (Named `mockdata` rather than `models` to stay clearly separate from the
//! out-of-scope client scaffolding that was added under `src/models/`.)

use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Map, Value};

use crate::config::Config;

/// ROS service-name tokens from the spec (§2.3 and §7.2). Used to generate a
/// plausible per-service host map in the mock session object.
pub const ROS_SERVICES: &[&str] = &[
    "auth",
    "scauth",
    "cloudsave",
    "ugc",
    "matchmaking",
    "conductor",
    "presence",
    "friends",
    "inbox",
    "feed",
    "profilestats",
    "leaderboards",
    "achievements",
    "unlocks",
    "commerce",
    "catalog",
    "telemetry",
    "geolocation",
    "ageassurance",
    "members",
    "entitlements",
    "playerreporting",
];

/// Build a fabricated session object whose *shape* matches the documented
/// session record (spec §7.1: `SessionTicket`, `SessionKey`, `PlayerAccountId`,
/// `Privileges`, `SecsUntilExpiration`, `Region`, per-service `Host`).
pub fn mock_session(cfg: &Config) -> Value {
    let account_id = 1_000_000_000u64 + (u64::from(rand::random::<u32>()) % 900_000_000);
    json!({
        "_mock": true,
        "_note": "Fabricated session. No authentication was performed; \
                  SessionTicket/SessionKey are random bytes, not valid credentials.",
        "SessionTicket": fake_b64(40),
        "SessionKey": fake_b64(24),
        "PlayerAccountId": account_id,
        "RockstarId": account_id,
        "Privileges": [1, 2, 5, 9],
        "SecsUntilExpiration": 86_400,
        "Region": 1,
        "PublicIp": "203.0.113.7",
        "Services": service_hosts(cfg),
    })
}

/// Generate the `<env>-<service>.ros.rockstargames.com` host map described in
/// spec §2.2 / §7.1. These are strings only; the server never resolves or
/// contacts them.
pub fn service_hosts(cfg: &Config) -> Value {
    let mut map = Map::new();
    for service in ROS_SERVICES {
        let host = format!("{}-{}.ros.rockstargames.com", cfg.environment, service);
        map.insert((*service).to_string(), json!(host));
    }
    Value::Object(map)
}

/// Random base64 string of `n` source bytes — a placeholder for an opaque token.
pub fn fake_b64(n: usize) -> String {
    let bytes: Vec<u8> = (0..n).map(|_| rand::random::<u8>()).collect();
    STANDARD.encode(bytes)
}
