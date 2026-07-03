# Companion PWA — author a qapp, install a wasm app on your phone

**Status:** foundation landed (2026-07-03); a first-class, multi-stage workstream from here.
**Owner intent (Timothy, 2026-07-03):** *"The companion PWA is an essential element. It is the means for
people to define a qapp, of various kinds, then get an installable wasm app on their phone. There are
various uses for it; it is a key feature that can then be built upon."*

This document treats the companion PWA as that platform capability — not the narrow LAN health-companion
pairing the desktop ships today — and lays a staged, honest path to it. It supersedes the single deferred
"T3.2 companion PWA" checklist line with a real roadmap.

---

## 1. What it is

A person authors a **qapp** (a least-privilege application over their own Qualia graph — a journal, a
health companion, a cooperative board, a directory, or something bespoke), and the system hands them an
**installable Progressive Web App** they can add to their phone's home screen and run — offline, as a real
installed app, driven by a **wasm** bundle. No app store, no build toolchain on the phone, no server they
don't control.

The pipeline is five moves:

1. **Author** — describe the qapp: identity, kind, the *capabilities it is allowed to touch* (least
   privilege), its icon and entry. This is a `QappManifest`.
2. **Build** — compile/assemble the qapp's wasm bundle and content-address it (hash + size).
3. **Package** — turn the manifest + wasm into a standards-compliant, installable PWA scaffold
   (`manifest.webmanifest` + service worker + loader). This is `generate_pwa`.
4. **Deliver** — serve the PWA to the phone over a **secure origin** (installability *requires* HTTPS or
   localhost; a plain LAN-HTTP origin is not installable on modern browsers).
5. **Install & run** — the phone "Add to Home Screen"s it; the service worker caches the shell + wasm for
   offline; the app runs against the person's graph (paired to the desktop node, or standalone).

## 2. Why it is load-bearing

- It is the **distribution surface** for everything else. Every domain we have built (WellFair health,
  cooperative work, credentials, the agency/guardianship layer) becomes a *shippable qapp* through this
  path instead of being locked inside one desktop binary.
- It is where **least-privilege isolation** becomes real and visible: a qapp declares the capabilities it
  needs; the token/permission layer (WP1) enforces exactly those.
- It is the on-ramp for **non-technical authors** (WP2 "Package & Publish"): define a qapp without
  hand-editing JSON, get an installable artifact.

## 3. What is built now (P0 — foundation, DONE)

`qualia-cooperative-core::qapp_package` (transport-neutral, pure Rust, 21 tests):

- **`manifest.rs`** — `QappManifest` (id, name, kind, version, description, **capabilities** [least
  privilege: ReadRecords / WriteRecords / Sync / BlobStore / Notifications / Camera / Custom], a
  content-addressed `WasmRef { path, sha256_hex, size_bytes }`, icons, theme/background colour, display,
  offline), an extensible `QappKind` (Cooperative / Health / Journal / Directory / Custom), a builder, and
  `validate()`.
- **`pwa.rs`** — `generate_pwa(&QappManifest) -> PwaBundle` emitting a correct, installable-in-principle
  scaffold: a W3C **`manifest.webmanifest`** (standalone display, `start_url`, icons), a cache-first
  **service worker** (`sw.js`, version-stamped cache, precache of shell + wasm + icons, offline fetch), and
  an **`index.html`** loader (viewport, manifest link, Apple touch tags for iOS, service-worker
  registration, `WebAssembly.instantiateStreaming`).

**Honest scope of P0:** this is the *authoring + packaging* core. It does **not** yet (a) serve the PWA
over a secure origin, or (b) compile the wasm bundle — those are the genuinely hard adjacent stages below,
not gaps in P0. The module doc says so plainly.

## 4. Roadmap from here

Each stage is independently useful and testable; later stages assume earlier ones.

- **P1 — Secure-origin delivery (the hard, load-bearing part).** Serve the generated PWA over a context the
  phone will treat as installable: HTTPS with a real cert, `localhost`, or a WebRTC data channel to a
  local origin. Options to evaluate: a loopback/HTTPS server with a device-trusted cert; a tunnelled TLS
  origin; or pairing over WebRTC. This is where "installable on your phone" actually becomes true. Reuses
  and generalises the existing companion gateway (currently plain LAN-WS — Grok's Phase-4 lane; coordinate,
  do not duplicate).
- **P2 — Pairing + install flow.** QR/short-code pairing from desktop to phone, the delivery of the PWA
  bundle, and the guided "Add to Home Screen" experience, with the qapp bound to the person's node.
- **P3 — Wasm build pipeline.** Produce the qapp's wasm bundle from the shared engine (the full-wasm bundle
  already exposes the compute engine to wasm) + the qapp's own UI, content-addressed into the `WasmRef`.
- **P4 — Per-app isolation + token v2 (cooperative plan WP1).** Enforce the manifest's declared
  capabilities: a qapp token scoped to exactly those capabilities, CSP, per-app storage isolation. This is
  a release gate for any qapp that touches restricted data.
- **P5 — Studio "Package & Publish" (cooperative plan WP2).** Author a qapp and produce the least-privilege
  manifest + packaged PWA from the Studio UI, without hand-editing JSON.
- **P6 — The standalone Cooperative Qapp (cooperative plan WP4)** as the first real dogfood qapp shipped
  through this pipeline.

## 5. Crossover with existing work (reuse, don't rebuild)

- **`qapp_package`** (this doc, P0) is the authoring/packaging home; it lives in `qualia-cooperative-core`
  alongside the record/projects/finance/work-item/agency domains, so every domain is shippable through it.
- **Companion gateway / live share** (Grok's Phase-4 lane) is the starting point for P1/P2 delivery and
  pairing — generalise it to a secure origin rather than standing up a parallel path.
- **Full-wasm engine bundle** feeds P3 (the compute engine already runs in wasm).
- **WP1 token v2 / WP2 Package & Publish / WP4 Cooperative Qapp** in
  [`cooperative-qapps-desktop-implementation-plan.md`](cooperative-qapps-desktop-implementation-plan.md)
  are P4/P5/P6 here — this doc sequences them behind the PWA delivery they depend on.

## 6. ⚑ Where the human decides

- **P1 secure-origin strategy is a real fork** (device-trusted HTTPS cert vs tunnelled TLS vs WebRTC),
  each with different trust/UX trade-offs. This is the first decision to make before P1 code — I will bring
  a concrete comparison when we reach it.
- **Which qapp kinds ship first** (journal? health companion? cooperative board?) — sequences P5/P6.
