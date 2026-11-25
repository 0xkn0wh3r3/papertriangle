# papertriangle

A **mock / observation server**, written in Rust, that mirrors the *documented*
network surface of a ROS-style online backend as described in
[`docs/spec/network_calls.md`](docs/spec/network_calls.md).

Its job is to help you **understand and document how the protocol surface is
shaped** — nothing more. It serves fabricated responses, logs what a client
sends, and never contacts a live service.

> **Scope matters here.** Please read [`docs/scope-and-ethics.md`](docs/scope-and-ethics.md)
> before extending this. This project deliberately stops at the *shape* of the
> surface. It does **not** implement the proprietary request-signing crypto,
> authentication, or anything that would let it stand in for a real backend or
> let a client reach one.

## What it does

- Exposes HTTP routes whose **paths, header names, and field names** match the
  spec (ScAuth, ROS gameservices, cloud storage, console-link flow, Xbox Live
  platform calls, RTA/pub-sub websockets, crash upload).
- Returns clearly-fabricated stub responses (every JSON body carries
  `"_mock": true`).
- **Records every request** — which documented security headers were presented,
  which query params, how many bytes — and exposes them at `GET /__/observations`
  and in the logs. This is the "observation" half: you can point a test client
  at it and see exactly what it emits.

## What it does NOT do

- No real authentication; `SessionTicket`/`SessionKey` are random bytes.
- No `ros-HeadersHmac` / `ros-ContentHash` / `ros-ContentSignature` computation
  or verification (the algorithms/keys aren't in the spec — see §8 of it).
- No connections to `*.rockstargames.com`, `*.xboxlive.com`, Backtrace, or any
  other host. It binds **loopback only** by default.
- No DRM/anti-cheat circumvention, no game-traffic relaying.

## Run it

```powershell
cargo run
```

Then, in another shell:

```powershell
# Landing page describing the mock and its routes
curl http://127.0.0.1:8080/

# Fabricated ScAuth ticket (send some ros-* headers to see them observed)
curl -X POST http://127.0.0.1:8080/scauth/auth.asmx/CreateTicketScAuthToken2 `
  -H "ros-SessionTicket: demo" -H "ros-SecurityFlags: 239"

# Generic ROS gameservices shape
curl http://127.0.0.1:8080/gta6/1/gameservices/matchmaking.asmx/Find

# Cloud round-trip (store then fetch an opaque blob)
curl -X PUT  --data-binary "hello" http://127.0.0.1:8080/cloud/42/cloudservices/save0
curl         http://127.0.0.1:8080/cloud/42/cloudservices/save0

# See what the server observed
curl http://127.0.0.1:8080/__/observations
```

Configuration (flags or env vars): `--bind`/`PT_BIND`, `--env`/`PT_ENV`,
`--title`/`PT_TITLE`. See `cargo run -- --help`.

## Status

Early scaffold / stubs. See [`docs/api-surface.md`](docs/api-surface.md) for the
endpoint-by-endpoint state.

## License

MIT — see [LICENSE](LICENSE).
