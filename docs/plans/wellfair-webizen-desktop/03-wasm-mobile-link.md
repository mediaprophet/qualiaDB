# Workstream 3 — WASM Packaging, Companion Gateway, and Mobile Link

**Goal:** let Webizen Desktop create and serve a signed installable mobile companion while
preserving a contract that a future native mobile app can implement.

## Current state

Useful seams exist, but the flow is not operational:

- `qualia-mobile-harness` is a Dioxus scaffold with simulated QR and authentication;
- `webizen_server` mobile stream/pane endpoints are mocks;
- the Android packaging script does not build/copy a full distribution;
- qApp WASM export depends on a stale/missing package and emits no PWA contract;
- the exported LAN server is unauthenticated HTTP;
- qApp install and launch registries are inconsistent;
- OPFS block access exists, but browser WAL/outbox persistence does not.

This workstream replaces those scaffolds rather than layering product claims over them.

## Target deliverables

1. deterministic signed qApp/PWA package;
2. desktop UI action to export, inspect, serve, and revoke it;
3. secure install origin;
4. real mutual device pairing;
5. transport-neutral host protocol;
6. encrypted mobile cache and durable outbox;
7. sync receipts and device revocation;
8. future-native conformance suite.

## WASM profile

Create a named WellFair linked-companion profile with a strict capability/size budget.

Include:

- qApp UI runtime;
- package/hash/signature verification;
- device identity and pairing;
- CBOR-LD/wire DTO decoding;
- bounded receipt and projection verification;
- encrypted local cache/outbox;
- bounded local policy/shape checks needed while offline.

Exclude initially:

- native daemon/network stack;
- full desktop graph and mmap storage;
- desktop LLM/model lifecycle;
- unrestricted scientific/portal profile;
- remote inference;
- Sanctuary records;
- arbitrary filesystem access.

Use canonical `qualia-core-db` WASM features. Do not make `webizen-web` a parallel semantic
authority, and do not use `webizen-lite-wasm` as an app shell.

## Task packages

### M1. Repair qApp installation authority

**Ownership**

- `crates/qualia-client-core/src/qapp_registry.rs`
- `qapp_manifest.rs`
- `qapp_paths.rs`
- `qapps_protocol.rs`

**Tasks**

- define one package directory and registry;
- validate manifest, signature, hashes, paths, ABI, schemas, and capabilities;
- copy/install atomically into `{storage}/Qapps/<package-id>/<version>`;
- reconcile registry on startup;
- add update, rollback, uninstall, and revocation;
- issue scoped sessions only for verified installed versions.

**Acceptance**

- install/list/launch/token use the same package identity and files;
- path traversal and symlink escapes fail;
- interrupted update leaves the prior version usable;
- revoked package cannot launch.

### M2. Deterministic bundle builder

**Ownership**

- new package builder in `qualia-client-core`;
- thin Tauri commands in `webizen-desktop`;
- build scripts/CI.

**Bundle**

```text
wellfair-companion/
  qapp.json
  package-manifest.cbor
  package-manifest.json
  index.html
  assets/<content-hashed files>
  wasm/<profile>.js
  wasm/<profile>_bg.wasm
  data/<optional signed seed.q42>
  manifest.webmanifest
  sw.js
  icons/
  LICENSES/
  SBOM/
```

**Tasks**

- consume a prebuilt signed WASM artifact;
- produce deterministic ordering, timestamps, names, and hashes;
- sign the content manifest;
- emit archive and served directory;
- generate CSP/Permissions-Policy/COOP/COEP;
- generate content-hashed cache version and migration rules;
- run offline and integrity smoke tests before declaring export successful.

**Acceptance**

- same inputs produce the same content hash;
- missing/mismatched asset fails closed;
- service worker update cannot mix versions;
- no developer machine path or secret appears in the bundle.

### M3. Secure origin

Record an ADR choosing:

- trusted HTTPS bootstrap origin with authenticated WSS/WebRTC; or
- local HTTPS with deliberate certificate onboarding.

**Acceptance**

- supported mobile browsers report `isSecureContext`;
- service worker, camera, crypto, and storage features work;
- install is not dependent on unauthenticated LAN HTTP;
- offline launch works after installation.

### M4. CompanionGateway

**Ownership**

- separate module/service, not the loopback daemon router.

**Tasks**

- disabled-by-default LAN binding;
- one-time pairing sessions;
- endpoint/certificate fingerprint advertisement;
- mutual challenge-response;
- explicit desktop approval and short verification code;
- method/scope/purpose/audience/expiry-bound tokens;
- rate and payload limits;
- device list, revoke, and rotate;
- privacy-safe connection diagnostics.

**Acceptance**

- unpaired clients cannot probe graph/query endpoints;
- stolen/expired QR data cannot pair;
- a revoked device loses new access immediately;
- loopback control API remains unreachable from LAN.

### M5. Versioned companion protocol

**Ownership**

- shared DTO crate/module owned by this workstream after Host API v1 freezes.

**Tasks**

- define message envelope, limits, version negotiation, errors, and feature flags;
- implement projection, command, consent, event, sync, receipt, and revoke messages;
- add operation IDs, sequence/ack, replay window, resume cursor, and parent hashes;
- convert wire DTOs to canonical records only after validation;
- add compatibility fixtures for current and previous supported versions.

**Acceptance**

- malformed, oversized, unsigned, replayed, wrong-audience, and unsupported-version frames fail;
- disconnect/reconnect resumes without duplicating operations;
- logs never contain tokens or sensitive payloads.

### M6. Linked PWA

**Ownership**

- `crates/qualia-mobile-harness`
- its assets and packaging scripts

**Tasks**

- replace simulated scanner with real camera plus manual code/paste fallback;
- persist device identity securely within browser constraints;
- install and update PWA;
- pair and render desktop-approved projections;
- queue bounded offline commands;
- encrypt OPFS/IndexedDB cache/outbox;
- show connection, authority, cache, expiry, and sync state;
- add device unlink and local wipe;
- never cache Sanctuary data in the linked profile.

**Acceptance**

- install, offline reopen, pair, query projection, consent prompt, receipt, unlink;
- quota, permission denial, browser suspension, and upgrade are handled;
- local wipe does not imply remote deletion and vice versa.

### M7. OPFS journal and outbox

**Ownership**

- WASM-safe storage modules and worker integration.

**Tasks**

- bridge the existing OPFS worker/block primitives;
- add checksummed journal entries;
- commit/flush/ack semantics;
- recover partial writes;
- enforce quota and compaction;
- encrypt at rest;
- retain only approved projections and pending commands.

**Acceptance**

- crash/refresh during write recovers;
- duplicate resend is idempotent;
- a corrupt record is quarantined;
- cache expiry and policy revocation remove future accessibility.

### M8. Future native-mobile compatibility

Deliver:

- protocol conformance vectors;
- package-manifest vectors;
- pairing challenge vectors;
- record/projection/sync operation fixtures;
- authority-promotion design for a future peer;
- capability downgrade rules for devices without a feature.

The native app must not need to emulate Dioxus, Tauri, browser ports, or JavaScript-specific
storage to speak to the desktop.

## Suggested agent split

| Agent | Ownership | Dependency |
|---|---|---|
| M-A | package registry/install/update | qApp manifest ADR |
| M-B | deterministic bundle/CI | WASM profile and registry |
| M-C | secure origin + gateway | security ADRs |
| M-D | protocol/pairing | Host API v1 |
| M-E | PWA UI/install | package + protocol |
| M-F | OPFS journal/cache | record/projection format |
| M-G | conformance/adversarial tests | all contracts |

## Release gates

### Linked PWA alpha

- secure install;
- pairing and revocation;
- read-only approved projections;
- receipts;
- no offline writes.

### Linked PWA beta

- encrypted cache;
- bounded offline commands/outbox;
- resume and conflict state;
- permission/platform matrix.

### Linked PWA release

- reproducible signed packages;
- update/rollback;
- adversarial gateway/protocol test;
- Android/iOS supported-browser install evidence;
- privacy review and no Sanctuary path.

## Completion criteria

Webizen Desktop can produce a verifiable installable companion package that works from a secure
origin, pairs through mutual authentication, exposes only policy-approved projections, supports
revocation and offline recovery, and passes the same protocol fixtures reserved for the future
native mobile peer.
