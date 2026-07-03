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
