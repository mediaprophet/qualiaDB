# WellFair §8.1 Exit Sprint — Sub-agent orchestration (2026-07-02)

**Canonical repo:** `C:\Projects\qualia-27062026` | **Branch:** `0.0.24`  
**Epic:** Phase 2 exit — first usable journey (README §8.1)

## Goal

Close the Phase 2 exit criterion: *the first usable journey in section 8.1 passes offline and after restart; source, normalization, decision, and export provenance is inspectable.*

## Lanes (parallel, minimal file overlap)

| Lane | Agent | Scope | Owns | Gate |
|------|-------|-------|------|------|
| **A** | E2E-journey | API-level §8.1 journey automation | `journey_tests.rs`, `mod.rs` | Full host path: import → med → condition → consent → checkpoint → reopen |
| **B** | Export-package | Signed standards-readable Turtle export | `export_package.rs`, `api.rs` (export only), Tauri, `tools_panel.rs` | Export round-trip + content hash + checkpoint binding |
| **C** | Graph-query | Bounded quin coverage query over journal | `graph_query.rs`, `vault.rs` (read helpers), `api.rs` (query only) | Journal entry → quin count matches compile_to_quins |

## Orchestrator merge order

1. Lane C + B (vault/api additions)
2. Lane A (uses export + query APIs)
3. Single `cargo test -p qualia-client-core wellfair --lib`

## Deferred (Phase 2 remainder)

- OS medication reminders (Q6)
- Disputed diagnosis + housing/safety (Q2)
- Full SPARQL daemon query path
- UI E2E (Playwright/Tauri driver)

## Verification

```powershell
cd C:\Projects\qualia-27062026
cargo test -p qualia-client-core wellfair --lib
cargo check -p webizen-studio -p webizen-desktop
```