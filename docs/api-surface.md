# API surface

How the documented destinations map onto this mock server, and the state of each
stub. Section references (§) point to [`spec/network_calls.md`](spec/network_calls.md).

Because everything runs on one local server, real hostnames collapse onto local
paths. The mapping below is the bridge between the spec and the routes.

| Real host (spec) | Transport | Local route(s) | Handler | State |
|---|---|---|---|---|
| `%s.scauth.rockstargames.com` (§1 #1) | HTTPS | `POST\|GET /scauth[/{*op}]`, `/auth.asmx/{*op}` | `routes::scauth` | Returns fabricated session |
| `%s-%s.ros.rockstargames.com` (§2) | HTTPS | `GET\|POST /{title}/{version}/gameservices/{service}.asmx/{Op}` | `routes::gameservices` | Generic stub; `auth`/`telemetry`/`cloudsave` specialized |
| `cloud.rockstargames.com` (§2.4) | HTTPS | `GET\|PUT\|POST /cloud/{id}/cloudservices/{name}[/{tail}]` | `routes::cloud` | In-memory blob round-trip |
| `https://%s/console/…` (§2.8) | HTTPS | `GET /console/`, `GET\|POST /console/complete`, `GET /console/complete/error` | `routes::console` | Echoes query (incl. `machineId`) |
| `sessiondirectory.xboxlive.com` (§3.1) | HTTPS | `ANY /connections/{*rest}` | `routes::xbox` | MPSD stub |
| `rta.xboxlive.com` (§3.2) | WSS | `GET /connect` (ws) | `routes::rta` | Sample frame + echo |
| Xbox Live stats (§3.3) | HTTPS | `GET\|POST /scids/{scid}/stats/users/{xuid}` | `routes::xbox` | Empty stats stub |
| Xbox Live presence/social (§3.3) | HTTPS | `ANY /users/{*rest}` | `routes::xbox` | Presence/recent-players stub |
| ROS `ros.*` topics (§2.9) | (pub/sub) | `GET /ros/pubsub` (ws) | `routes::rta` | One sample frame per topic + echo |
| Backtrace crash upload (§4) | HTTPS | `POST /crash/upload`, `POST /post` | `routes::crash` | Counts multipart parts, discards |
| — (mock meta) | HTTP | `GET /`, `GET /healthz`, `GET /__/observations` | `routes::meta` | Server info / health / observations |

## Notes per area

### ScAuth / auth (§2.6, §7.1)
The handler returns a session object whose field names follow the spec:
`SessionTicket`, `SessionKey`, `PlayerAccountId`, `RockstarId`, `Privileges`,
`SecsUntilExpiration`, `Region`, `PublicIp`, and a `Services` map of
`<env>-<service>.ros.rockstargames.com` strings. **All values are fabricated.**
Any `ros-*` headers you send are reported back via `/__/observations`.

### ROS gameservices (§7.2)
URL shape `/<title>/<version>/gameservices/<service>.asmx/<Operation>`. The
dispatcher splits `<service>` from `<Operation>` and returns a generic stub,
with light specialization for `auth` (session), `telemetry`
(`SubmitRealTime` ack), and `cloudsave` (points at the `/cloud/...` routes).

### Cloud storage (§2.4)
The spec's builder composes
`/cloud/<id>/cloudservices/<name>/<mode-tail>` with a 5-way mode switch. Here,
`GET` returns a stored blob or a placeholder; `PUT`/`POST` store the opaque body
in memory. Content type is `application/octet-stream`, per §2.7.

### Console link flow (§2.8)
`GET /console/` accepts the documented query template
(`cid`, `scAuthLinkToken`, `playerTicket`, `machineId`, `lang`, `mode`) and
echoes it. `machineId` is flagged in observations as a per-device identifier
(§2.8 / §6).

### Xbox Live (§3)
Standard platform calls, mirrored as stubs. Keyed by SCID/xuid where the spec
is (§3.3). No Xbox SDK behavior is reproduced; privacy gating
(`XblPrivacyCheckPermission*`) is noted in the spec but not simulated.

### Real-time (§2.9, §3.2)
Two websockets. `/connect` models Xbox RTA (one sample presence frame, then
echo). `/ros/pubsub` emits one sample frame for each documented `ros.*` topic so
the control-plane vocabulary is visible, then echoes. Real framing/crypto are
**not** reproduced.

### Crash upload (§4)
Accepts `multipart/form-data`, reports each part's name/filename/size (e.g.
`upload_file_minidump`), and discards the bytes. Nothing is stored or forwarded.

## Not modeled

- Raw TCP/UDP P2P / matchmaking sockets and relays (§5) — out of scope for an
  HTTP mock.
- The telemetry **event catalog** (§7.5) — not recoverable from the spec.
- Any request-signing / body-encryption behavior (§2.6, §8) — intentionally
  excluded; see [`scope-and-ethics.md`](scope-and-ethics.md).
