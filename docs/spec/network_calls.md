# Network Call Inventory: the client binary

*Every external network destination and call pattern recoverable by **static analysis** of the binary (strings, URL/header templates, import tables, and the telemetry/auth code already disassembled). The binary was never executed.*

> **Read this first — what "static" means for payloads.** Hostnames, URL templates, header names, content types, and many field names are present as literals in the binary and are reported here with high confidence. **Exact request bodies, methods, and header values are assembled at runtime** (often from a server-pushed config) and are **TLS-encrypted** on the wire, some additionally signed/encrypted at the app layer. Where a method or body is marked *(inferred)*, confirm it with a live capture on a lawfully obtained copy. `%s` / `%x` are runtime-filled placeholders.

Transport libraries in use (from imports): **WinHTTP** (`WinHttpOpen/Connect/SendRequest/…`), **libHttpClient.GDK** (Xbox `HCHttpCall*`), **WinSock2** (`WS2_32` — TCP/UDP), **WinHTTP WebSocket/`wss://`** for Xbox real-time activity. Crash upload uses WinHTTP directly (Breakpad).

---

## 1. Destinations at a glance

| # | Host (template) | Operator | Transport | Purpose |
|---|---|---|---|---|
| 1 | `%s.scauth.rockstargames.com` | Rockstar | HTTPS | Social Club authentication (ScAuth) |
| 2 | `%s-%s.ros.rockstargames.com` | Rockstar | HTTPS | Rockstar Online Services — game backend APIs |
| 3 | `cloud.rockstargames.com` | Rockstar | HTTPS | Cloud saves / config / assets |
| 4 | `support.rockstargames.com` | Rockstar | HTTPS | In-game support/error messages |
| 5 | `https://%s/console/…` | Rockstar (Social Club web) | HTTPS | Console↔account link flow |
| 6 | `sessiondirectory.xboxlive.com` | Microsoft | HTTPS | Multiplayer session directory |
| 7 | `rta.xboxlive.com` (`wss://…/connect`) | Microsoft | WSS (WebSocket) | Real-Time Activity (presence, notifications) |
| 8 | Xbox Live services (via libHttpClient) | Microsoft | HTTPS | Profile, achievements, stats, privacy, presence |
| 9 | Backtrace (via Breakpad uploader) | Rockstar/Backtrace | HTTPS | Crash minidump upload |

No third-party advertising/analytics hosts are present. `adobe.com`, `w3.org`, `example.com`, `flash.net`, `17.tv` appear only as XML-namespace / Scaleform-ActionScript boilerplate and are **not** contacted as endpoints.

---

## 2. Rockstar Online Services (ROS) — the main backend

### 2.1 Hosts
- `%s-%s.ros.rockstargames.com` — the two `%s` are an **environment/prefix** and a **service name** (e.g. `prod-<service>.ros…`), so each ROS service is its own subdomain.
- `%s.scauth.rockstargames.com` — authentication frontend.
- `cloud.rockstargames.com` — cloud storage (`%s.%s` built just before it, i.e. `<something>.cloud.rockstargames.com`).

### 2.2 Host structure (disassembled — `sub_141a75fa0`, `sub_143dcd910`)
ROS is a family of per-service subdomains. The host builder assembles:
```
<env>-<service>.ros.rockstargames.com      (e.g. prod-<service>.ros…)
<service>.ros.rockstargames.com            (fallback when the env prefix is empty)
<prefix>.cloud.rockstargames.com           (cloud storage frontend)
<prefix>.scauth.rockstargames.com          (auth frontend)
```
- The `<env>` slot (`prod`/`test`/`dev` candidates are present as strings) selects the backend environment; `<service>` selects the ROS service. So each ROS capability lives at its own subdomain, chosen at runtime.

### 2.3 ROS service names (candidates — strings present; subdomain mapping inferred)
The binary contains these service-name tokens that plug into the `<service>` slot or appear as ROS sub-APIs. Mapping each to its exact subdomain needs a live capture, but the names themselves are literals:
```
scauth        authentication / ticketing        (confirmed host: *.scauth…)
cloud         cloud saves & config files         (confirmed host: *.cloud…, see 2.4)
ugc           user-generated content
store         commerce / catalog / purchases
feed          activity / news feed
email         account email / messaging
members       membership / profile
achievements  achievements (also surfaces via Xbox Live, §3)
```

### 2.4 Cloud storage path structure (disassembled — `sub_143ebde80`)
Cloud files are addressed by a builder that composes:
```
https://<prefix>.cloud.rockstargames.com/cloud/<id>/cloudservices/<name>/<mode-specific tail>
   <id>    numeric id (title/config version, from an object field +0x14)
   <name>  resource name
   <mode>  a 5-way switch (ebp 0..4) picks the tail form — i.e. 5 kinds of cloud resource
```
> **Reading:** this is Rockstar's cloud-save / cloud-config store (the same mechanism GTA Online uses for character and progress blobs). The 5 modes are different resource classes; the body for writes is `application/octet-stream` (opaque blob) or `multipart/form-data` for file uploads.

### 2.5 Path segments observed (ROS)
```
/cloud/            /cloudservices/        cloud store (see 2.4)
/console/          /console/complete/     console↔account link (see 2.8)
/console/complete/error
```
*(The `/scids/`, `/stats/users/`, `/titles/`, `/recentplayers`, `/users/xuid(` paths are **Xbox Live**, not ROS — moved to §3.3.)*

### 2.6 ROS security header scheme (high-confidence — all are literals)
ROS signs each request at the application layer. These header names are present:

| Header | Role (inferred from name) |
|---|---|
| `ros-SessionTicket` | the session ticket from auth (see `telemetry_disassembly_notes.md` §E) |
| `Scs-Ticket` / `ScAuthToken` | Social Club service ticket / auth token |
| `ros-Challenge` | anti-replay challenge value |
| `ros-HeadersHmac` | HMAC over the request headers |
| `ros-ContentHash` | hash of the request body |
| `ros-ContentSignature` | signature over the body |
| `ros-SecurityFlags` | bitfield controlling which protections apply |
| `SCS-RequestId` | per-request correlation id |
| `Authorization` | standard auth header (scheme assembled at runtime) |
| `Content-Type`, `Content-Length`, `Accept-Language`, `User-Agent`, `Transfer-Encoding` | standard |

> **Reading:** request bodies are hashed and signed and the headers HMAC'd, so even with TLS stripped on a test box, tampering is detectable and bodies may be opaque. This is anti-cheat/anti-fraud hardening, and it's why a payload capture needs a legitimately provisioned client.

### 2.7 Content types used
```
application/json; charset=utf-8
application/x-www-form-urlencoded; charset=utf-8
application/octet-stream              (binary: cloud files, tickets)
multipart/form-data; boundary=%s      (file uploads)
  Content-Disposition: form-data; name="fileField"; filename="%s"
```

### 2.8 Auth / console-link flow (host `#5`, disassembled in notes §F)
`GET https://%s/console/` then a completion POST, with this query template:
```
?cid=%s&scAuthLinkToken=%s&playerTicket=%s&machineId=%s&lang=%s&mode=%s
    cid               console/client id
    scAuthLinkToken   one-time link token
    playerTicket      player auth ticket
    machineId         per-device identifier   ← device identifier on the wire
    lang / mode       locale / flow mode
```
Error path: `/console/complete/error`.

### 2.9 ROS real-time message topics (pub/sub — intent per topic)
These are event/topic names carried over the ROS connection (the `ros.*` namespace), not distinct hosts. Together they describe what ROS pushes to the client and what it reacts to:

| Topic | Direction (inferred) | What it's for |
|---|---|---|
| `ros.publish` | client→server | publish an event/message onto the bus |
| `ros.msgs` | server→client | inbound message delivery (the inbox) |
| `ros.ticket.renew` | server→client | session ticket is near expiry → re-auth |
| `ros.ticket.invalidate` | server→client | **force-logout / revoke** the current ticket (ban, policy, or sign-in elsewhere) |
| `ros.account.link` / `ros.account.unlink` | server→client | the console↔Social Club link changed |
| `ros.entitlement.changed` | server→client | owned content / DLC / edition entitlement changed |
| `ros.sc.membership.changed` | server→client | Social Club membership tier changed (e.g. GTA+) |
| `ros.cloud.file.changed` | server→client | a cloud-save/config file changed elsewhere → re-sync |
| `ros.attrs.refresh` | server→client | re-fetch server-controlled account attributes/flags |

> **Intent reading:** this channel is the live control plane for the account: Rockstar can renew or revoke your session, flip entitlements and membership, force a cloud re-sync, and refresh account flags in real time while you play. It's consistent with license/anti-fraud enforcement and live-service content gating, not with harvesting data off the device.

---

## 3. Microsoft Xbox Live (platform — standard for any online Xbox game)

### 3.1 Session directory (host `#6`)
```
POST/PUT/GET https://sessiondirectory.xboxlive.com/connections/…
Header: x-xbl-contract-version
```
Multiplayer session creation/join/membership (MPSD).

### 3.2 Real-Time Activity (host `#7`)
```
wss://rta.xboxlive.com/connect            (also "rta.xboxlive.com.V2")
```
WebSocket for presence and live notifications.

### 3.3 Xbox Live service calls via libHttpClient (`HCHttpCall*`)
Resolved by the GDK at runtime (hosts not literal in the binary). These hit the standard `*.xboxlive.com` service family. The path fragments are keyed by **SCID** (Service Configuration ID) and **xuid** (Xbox user id) — both Xbox Live concepts, which is how we know these are platform calls, not ROS:

| Path fragment (disassembled) | Service | Purpose |
|---|---|---|
| `/scids/<scid>/stats/users/<xuid>` (`sub_14563d310`) | title stats | read/write the player's title statistics |
| `/users/xuid(<id>)/devices/current/titles/current` (`sub_145626c80`) | presence/title | report the title currently running on this device |
| `/users/xuid(<id>)/.../recentplayers` + `/titles/` (`sub_14563ef20`) | social | recent-players list |

Operations surfaced through the Xbox SDK:
```
XblAchievementsGetAchievementsForTitleIdAsync / UpdateAchievementForTitleIdAsync
XblPresenceGetPresenceAsync / SetPresenceAsync
XblPrivacyCheckPermissionAsync / BatchCheckPermissionAsync
```
All presence/social sharing is gated by `XblPrivacyCheckPermission*`, so it respects the console's privacy settings. `achievements` also appears as a ROS service name (§2.3), i.e. Rockstar mirrors achievement/stat state on its own backend in addition to Xbox Live.

---

## 4. Crash reporting (host `#9` — Breakpad → Backtrace)

```
Transport:   WinHTTP (WinHttpOpen/Connect/OpenRequest/SendRequest/ReadData)
User-Agent:  Backtrace-breakpad minidump uploader/0.1
Body:        multipart/form-data
  file part:  upload_file_minidump  (the .dmp)
  attributes (UTF-16 keys, from disassembly of sub_1455fe642):
     xbox.id            console id (XSystemGetConsoleId)
     guid               report instance id
     xbox.device_type   xbox.family
     xbox.os.version    xbox.os.host_version
     device.model
     parameter "%s" = "%s"   arbitrary key/value context pairs
```
The destination URL is supplied at runtime (logged via the `BacktraceBreakpad:%s:%d:URL: %S` trace string), not hard-coded as a literal host. Minidumps can contain incidental process-memory fragments.

---

## 5. Raw sockets (WS2_32)

The game uses TCP and UDP directly (`socket`, `connect`, `bind`, `listen`, `accept`, `send`/`recv`, `sendto`/`recvfrom`, `select`) for **peer-to-peer / matchmaking game traffic** (session data, voice, state sync), separate from the HTTPS control plane. Peers and relays are chosen at runtime; the relay list (with `Host`, `CountryCode`, `RegionCode`, `Latitude`/`Longitude` of the **relays**, `PublicKey`) is delivered via ROS — see `telemetry_findings.md` §3. `gethostname`+`getaddrinfo` are used only to discover the **console's own** local IP.

---

## 6. What rides on these calls (data summary, cross-referenced)

| Data | Where it goes | Call |
|---|---|---|
| Account id (`RockstarId`, `PlayerAccountId`), email, zip, nickname, country, language, age | Rockstar ROS | §2 account/session serializers |
| Session tickets/keys, `PublicIp`, `Region`, privileges | Rockstar ROS | §2.3 headers + session object |
| `machineId` (per-device) | Rockstar (Social Club link) | §2.5 query string |
| Console id + device model/family/OS | Backtrace (crash) | §4 |
| Xbox user id (xuid), gamertag, presence, achievements, stats | Microsoft Xbox Live | §3 |
| Telemetry metrics (server-defined catalog) | Rockstar ROS | pipeline in `telemetry_findings.md` + `…disassembly_notes.md` §A |
| Gameplay/session/voice (P2P) | Peers / relays | §5 |

---

## 7. Cross-check against public ROS reverse-engineering

There is no official Rockstar documentation for ROS, but the GTA modding / private-server community has reverse-engineered the GTA V–era stack. Comparing their **measured-on-the-wire** results against our **static** findings is a strong validation — and it also exposes a few GTA VI-era changes. Sources are community/unofficial (see footnotes); treat them as corroboration, not authority.

### 7.1 What matches (our static findings confirmed by observed traffic)

| Our static finding | Community (measured) | Verdict |
|---|---|---|
| Headers `ros-SecurityFlags`, `ros-SessionTicket`, `ros-Challenge`, `ros-HeadersHmac` | SocialClub Plus lists all four as **Measured**, incl. `ros-SecurityFlags: 239` and `ros-Challenge` = 8 random bytes Base64; `ros-HeadersHmac` = HMAC-SHA1 over method+path+flags+ticket+challenge | **Confirmed** — exact header names and semantics |
| Host pattern `<env>-<service>.ros.rockstargames.com` | Real hosts `gta5-prod.ros`, `ugc-gta5-prod.ros`, `mm-gta5-prod.ros`, `prs-gta5-prod.ros`, `prod.ros` — per-service subdomains with env | **Confirmed** — same scheme |
| Separate `cloud.rockstargames.com` with `/cloud/.../cloudservices/` | `prod.cloud.rockstargames.com` serves job files, tunables, crew emblems | **Confirmed** |
| Session object: `SessionTicket`, `SessionKey`, `PlayerAccountId`, `Privileges`, `SecsUntilExpiration`, `Region`, per-service `Host` | Ticket response carries session key, account ids, `Privileges`, a host per service, ~1-day lifetime | **Confirmed** field-for-field |
| `machineId` on the auth/link flow | Ticket request carries a machine fingerprint (`SCS-MFAMachineFingerPrint`) + `rememberedMachineToken`; fingerprint = device name, machine name, OS version, volume serial, CPU | **Confirmed** (ours is the console analogue) |
| Bodies hashed + signed (`ros-ContentHash`/`ContentSignature`), headers HMAC'd | Form body RC4-encrypted per-request, keyed with the session key; HMAC-SHA1 trailer | **Confirmed** — our "signed & opaque" inference is correct; community has the exact mechanism |
| Service-name candidates `ugc`, `feed`, `achievements`, `cloud`, `store`, `members`, `scauth` | Qrix endpoint dump: `ugc.asmx`, `Feed`, `Achievements.asmx`, `cloudsave.asmx`, `Commerce`, `accounts`/`socialclub`, `auth.asmx` | **Confirmed** (names refined — see 7.3) |
| Telemetry goes to a ROS service | Qrix: `Telemetry.asmx/SubmitRealTime` | **Confirmed** — matches our `telemetrycache.dat` + sampling pipeline |
| `ros.ticket.invalidate` = force-logout | One session per account; a new ticket returns `AuthenticationFailed/DupLogin` and signs out the running session | **Confirmed** behavior |

### 7.2 The full ROS service catalog (from the community `endpoints` dumps)

Our static pass surfaced ~8 service names; the community binary dumps list the fuller `*.asmx`/`*.svc` set, which is what each `<service>` subdomain resolves to. Observed GTA-family services include:
```
auth, socialclub, accounts, entitlements, matchmaking, Conductor (matchmaking orchestration),
Presence, Friends, Inbox, Feed, ProfileStats/ProfileStatGroups, Leaderboards, Achievements,
Unlocks, cloudsave, ugc, Commerce/CashTransactions/GameTransactions, Catalog/store,
GeoLocation, AgeAssurance, Clans, PlayerReporting/Complaint/ContentModeration, DeviceLink,
SaveMigration, legalpolicies, licenseplates, Telemetry, Posse (RDR), playersearch
```
URL shape (GTA V): `/<title>/<version>/gameservices/<service>.asmx/<Operation>` — e.g. `auth.asmx/CreateTicketScAuthToken2`, `cloudsave.asmx/GetFile`, `matchmaking.asmx/Find`, `ugc.asmx/QueryContent`, `Telemetry.asmx/SubmitRealTime`. GTA VI will differ in title/version and likely service mix, but the architecture is the same.

### 7.3 Where GTA VI appears to differ / things to verify

- **New-looking headers:** `ros-ContentHash`, `ros-ContentSignature`, `SCS-RequestId`, and `Scs-Ticket` are in *our* binary but are **not** in the GTA V community write-ups. These look like GTA VI-era additions — plausibly a stronger body-integrity scheme (separate content hash + signature) and explicit request correlation. **Verify on the wire.**
- **Crypto may have moved on:** the community documents RC4 + HMAC-**SHA1** for GTA V. Whether GTA VI kept RC4/SHA-1 or upgraded (e.g. AES/SHA-256) is **not** determinable from strings. The presence of `bcrypt` (`BCryptGenRandom`/algorithm-provider APIs) in our import table is consistent with a modern crypto path but doesn't prove the request-signing algorithm.
- **Content-signature vs. RC4-only:** GTA V's scheme encrypted+HMAC'd the body; a distinct `ros-ContentSignature` suggests asymmetric signing may have been added. Unconfirmed.
- **`achievements` on both ROS and Xbox Live:** community shows `Achievements.asmx` on ROS; we also see Xbox Live achievement APIs. Rockstar mirrors achievement/stat state on its own backend — consistent across both.

### 7.4 Why this cross-check matters for a publication

Every privacy-relevant claim we made from the binary (device identifier on auth, account PII in the session record, signed/opaque bodies, telemetry-to-ROS, real-time ticket revocation) is independently corroborated by people who captured the actual traffic on the prior title. That moves those claims from "inferred from strings" toward "consistent with observed ROS behavior" — while the GTA VI-specific deltas above are exactly what a fresh live capture should target.

*Sources (unofficial, community reverse-engineering — Rockstar's ToS likely restricts this work; verify before relying on it):* [SocialClub Plus — how Social Club / ROS works](https://socialclubplus.com/docs), [Qrix/ROS-Endpoints (endpoint dumps from R\* binaries)](https://github.com/Qrix/ROS-Endpoints), [BugisoftRSG/SCAPI (GTA V prod.ros API)](https://github.com/BugisoftRSG/SCAPI), and FiveM's `ros-patches-five` (referenced by SocialClub Plus for the signing scheme).

### 7.5 Attempt to read the data categories from the game data files

I examined a partial game-data download (~193 GB, mostly `.rpf` archives + `.ttpf` streaming packs) to try to recover the actual telemetry **event catalog** (the field/metric names each request sends).

**Result: the archives are opaque to static extraction.**
- Every archive is **RPF9** (magic `39 46 50 52` = "RPF9", GTA VI's package format). Mid-file entropy is ~8.0 for `data.rpf`/`appdata0.rpf`/`Catalog000.bin` (encrypted) and the lower-entropy `script.rpf`/`common.rpf` still yielded **zero** readable filenames or schema tokens — i.e. the RPF9 **table of contents is encrypted** and entries are compressed/encrypted individually.
- A filename/TOC scan across `data.rpf`, `common.rpf`, `script.rpf`, `appdata0.rpf` returned **0 filenames and 0 telemetry/stat tokens**. Plain `strings` gets nothing.
- `textures.rpf` (and other large packs) are a **sparse/partial download** (long zero runs), so some content isn't present at all.
- There is **no public RPF9 extractor** for GTA VI yet, and the decryption keys are derived inside the title at runtime, not stored in the data files. So the event catalog cannot be read from this download without a working RPF9 toolchain + keys.

**What this means:** the game-data files do not add schema detail on top of the binary. The best available answer to "what *kinds* of data are sent" therefore comes from the **ROS operation names** (community binary dumps) plus our binary findings — mapped below.

### 7.6 Data categories by ROS operation (what each call is sending)

Grounded in the observed operation names (§7.2) and our disassembled serializers. The **operation name** is factual (from community dumps); the **data sent** is inferred from the name + our field-level findings. This is category-level, not the exact wire schema.

| ROS operation (observed) | Data category it sends |
|---|---|
| `auth.asmx/CreateTicketScAuthToken2` | device fingerprint/`machineId`, auth tokens, platform — establishes session |
| `Telemetry.asmx/SubmitRealTime` | **gameplay telemetry events** — the sampled metric batch (server-defined names); the one most relevant to privacy |
| `ProfileStats.asmx/ReadStatsByGamer2`, `ProfileStatGroups` | player statistics (progression, playtime, per-stat counters) |
| `Leaderboards.asmx/*` | score/rank submissions |
| `Achievements.asmx/AwardAchievements`, `Unlocks` | achievement/unlock state |
| `CashTransactions`, `GameTransactions`, `Commerce`, `Catalog` | **economy/commerce** — purchases, in-game currency, store/catalog, entitlements |
| `cloudsave.asmx/GetFile`/`SetFile` | cloud-save blobs (character/progress data) |
| `ugc.asmx/*` (`QueryContent`, `SetPlayerData`, `CreateContent2`) | user-generated content + player-authored data |
| `matchmaking.asmx/Find`, `Conductor`, `Presence`, `Posse` | session/lobby, presence, who you're playing with |
| `Friends`, `Inbox`, `Feed`, `Clans`/`Crews` | social graph, messages, activity feed |
| `GeoLocation` | coarse region/country (not GPS — see `telemetry_findings.md`) |
| `PlayerReporting`, `Complaint`, `ContentModeration`, `Modulate` | abuse reports, moderation (incl. voice moderation — `Modulate` is a voice-moderation vendor) |
| `AgeAssurance`, `legalpolicies` | age/consent gating, policy acceptance |
| `DeviceLink`, `accounts`, `socialclub` | account/device linking, profile |

> **Privacy reading:** the category that actually carries behavioral data is `Telemetry.asmx/SubmitRealTime`; the rest are functional (auth, saves, commerce, social, moderation). `Modulate` is notable — it indicates **voice-chat moderation** (voice may be processed server-side for moderation), which a privacy writeup should call out and confirm with a live capture. The exact telemetry event list remains the one missing piece, recoverable only via `telemetrycache.dat` at runtime or a traffic capture.

---

## 8. Confidence & limits (include in any publication)

- **High confidence:** the host list, path segments, header names, content types, and the crash-report schema — all are literal strings / disassembled code.
- **Inferred:** HTTP methods, exact body layouts, header *values*, and the precise service behind each `%s` subdomain. These are runtime/config-driven.
- **Not knowable statically:** the full telemetry event catalog. It lives in server config and game-script data; the game data files are **RPF9 with an encrypted TOC + packed entries** (verified on the partial download, §7.5), so it can't be read from disk without an RPF9 toolchain and runtime keys. Also anything resolved dynamically via `GetProcAddress`.
- **Encryption:** everything is over TLS; ROS additionally hashes+signs bodies and HMACs headers (`ros-ContentHash/Signature/HeadersHmac`), so capturing readable payloads requires a legitimately provisioned client and a TLS-intercept test setup.
- **Provenance:** corroborate with a capture from a lawfully obtained copy before publishing.

### To turn this into ground truth
1. TLS-intercept (mitmproxy + trusted cert) on a retail console/devkit; map each `%s-%s.ros.rockstargames.com` subdomain to its service.
2. Log `HCHttpCall*` URLs at runtime to enumerate the exact Xbox Live endpoints.
3. Recover/decrypt `telemetrycache.dat` for the real `Metrics[]` list (names + sampling).
4. Diff traffic with telemetry toggled to separate required (auth/session) from optional (analytics) calls.

---
*Method: string/URL/header template extraction + import classification + reuse of the telemetry/auth disassembly in the companion docs. No game code reproduced; only endpoints, header names, field identifiers, and templates.*
