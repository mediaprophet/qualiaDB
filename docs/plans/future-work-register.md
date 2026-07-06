# Future-work register — parked & prioritized initiatives

**Purpose:** a single index so the large future initiatives are **not forgotten**. Each entry points to
its full plan; this file only tracks *what exists, its priority, and the concrete on-ramp*. Linked from
[`MASTER-EXECUTION-CHECKLIST.md`](MASTER-EXECUTION-CHECKLIST.md) so it stays visible.

These are multi-phase (P0–P12) plans — real engineering, not quick tasks. This register keeps them alive
without pretending they are near-done.

---

## ★ Prioritized (Timothy, 2026-07-03): 3D anatomy

**Why:** Timothy flagged 3D anatomy as important; "the rest can wait."

3D anatomy is a **slice of** [`native-visual-intelligence-and-generative-3d.md`](native-visual-intelligence-and-generative-3d.md)
(Phase 9 + Phase 11/12 biological profile) plus the manual
[`../manuals/computational-3d-assets-and-digital-twins.md`](../manuals/computational-3d-assets-and-digital-twins.md).
The near-term, tractable **on-ramp** (does not need the image-generation/reconstruction phases):

1. **Fix `mesh_to_nquins` parity** (`qualia-core-db/src/render/assets.rs`) — currently emits `parity: 0`
   (invalid). *(Done as a quick win, see below — this is the first real step and it's on the anatomy path.)*
2. **Canonical GLB→Q42 asset compiler (visual Phase 9).** Extend `render/assets.rs` GLB ingest to honour
   real glTF **accessor layout** (normals, UVs, indexed primitives, node hierarchy/transforms,
   materials/PBR) instead of assuming the BIN chunk starts with packed positions; preserve the source GLB
   content-addressed + emit a page-aligned compiled-geometry sidecar; valid-parity Q42 facts.
3. **Retire the desktop Anatomy parser.** `webizen-desktop::commands::glb_ingest`
   (`GLBIngestionManager`/`Tensor10DMapping`) reads into a `Vec`, assumes BIN-offset packed positions, and
   overloads `Tensor10D` axes with ontology IDs — make it a thin client of the canonical core importer, and
   migrate the VH-Male / anatomy assets through it (semantic ids attached to component ids, not Tensor10D
   axes).
4. **Wire `QualiaSuperBlock::fea_mesh_index_id`** (or supersede it with a Q42 predicate) once an analysis
   mesh exists — the reserved analysis-mesh link is currently written as zero.
5. Later: bind an anatomy component to units/materials and the **F2 analytical** kernels that already exist
   (visual Phase 11), under the F/A fidelity-vs-assurance policy.

**First reviewable milestone:** import an anatomy GLB through the canonical core compiler with correct
accessor handling + valid-parity Q42 facts + a compiled-geometry sidecar the renderer uses without
reparsing GLB. No new model runtime required.

---

## Parked (can wait — tracked so they aren't lost)

### Native visual intelligence & generative 3D (the rest)
[`native-visual-intelligence-and-generative-3d.md`](native-visual-intelligence-and-generative-3d.md) —
`qualia-vision` crate; P0 audit → fixed-buffer ABI → media sidecars/provenance → Forge vision ops
(Conv2D/Pool/Resize) → encoder+classifier → detection+video → SPARQL-MM repair → synthetic datasets →
image generation → image-to-3D → computational-3D substrate → verified solvers + F/A assurance. Runs on the
existing shared wgpu + WGSL Forge + P64; **no Candle/Burn/Python** in the production ABI.

### Native auditory, language & music intelligence
[`native-auditory-language-and-music-intelligence.md`](native-auditory-language-and-music-intelligence.md) —
`qualia-audio` crate; consolidate U3/Sonic-Token/Q4AU contracts → capture/codecs → streaming STFT/CQT/mel
features → provenance/semantic compiler → **acoustic event detection + VAD (first learned "ears")** →
community-governed **language-resource workbench** (oral-first, no forced text) → speech + alignment →
music analysis → production engine → TTS (consent-gated) → separation/generation → eyes+ears composition.
Strong human-rights/consent/cultural-protocol requirements throughout.

### Chora (crate `qualia-ste`) — spatio-temporal permissive-commons omniverse (the "10d browser" exploration world)
[`spatio-temporal-commons-canvas/README.md`](spatio-temporal-commons-canvas/README.md) — a Google-Earth-like
explorable 3D world for Webizen Desktop: temporal scrubbing, permissive-commons asset "planting", layered
open datasets (OGC/OSM/DEM, council, biosphere/GBIF, historical maps/HGIS, orbital/JPL-Horizons), decoupled
per-location **governed** scripting (puzzles/life-games), hypermedia containers packaging native-derivative +
original-source + provenance, nquin stewardship. Ontology spine (Timothy): **world of man** (OWL / digital
twin, proper) vs **world of god** (natural — computational approximation, *never* a twin); thesis: *context
is the asset* (anti-commodification → the high-signal grounding that fixes hallucination). Grounded gap
analysis (6 read-only code explorations): the hard engine primitives are **REAL** (render/Tensor10D/σ colour+
sound, `.10d` container + QEM LOD, WAL+Merkle-DAG `nodes_as_of(t)` time-travel, RCC-8/Allen VM opcodes,
VC/deontic access, webseed + chat/sync relays, `ccf_resolver` SPARQL discovery); the build is the
**geospatial backbone** (geodetic + DEM + globe), spatio-temporal range query + H3/quadtree (stubs),
time-scrub replay, the `.10d` **provenance-sidecar (type 7)** + licence predicates + validate-before-render,
qapp-facing scripting + steward/delegation, host render-surface/temporal/streaming APIs, and data-layer
adapters + a per-adapter disclosure registry. P0–P8 phased; single-participant offline-capable canvas comes
early, external data + shared worlds later. Dependency-gated relative to the wellfair MVP. Docs-only, no code.

### T3.4 — Phase-7 optional (WellFair roadmap tail)
From [`remaining-work-consolidated-plan.md`](remaining-work-consolidated-plan.md): 3D anatomy *(now
prioritized above)*, studies/rules engine, authenticated **Solid Pod** sync, model-assisted extraction,
wallet / private transport, distributed (federated) analytics, native mobile peer. Individually optional;
each is its own effort.

### Cooperative Qapp initiative (WP0–WP11)
[`cooperative-qapps-desktop-implementation-plan.md`](cooperative-qapps-desktop-implementation-plan.md) —
tracked in checklist **§F**. The canonical Cooperative Qapp (§11/WP4), the QualiaDB **Development
Cooperative** (§17/WP9), installed-Qapp **token v2 + loopback CSP/origin isolation** (§7/WP1, the
restricted-data release gate), finance/agreements/sync/forge/release WPs. Companion-PWA delivery (T3.2) is
the phone side; **P1 secure-origin decided: WebRTC to a local origin, same-network** (see
[`companion-pwa-installable-qapps.md`](companion-pwa-installable-qapps.md)).

### Solid Chat interop — LDP transport + live wiring (mapping DONE)
[`solid-chat-interop.md`](solid-chat-interop.md) — the native ↔ SolidOS "long chat"
(<https://solid.github.io/chat/>) mapping is **built + verified lossless** (`qualia-client-core/src/solid_chat.rs`,
4 tests; **additive-fidelity** mechanism — the standard `meeting:`/`sioc:`/`foaf:` subset every Solid client
reads, PLUS native-only `qc:` triples on the same resource, so Qualia→Solid→Qualia round-trips losslessly and
the native format is untouched). **Parked (Timothy, 2026-07-06 — discovery comes first):** the LDP transport
(PUT `index.ttl` + HTTP **PATCH** the day file + GET/parse), wiring to live `chat_session`/`chat_graph` + a
"Publish / Import Solid chat" desktop action, the `sioc:has_reply` threading projection from chat-graph reply
edges, and swapping the focused round-trip parser for the engine `N3Parser` on inbound.

### Comprehensive multi-chain wallet + semantic tokens (Timothy, 2026-07-06)
A **BIP-39** (24-word / 256-bit) **HD** wallet (BIP-32/44) in **multisig** (m-of-n) — the concrete
instantiation of the deferred [`selfhood-cryptography-fabric.md`](selfhood-cryptography-fabric.md)
(threshold / social-recovery / dead-man-switch; the multisig *is* that primitive). Enumerates addresses for
**Bitcoin + Lightning** (LN on BTC), **eCash (XEC)**, and **Nym (NYM)**; **full eCash token support — both
standards: SLP** (Simple Ledger Protocol) **+ ALP** (A Ledger Protocol); and **semantic tokens** — a native
Rust re-expression of Timothy's [semantic-tokens-xec](https://github.com/mediaprophet/semantic-tokens-xec)
(+ `xec-slp-rdf`) model: RDF/JSON-LD token metadata + IPFS on XEC SLP/ALP, **grounded in the Qualia
ontology/quins** (his prior art — do not reattribute). Ties into the permissive-commons economy (Compute
Bounties, ILP metering, Lightning micropayments — `webizen-protocol-rfc.md` §4). **Existing scaffolding:**
`api.rs` `WalletStatus`/`CoinBalance`/`TokenEntry`/`fetch_wallet_portfolio`/`mint_semantic_token` +
Lightning/ILP (`ilp_dispatcher.rs`). **REAL crypto only** (bip39/bip32/secp256k1; no simulation — project
rule). Its own major workstream; not built as a unit.

### Networking setup options (Cloudflare / Nym) — part of the discovery build
Fold into the discovery plan + [`personal-platform-provider-and-networking.md`](personal-platform-provider-and-networking.md)
§4: a tiered rendezvous menu — **email/manual** (zero-infra MVP), **Cloudflare** (easiest for domain owners:
**Cloudflare Tunnel** = outbound-only reachability, no inbound port / static IP; + DNS front-door record +
Workers mailbox + R2 blind state store — all BYO under the user's own account), **own-edge/generic**, **Nym**
(privacy-max, opt-in/paid, sensitive cases), + **mDNS** local. Make install as easy as possible.

---

## Quick wins spotted while auditing the visual/auditory plans (verified in code, not parroted)

| Win | Evidence (verified) | Status | Lane |
|---|---|---|---|
| **`mesh_to_nquins` parity fix** | `render/assets.rs` hardcoded `parity: 0`; codebase pattern is `parity = subject ^ predicate ^ object ^ context ^ metadata` (`NQuin::calculate_parity`, `resource_catalog.rs`, `deontic_logic.rs`). Emitted geometry NQuins violated the "every NQuin has valid parity" invariant. | **Done** — on the 3D-anatomy path. | qualia-core-db |
| **`audio_contract.rs` false "zero-heap compliance" claim** | Module doc claims zero-heap/zero-copy while the type is `sigma: Vec<f32>` with `vec![0.0f32; 64]` (heap). A §12 measurement-honesty defect. | **Done** — corrected the claim to be honest. | webizen-render |
| **C2PA/SPARQL-MM verification-status honesty** | `sparql_mm.rs` "signature verification is explicitly simplified"; status can read as verified when it is only parsed. Make the status honest (parsed / integrity-checked / signature-verified / unsupported), like the `sparql_did` fail-closed precedent. | *Candidate* — small-ish, needs a bit of care on the status enum. Not yet done. | qualia-core-db |

Genuinely **not** quick (flagged so they aren't mistaken for wins): full SPARQL-MM repair (§6.3, 8 items),
GLB normals/UV/material support (Phase 9), the `qualia-vision`/`qualia-audio` crates and their model
runtimes.
