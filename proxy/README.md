# Capture proxy

A passive [mitmproxy](https://mitmproxy.org/) addon that records what a client
sends to the ROS / Xbox Live surface, so you can **measure the game's
data-collection footprint** on your own device. It is the live-capture companion
to the mock server in this repo and writes the **same observation schema**, so
captures and mock runs read alike.

> Read [`../docs/scope-and-ethics.md`](../docs/scope-and-ethics.md) first. This
> addon is **read-only**: it observes and records, it never modifies, replays, or
> re-signs traffic, and it computes no signatures.

## What you get

Per in-scope flow, one JSON line with:

- `method`, `host`, `path`, and the documented endpoint classification
  (`scauth` / `gameservices` / `cloud` / `console` / `xbox*` / `rta`);
- for ROS gameservices, the `service`, `operation`, and the **data category**
  that operation carries (from spec §7.6) — e.g. `telemetry` →
  *"gameplay telemetry events"*;
- `observed_headers` — which documented `ros-*` / platform headers were present;
- request/response **content types and byte sizes**, and an `opaque` flag when a
  body isn't textual (ROS bodies are signed/encrypted, so expect many opaque
  ones — that's a finding, not a failure).

Bodies are **not** stored unless you pass `--set pt_bodies=true`.

On exit it appends a `capture-summary` line: flows per host, per endpoint, which
security headers appeared, and which data categories were observed.

## Setup

1. Install:

   ```bash
   pip install -r proxy/requirements.txt
   ```

2. Run the proxy with the addon:

   ```bash
   mitmdump -s proxy/papertriangle_capture.py --set pt_out=capture.jsonl
   ```

3. Point the device you're auditing at the proxy (system/console network
   settings → manual proxy → this machine's IP:8080) and **install + trust the
   mitmproxy CA on that device** (`http://mitm.it` from the proxied device). TLS
   is decrypted only because *you* trust that cert on *your* device; the addon
   does not break TLS.

4. Play / exercise the client, then stop mitmdump (`q`) and inspect
   `capture.jsonl` and the summary.

### Options

| Option | Default | Meaning |
|---|---|---|
| `pt_out` | `papertriangle_capture.jsonl` | Output JSONL path |
| `pt_all_hosts` | `false` | Capture every host, not just the ROS/Xbox set |
| `pt_bodies` | `false` | Store base64 bodies up to the cap |
| `pt_body_cap` | `4096` | Max bytes stored per body |

## Reading the output

```bash
# Which data categories showed up, by volume
grep '"source": "capture"' capture.jsonl \
  | jq -r 'select(.data_category) | .data_category' | sort | uniq -c | sort -rn

# Telemetry calls only (the behavioral batch — spec §7.6)
jq 'select(.service=="telemetry")' capture.jsonl

# The final summary
grep '"source": "capture-summary"' capture.jsonl | jq
```

## Generate a report

Turn a capture into a Markdown data-collection report (offline, stdlib-only):

```bash
python proxy/report.py capture.jsonl            # to stdout
python proxy/report.py capture.jsonl -o report.md
```

It summarizes: where data goes (by host), what is called (by endpoint), the data
categories observed (§7.6), which identifiers the observed channels are
documented to carry (§6), a functional-vs-analytics heuristic split, payload
opacity (how much is readable), and which documented security headers appeared.
Identifier rows describe what each channel *carries* per the spec, not values
read out of (often opaque) payloads.

## Limits (state these in any writeup)

- **Opaque payloads.** ROS hashes + signs bodies and HMACs headers; the exact
  telemetry **event catalog** is not on the wire in readable form. You'll get
  hosts, headers, timing, sizes, and metadata — not decrypted event fields.
- **Certificate pinning.** A hardened client may pin its CA and refuse a
  mitmproxy cert; then only connection metadata is visible, not decrypted
  content. Do not attempt to defeat pinning — note it as a limit.
- **Scope.** Audit only a copy you lawfully own, your own account, and your own
  device/network. See the legal note in `../docs/scope-and-ethics.md`.

## Legal note (not legal advice)

Reverse engineering for interoperability, security, and privacy research is
treated differently across jurisdictions, and the publisher's Terms of Service
may restrict capture of its traffic. This tooling is for informing people about
data collection on software they own — not for circumventing access controls or
substituting for the service. Confirm what applies to you before relying on it.
