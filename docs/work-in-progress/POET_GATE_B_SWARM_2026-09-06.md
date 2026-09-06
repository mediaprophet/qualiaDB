# Poet Gate B + incorporation swarm — 2026-09-06

**Branch:** `0.0.36-dev`  
**Freeze:** `vibe-host-0.1` · no Host widen · no invented ALL_BOUND IDs  
**Workspace:** `C:\github\qualiaDB` (no worktrees)  
**Not committed** unless owner asks.

## Goal

1. Start Review Gate B portable-app / Desktop control-plane packets.
2. Inventory **sibling-crate infrastructure** (`qualia-client-core`, `qualia-cooperative-core`,
   `qualia-vision`, and peers) that is built but weakly incorporated into Vibe and/or Poet —
   classify seam (Vibe / MCP / Desktop / Poet Live / cold) without inventing Host IDs.

## Lanes (disjoint writes)

| Lane | Packet | Owns (write) | Must not touch |
|------|--------|--------------|----------------|
| A | `EXP-C1` crate→seam inventory | `scripts/vibe_surface_gap_review.py` PRIORITY_GLOBS only as needed; `docs/work-in-progress/CRATE_SURFACE_INCORPORATION_2026-09-06.md`; may regenerate/extend gap review | `q42/app_*`, poet product UI, webizen-desktop shell |
| B | `APP-03` projection adapters | `crates/qualia-core-db/src/q42/app_manifest/` only (`project*.rs` / projections + tests in mod) | poet/, webizen-desktop/, app_registry, gap scripts |
| C | `WD-01` control-plane IA | `docs/work-in-progress/WD_01_CONTROL_PLANE_IA_2026-09-06.md` (+ tiny ADR link in DIRECTORY_INDEX if required) | product Rust; do not rewrite desktop shell yet |
| D | `WD-02` installed-app registry v0 | New `crates/qualia-core-db/src/q42/app_registry/` + one-line `q42/mod.rs` `pub mod` | app_manifest internals beyond reading public API; poet/; desktop UI |

## Honesty

- No network downloaders; no Host widen; no invented `Family.method` IDs.
- APP-03: same authorization result across projections; presentation hints never grant authority; no projection-specific private DB.
- WD-01: docs/IA only this lane — POET under Apps; old routes remain reachable in the plan; no fake daemon status.
- WD-02: read-only inspect; malformed packages quarantined; registry does not execute apps; POET first bundled fixture optional.
- EXP-C1: classification table is the deliverable — do **not** bind Host IDs in this lane.

## Parent integrate — **COMPLETE** 2026-09-06

| Lane | Packet | Status | Evidence |
|------|--------|--------|----------|
| A | `EXP-C1` | **Done** ([crate inventory](ab8cc5d5-4375-4b5e-89f2-cdf8d2aabc04)) | `CRATE_SURFACE_INCORPORATION_2026-09-06.md`; gap review ALL_BOUND=892 modules=1481 |
| B | `APP-03` | **Done** ([projections](090ae21a-5c80-4438-a9e0-17ecfc02babe)) | `app_manifest/project.rs`; parent app_manifest **21** |
| C | `WD-01` | **Done** ([IA docs](6ce9046c-5072-4b38-a614-ce25e0a44e2c)) | `WD_01_CONTROL_PLANE_IA_2026-09-06.md`; ADR index under 0013 |
| D | `WD-02` | **Done** ([app registry](717bb637-754b-41c1-a685-d9e2f24609a0)) | `q42/app_registry/`; parent app_registry **11**; app_manifest **21** |

Register + session ledger updated. Not committed (await owner).

### Next (owner choice)
- Poet consume remaining `ComputerVision.*` / more `Econ.*` Live (from EXP-C1)
- `APP-04` Health proof package · `WD-03` lifecycle · shell IA apply from WD-01
- Review Gate B when APP/WD bar met
