#!/usr/bin/env python3
"""papertriangle report — turn a capture.jsonl into a readable privacy report.

Offline, stdlib-only. Reads the JSONL produced by ``papertriangle_capture.py``
(or by the mock server's observations, which share the schema) and prints a
Markdown data-collection report: where data goes, what is called, which data
categories and identifiers appear, a required-vs-optional split, and how much of
the traffic is actually readable vs opaque.

Usage
-----
    python proxy/report.py capture.jsonl
    python proxy/report.py capture.jsonl -o report.md

Scope
-----
This only *summarizes* what was already captured. It reads no live traffic,
contacts no host, and decrypts nothing. Identifier claims are derived from the
observed endpoints plus the spec's data table (§6) — they describe what the
documented surface *carries*, not values read out of (opaque) payloads.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter, defaultdict
from typing import Any, Iterable

# Endpoints/services that are functional (needed to play/sign in/save) vs those
# that are analytics/behavioral. This is a documented heuristic, not a verdict.
ANALYTICS_SERVICES = {"telemetry"}
FUNCTIONAL_ENDPOINTS = {
    "scauth",
    "cloud",
    "console",
    "xbox",
    "xbox.sessiondirectory",
    "xbox.titlestats",
    "rta",
    "ros",
}

# Identifiers the documented surface carries, keyed by endpoint/service.
# Source: spec §6 (data summary) and §2.8 (console link). These are what the
# channel is known to carry, independent of whether a given body was readable.
ENDPOINT_IDENTIFIERS = {
    "scauth": [
        "account id (RockstarId / PlayerAccountId)",
        "session tickets / keys",
    ],
    "console": [
        "machineId (per-device identifier)",
        "player auth ticket",
    ],
    "crash": [
        "console id",
        "device model / family / OS version",
    ],
    "cloud": [
        "cloud-save blobs (character / progress)",
    ],
    "xbox": [
        "Xbox user id (xuid)",
        "gamertag / presence",
    ],
    "xbox.titlestats": ["xuid", "title statistics"],
    "xbox.sessiondirectory": ["xuid", "session membership"],
}


def load_records(path: str) -> tuple[list[dict[str, Any]], dict[str, Any] | None]:
    """Return (flow_records, summary_or_None)."""
    flows: list[dict[str, Any]] = []
    summary: dict[str, Any] | None = None
    with open(path, "r", encoding="utf-8") as fh:
        for line_no, line in enumerate(fh, 1):
            line = line.strip()
            if not line:
                continue
            try:
                rec = json.loads(line)
            except json.JSONDecodeError:
                print(f"warning: skipping malformed line {line_no}", file=sys.stderr)
                continue
            if rec.get("source") == "capture-summary":
                summary = rec
            else:
                flows.append(rec)
    return flows, summary


def _req_bytes(rec: dict[str, Any]) -> int:
    return int(rec.get("request", {}).get("bytes", 0) or 0)


def _resp_bytes(rec: dict[str, Any]) -> int:
    return int(rec.get("response", {}).get("bytes", 0) or 0)


def _table(headers: list[str], rows: Iterable[Iterable[Any]]) -> list[str]:
    out = ["| " + " | ".join(headers) + " |",
           "|" + "|".join("---" for _ in headers) + "|"]
    for row in rows:
        out.append("| " + " | ".join(str(c) for c in row) + " |")
    return out


def build_report(flows: list[dict[str, Any]], summary: dict[str, Any] | None) -> str:
    lines: list[str] = ["# Data-collection report", ""]

    if not flows:
        lines.append("_No flow records found in the capture._")
        return "\n".join(lines)

    total = len(flows)
    req_total = sum(_req_bytes(r) for r in flows)
    resp_total = sum(_resp_bytes(r) for r in flows)
    ts = [r["ts"] for r in flows if isinstance(r.get("ts"), (int, float))]
    sources = Counter(r.get("source", "unknown") for r in flows)

    # --- Overview ----------------------------------------------------------
    lines += ["## Overview", ""]
    lines += _table(
        ["Metric", "Value"],
        [
            ["Flows", total],
            ["Request bytes (outbound)", f"{req_total:,}"],
            ["Response bytes (inbound)", f"{resp_total:,}"],
            ["Distinct hosts", len({r.get("host", "?") for r in flows})],
            ["Source(s)", ", ".join(f"{k}={v}" for k, v in sources.items())],
        ]
        + ([["Capture window (s)", f"{max(ts) - min(ts):.1f}"]] if len(ts) >= 2 else []),
    )
    lines.append("")

    # --- Where data goes ---------------------------------------------------
    by_host: Counter = Counter()
    host_bytes: dict[str, int] = defaultdict(int)
    for r in flows:
        by_host[r.get("host", "?")] += 1
        host_bytes[r.get("host", "?")] += _req_bytes(r)
    lines += ["## Where data goes (by host)", ""]
    lines += _table(
        ["Host", "Flows", "Req bytes"],
        [[h, c, f"{host_bytes[h]:,}"] for h, c in by_host.most_common()],
    )
    lines.append("")

    # --- What is called ----------------------------------------------------
    by_endpoint: Counter = Counter(r.get("endpoint", "other") for r in flows)
    lines += ["## What is called (by endpoint)", ""]
    lines += _table(
        ["Endpoint", "Flows"],
        [[e, c] for e, c in by_endpoint.most_common()],
    )
    lines.append("")

    # --- Data categories ---------------------------------------------------
    cat_count: Counter = Counter()
    cat_bytes: dict[str, int] = defaultdict(int)
    for r in flows:
        cat = r.get("data_category")
        if cat:
            cat_count[cat] += 1
            cat_bytes[cat] += _req_bytes(r)
    lines += ["## Data categories observed (spec §7.6)", ""]
    if cat_count:
        lines += _table(
            ["Category", "Flows", "Req bytes"],
            [[cat, c, f"{cat_bytes[cat]:,}"] for cat, c in cat_count.most_common()],
        )
    else:
        lines.append("_No categorized ROS operations in this capture._")
    lines.append("")

    # --- Identifiers on the wire ------------------------------------------
    seen_ids: dict[str, int] = defaultdict(int)
    for r in flows:
        ep = r.get("endpoint", "")
        for ident in ENDPOINT_IDENTIFIERS.get(ep, []):
            seen_ids[ident] += 1
    lines += ["## Identifiers the observed channels carry (spec §6)", ""]
    lines.append(
        "_Derived from which endpoints were seen, not read out of payloads "
        "(many are opaque). These are what the documented surface carries._"
    )
    lines.append("")
    if seen_ids:
        lines += _table(
            ["Identifier", "Flows on channels that carry it"],
            [[i, c] for i, c in sorted(seen_ids.items(), key=lambda kv: -kv[1])],
        )
    else:
        lines.append("_No identifier-bearing endpoints observed._")
    lines.append("")

    # --- Required vs optional ---------------------------------------------
    functional = analytics = unknown = 0
    for r in flows:
        service = (r.get("service") or "").lower()
        ep = r.get("endpoint", "")
        if service in ANALYTICS_SERVICES:
            analytics += 1
        elif ep in FUNCTIONAL_ENDPOINTS or ep == "gameservices":
            functional += 1
        else:
            unknown += 1
    lines += ["## Functional vs analytics (heuristic)", ""]
    lines.append(
        "_Heuristic split: `telemetry` is treated as behavioral/analytics; "
        "auth, cloud, matchmaking, presence, platform calls as functional. "
        "Confirm by toggling in-game telemetry settings and re-capturing._"
    )
    lines.append("")
    lines += _table(
        ["Class", "Flows", "% of total"],
        [
            ["Functional", functional, f"{100 * functional / total:.0f}%"],
            ["Analytics (telemetry)", analytics, f"{100 * analytics / total:.0f}%"],
            ["Unclassified", unknown, f"{100 * unknown / total:.0f}%"],
        ],
    )
    lines.append("")

    # --- Payload opacity ---------------------------------------------------
    req_opaque = sum(1 for r in flows if r.get("request", {}).get("opaque"))
    resp_opaque = sum(1 for r in flows if r.get("response", {}).get("opaque"))
    lines += ["## Payload opacity", ""]
    lines.append(
        "ROS signs/encrypts bodies, so opaque payloads are expected — their "
        "metadata is still informative, but the fields inside are not readable."
    )
    lines.append("")
    lines += _table(
        ["Direction", "Opaque", "Readable", "% opaque"],
        [
            ["Request bodies", req_opaque, total - req_opaque, f"{100 * req_opaque / total:.0f}%"],
            ["Response bodies", resp_opaque, total - resp_opaque, f"{100 * resp_opaque / total:.0f}%"],
        ],
    )
    lines.append("")

    # --- Security headers --------------------------------------------------
    hdr_count: Counter = Counter()
    for r in flows:
        for name in r.get("observed_headers", {}):
            hdr_count[name] += 1
    lines += ["## Documented security headers seen", ""]
    if hdr_count:
        lines += _table(
            ["Header", "Flows"],
            [[h, c] for h, c in hdr_count.most_common()],
        )
    else:
        lines.append("_None of the documented headers were observed._")
    lines.append("")

    # --- Caveats -----------------------------------------------------------
    lines += [
        "## Caveats",
        "",
        "- Identifier rows describe what each channel is *documented to carry* "
        "(spec §6), not values extracted from payloads.",
        "- The functional/analytics split is a heuristic; verify by toggling "
        "telemetry settings and diffing captures.",
        "- Opaque bodies mean the exact telemetry event catalog is not readable "
        "from the wire (spec §7.5/§8).",
        "- Only audit a lawfully owned copy, your own account, and your own "
        "device/network.",
    ]

    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Turn a capture.jsonl into a Markdown privacy report.")
    parser.add_argument("capture", help="Path to capture.jsonl")
    parser.add_argument("-o", "--out", help="Write the report here instead of stdout")
    args = parser.parse_args(argv)

    # Ensure UTF-8 stdout so the section markers render on any console.
    try:
        sys.stdout.reconfigure(encoding="utf-8")
    except Exception:
        pass

    flows, summary = load_records(args.capture)
    report = build_report(flows, summary)

    if args.out:
        with open(args.out, "w", encoding="utf-8") as fh:
            fh.write(report + "\n")
        print(f"wrote {args.out} ({len(flows)} flows)", file=sys.stderr)
    else:
        print(report)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
