"""papertriangle capture — a passive mitmproxy addon for privacy auditing.

Purpose
-------
Record *what a client sends* to the ROS / Xbox Live surface so you can measure
the data-collection footprint of the game on your **own device** and a
**lawfully obtained copy**. It is the live-capture companion to the mock server
in this repo and writes the same observation schema, so mock traffic and real
captures read the same way.

What it does
------------
- Observes flows that mitmproxy has already decrypted (because *you* installed
  and trusted the mitmproxy CA on your own device — this addon does not break
  TLS itself).
- Classifies each request by the documented surface (scauth / ROS gameservices /
  cloud / console-link / Xbox Live / RTA / crash).
- Records metadata: method, host, path, which documented ``ros-*`` / platform
  headers were present, content types, and body **sizes**. Request/response
  bodies are **not** stored unless you explicitly opt in with ``pt_bodies``.
- Maps ROS operations to the data categories from the spec (§7.6) so the output
  is directly usable for a privacy writeup.
- Prints and writes a summary on exit.

What it does NOT do (by design — see docs/scope-and-ethics.md)
-------------------------------------------------------------
- It does not modify, replay, re-sign, or inject traffic. It is read-only.
- It does not compute, verify, or forge any ``ros-HeadersHmac`` /
  ``ros-ContentSignature``. ROS bodies are signed/encrypted, so many payloads
  will be opaque — that opacity is itself a finding, recorded as ``opaque``.
- It does not decrypt proprietary app-layer encryption or extract keys.

Usage
-----
    pip install mitmproxy
    mitmdump -s proxy/papertriangle_capture.py \
        --set pt_out=capture.jsonl

Then route the device/console through the proxy and trust the mitmproxy CA on
that device. See proxy/README.md for the full setup and the legal note.

Options
-------
- ``pt_out``        (str)  Output JSONL path. Default: ``papertriangle_capture.jsonl``.
- ``pt_all_hosts``  (bool) Capture every host, not just the ROS/Xbox set. Default: false.
- ``pt_bodies``     (bool) Store (base64) request/response bodies up to the cap. Default: false.
- ``pt_body_cap``   (int)  Max bytes stored per body when ``pt_bodies`` is on. Default: 4096.
"""

from __future__ import annotations

import base64
import json
import logging
from collections import Counter
from typing import Any, Optional

logger = logging.getLogger("papertriangle.capture")

# ---------------------------------------------------------------------------
# Documented header names (mirrors src/security.rs). Observed, never enforced.
# ---------------------------------------------------------------------------
SECURITY_HEADERS = [
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
    "x-xbl-contract-version",
    "User-Agent",
    "Content-Type",
    "Accept-Language",
]

# Host substrings that belong to the surface described in docs/spec/network_calls.md.
DEFAULT_HOST_MATCHES = [
    "ros.rockstargames.com",
    "scauth.rockstargames.com",
    "cloud.rockstargames.com",
    "support.rockstargames.com",
    "rockstargames.com",
    "xboxlive.com",
]

# ROS service -> data category it carries (spec §7.6). Category-level, inferred.
SERVICE_DATA_CATEGORIES = {
    "auth": "device fingerprint / machineId, auth tokens, platform (session establishment)",
    "scauth": "auth tokens / ticketing (session establishment)",
    "telemetry": "gameplay telemetry events (the sampled behavioral metric batch)",
    "profilestats": "player statistics (progression, playtime, counters)",
    "profilestatgroups": "player statistics (grouped)",
    "leaderboards": "score / rank submissions",
    "achievements": "achievement / unlock state",
    "unlocks": "unlock state",
    "cashtransactions": "economy / commerce (currency, purchases)",
    "gametransactions": "economy / commerce (in-game transactions)",
    "commerce": "economy / commerce (store/catalog, entitlements)",
    "catalog": "store catalog",
    "store": "store catalog / purchases",
    "cloudsave": "cloud-save blobs (character / progress data)",
    "ugc": "user-generated content + player-authored data",
    "matchmaking": "session / lobby, who you're playing with",
    "conductor": "matchmaking orchestration",
    "presence": "presence (online status, current title)",
    "posse": "group/session membership",
    "friends": "social graph",
    "inbox": "messages",
    "feed": "activity feed",
    "clans": "crew / clan membership",
    "geolocation": "coarse region / country",
    "playerreporting": "abuse reports / moderation",
    "complaint": "abuse reports",
    "contentmoderation": "moderation",
    "modulate": "voice-chat moderation (voice processed server-side)",
    "ageassurance": "age / consent gating",
    "legalpolicies": "policy acceptance",
    "devicelink": "account / device linking",
    "accounts": "account / profile",
    "socialclub": "account / profile",
    "members": "membership / profile",
    "entitlements": "owned content / DLC / edition",
}


def classify(scheme: str, host: str, path: str) -> dict[str, Any]:
    """Pure classifier: map a request onto the documented surface.

    Returns a dict with at least ``endpoint``; ROS gameservices also get
    ``service``, ``operation`` and (if known) ``data_category``.
    """
    h = host.lower()
    p = path.split("?", 1)[0]

    if "scauth.rockstargames.com" in h:
        return {"endpoint": "scauth", **_maybe_category("scauth")}

    if "cloud.rockstargames.com" in h or "/cloudservices/" in p:
        return {"endpoint": "cloud", **_maybe_category("cloudsave")}

    if "/gameservices/" in p:
        return _classify_gameservices(p)

    if "sessiondirectory.xboxlive.com" in h or p.startswith("/connections"):
        return {"endpoint": "xbox.sessiondirectory"}

    if "rta.xboxlive.com" in h or scheme in ("ws", "wss"):
        return {"endpoint": "rta", "transport": "websocket"}

    if "/console/" in p or p.rstrip("/") == "/console":
        return {"endpoint": "console"}

    if "/scids/" in p and "/stats/" in p:
        return {"endpoint": "xbox.titlestats"}

    if "xboxlive.com" in h or p.startswith("/users/"):
        return {"endpoint": "xbox"}

    if "ros.rockstargames.com" in h:
        return {"endpoint": "ros"}

    return {"endpoint": "other"}


def _classify_gameservices(path: str) -> dict[str, Any]:
    # .../gameservices/<service>.asmx/<Operation>
    try:
        tail = path.split("/gameservices/", 1)[1]
    except IndexError:
        return {"endpoint": "gameservices"}
    service_file, _, operation = tail.partition("/")
    service = service_file.replace(".asmx", "").replace(".svc", "").lower()
    out: dict[str, Any] = {
        "endpoint": "gameservices",
        "service": service,
        "operation": operation,
    }
    out.update(_maybe_category(service))
    return out


def _maybe_category(service: str) -> dict[str, Any]:
    cat = SERVICE_DATA_CATEGORIES.get(service.lower())
    return {"data_category": cat} if cat else {}


def _observed_headers(headers) -> dict[str, str]:
    seen = {}
    for name in SECURITY_HEADERS:
        if name in headers:
            seen[name] = headers[name]
    return seen


def _looks_opaque(content_type: Optional[str], body: bytes) -> bool:
    if not body:
        return False
    ct = (content_type or "").lower()
    if "json" in ct or "text" in ct or "xml" in ct or "form-urlencoded" in ct:
        return False
    # octet-stream, multipart, or anything non-textual: treat as opaque.
    return True


class PaperTriangleCapture:
    def __init__(self) -> None:
        self.out_path = "papertriangle_capture.jsonl"
        self.all_hosts = False
        self.bodies = False
        self.body_cap = 4096
        self._fh = None
        self.host_counts: Counter = Counter()
        self.endpoint_counts: Counter = Counter()
        self.header_counts: Counter = Counter()
        self.category_counts: Counter = Counter()
        self.bytes_up = 0
        self.flows = 0

    # --- mitmproxy hooks ---------------------------------------------------

    def load(self, loader) -> None:
        loader.add_option("pt_out", str, self.out_path, "Capture output JSONL path.")
        loader.add_option("pt_all_hosts", bool, False, "Capture every host, not just ROS/Xbox.")
        loader.add_option("pt_bodies", bool, False, "Store (base64) bodies up to pt_body_cap.")
        loader.add_option("pt_body_cap", int, 4096, "Max bytes stored per body.")

    def configure(self, updated) -> None:
        from mitmproxy import ctx  # imported here so the module is importable without mitmproxy

        self.out_path = ctx.options.pt_out
        self.all_hosts = ctx.options.pt_all_hosts
        self.bodies = ctx.options.pt_bodies
        self.body_cap = ctx.options.pt_body_cap
        if self._fh is None:
            self._fh = open(self.out_path, "a", encoding="utf-8")
            logger.info("papertriangle capture -> %s (bodies=%s)", self.out_path, self.bodies)

    def _in_scope(self, host: str) -> bool:
        if self.all_hosts:
            return True
        h = host.lower()
        return any(m in h for m in DEFAULT_HOST_MATCHES)

    def response(self, flow) -> None:
        req = flow.request
        if not self._in_scope(req.pretty_host):
            return

        info = classify(req.scheme, req.pretty_host, req.path)
        req_body = req.raw_content or b""
        resp = flow.response
        resp_body = (resp.raw_content or b"") if resp else b""

        record: dict[str, Any] = {
            "ts": req.timestamp_start,
            "source": "capture",
            "method": req.method,
            "scheme": req.scheme,
            "host": req.pretty_host,
            "path": req.path.split("?", 1)[0],
            "has_query": "?" in req.path,
            **info,
            "observed_headers": _observed_headers(req.headers),
            "request": {
                "content_type": req.headers.get("Content-Type"),
                "bytes": len(req_body),
                "opaque": _looks_opaque(req.headers.get("Content-Type"), req_body),
            },
            "response": {
                "status": resp.status_code if resp else None,
                "content_type": resp.headers.get("Content-Type") if resp else None,
                "bytes": len(resp_body),
                "opaque": _looks_opaque(
                    resp.headers.get("Content-Type") if resp else None, resp_body
                ),
            },
        }

        if self.bodies:
            record["request"]["body_b64"] = base64.b64encode(req_body[: self.body_cap]).decode()
            record["response"]["body_b64"] = base64.b64encode(resp_body[: self.body_cap]).decode()

        self._write(record)
        self._tally(record)

    # --- bookkeeping -------------------------------------------------------

    def _write(self, record: dict[str, Any]) -> None:
        if self._fh:
            self._fh.write(json.dumps(record, ensure_ascii=False) + "\n")
            self._fh.flush()

    def _tally(self, record: dict[str, Any]) -> None:
        self.flows += 1
        self.host_counts[record["host"]] += 1
        self.endpoint_counts[record.get("endpoint", "other")] += 1
        self.bytes_up += record["request"]["bytes"]
        for name in record["observed_headers"]:
            self.header_counts[name] += 1
        if record.get("data_category"):
            self.category_counts[record["data_category"]] += 1

    def done(self) -> None:
        summary = {
            "source": "capture-summary",
            "flows": self.flows,
            "request_bytes_total": self.bytes_up,
            "by_host": dict(self.host_counts.most_common()),
            "by_endpoint": dict(self.endpoint_counts.most_common()),
            "security_headers_seen": dict(self.header_counts.most_common()),
            "data_categories_seen": dict(self.category_counts.most_common()),
        }
        self._write(summary)
        logger.info("papertriangle capture summary: %s", json.dumps(summary, indent=2))
        if self._fh:
            self._fh.close()
            self._fh = None


addons = [PaperTriangleCapture()]
