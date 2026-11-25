# Architecture

`papertriangle` is a single [axum](https://docs.rs/axum) application. The design
is intentionally small: one router, one shared state object, a handful of route
modules, and no persistence beyond process memory.

```
                      ┌──────────────────────────────────────────┐
  test client ───────▶│  axum Router  (routes::router)            │
  (curl, your own     │                                           │
   tooling)           │  merge(                                   │
                      │    meta  scauth  gameservices  cloud      │
                      │    console  xbox  rta  crash)             │
                      │        │                                  │
                      │        ▼                                  │
                      │  handlers ──▶ security::observe(headers)  │
                      │        │            │                     │
                      │        │            ▼                     │
                      │        │     AppState.record(json)  ──────┼──▶ /__/observations
                      │        ▼                                  │     + tracing logs
                      │  models::mock_* (fabricated JSON)         │
                      └──────────────────────────────────────────┘
```

## Modules

| Module | Responsibility |
|---|---|
| `main.rs` | Init tracing, parse `Config`, call `run`. |
| `lib.rs` | `run()` — bind, build router, serve with graceful shutdown. |
| `config.rs` | `Config` (clap): `--bind`, `--env`, `--title`. |
| `error.rs` | `AppError` + `IntoResponse` (JSON, `"mock": true`). |
| `state.rs` | `AppState`: config, request counter, in-memory cloud map, bounded observation ring. Shared as `Arc<AppState>`. |
| `security.rs` | The documented security-header **names** and `observe()`, which reports which ones a request carried. No enforcement. |
| `models.rs` | Fabricated response builders (`mock_session`, `service_hosts`) and the ROS service-name list. |
| `routes/*` | One module per slice of the documented surface. |

## State & observation

`AppState` is created once and shared via `Arc`. Every handler calls
`AppState::record(json)` with a structured description of what it saw
(endpoint, observed headers, query params, byte counts). Records are:

- emitted to the `papertriangle::observe` tracing target, and
- pushed into a bounded in-memory ring (last 1000), readable at
  `GET /__/observations`.

This is the "observation" purpose of the server: the fabricated responses are
secondary to seeing what a client actually emits.

## Request lifecycle

1. axum matches the path and extracts params/headers/body.
2. The handler builds an observation and calls `AppState::record`.
3. The handler returns a fabricated stub (`models::*` or inline JSON).

No handler performs authentication, signature checks, or outbound I/O.

## Deliberate non-goals

- No TLS. The real surface is HTTPS end-to-end; here, loopback plaintext keeps
  the focus on shapes and makes captures trivial. (Terminate TLS with a local
  proxy if you need to exercise an HTTPS client.)
- No database. Cloud blobs live in a `HashMap` and vanish on exit.
- No signing/crypto. See [`scope-and-ethics.md`](scope-and-ethics.md).
