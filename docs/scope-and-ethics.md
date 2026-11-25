# Scope & ethics

This document is the contract for what `papertriangle` is and is not. It exists
because the source material (`docs/spec/network_calls.md`) is a static-analysis
writeup of a commercial game's online backend, and it is easy for a project like
this to drift from "understand the surface" into "reimplement the service."

## Purpose

To **understand and document** the *shape* of the observed network surface:
which hosts exist, what the URL paths look like, which headers and fields are
named, and what a client emits. A running mock is a convenient way to study that
shape — you can point a test client at it and watch what it sends.

## Hard boundaries (the project will not cross these)

1. **No proprietary crypto.** The ROS request-signing scheme
   (`ros-HeadersHmac`, `ros-ContentHash`, `ros-ContentSignature`, the
   session-ticket/`SessionKey` mechanism) is **not implemented**. The spec
   itself notes these algorithms and keys are *not knowable from static
   analysis* (§8). This server only knows the header **names**, and only so it
   can report which ones a client sent. It never computes, verifies, or forges a
   signature.

2. **No live hosts.** The server binds loopback by default and never contacts
   `*.rockstargames.com`, `*.xboxlive.com`, Backtrace, or any other real
   endpoint. It is not a proxy and it is not a client.

3. **Not a backend emulator.** Responses are fabricated stubs. The project will
   not grow into something an unmodified retail client can authenticate against
   or treat as a stand-in for the real service.

4. **No circumvention.** Nothing here targets DRM, anti-cheat, account/license
   enforcement, or matchmaking/game-traffic relaying.

## Why these lines

- A localhost stub that mirrors documented shapes and logs inputs is ordinary
  interoperability/research work — the same thing you do when you build a mock of
  any third-party API you integrate with or study.
- A tool that produces *authentication-valid* requests, or that reaches the live
  service, is a different thing entirely: it enables unauthorized access to a
  commercial service. That is out of scope here regardless of intent, and the
  pieces needed to build it are intentionally absent.

## Legal note (not legal advice)

The source spec is derived from static analysis of a client binary. Reverse-engineering work like this may be restricted by the publisher's Terms of Service
(§7, §8). Reverse engineering for interoperability, security, and privacy
research is treated differently across jurisdictions. If you intend to publish
findings or capture live traffic, confirm you are working from a lawfully
obtained copy and understand the terms that apply to you. This document is not
legal advice.

## If you're tempted to extend it

Good extensions: richer fabricated response shapes, more faithful path
coverage, a schema for observations, a test client that exercises the mock,
export of observations for analysis.

Out of scope: real signing, real hosts, anything that makes this usable as a
substitute backend or as a client against one.
