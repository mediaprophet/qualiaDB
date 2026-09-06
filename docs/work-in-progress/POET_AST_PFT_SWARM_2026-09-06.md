# Poet programme swarm — 2026-09-06 (AST + PFT)

**Branch:** `0.0.36-dev`  
**Freeze:** `vibe-host-0.1` · no Host widen · no invented ALL_BOUND IDs  
**Workspace:** `C:\github\qualiaDB` (do not use worktrees)

## Lanes (disjoint writes)

| Lane | Packet | Owns (write) | Must not touch |
|------|--------|--------------|----------------|
| A | `AST-02` | `crates/qualia-core-db/src/q42/asset_import/` + `q42/mod.rs` one-line `pub mod` | poet/, asset_envelope/, source_catalogue/ |
| B | `AST-07` | `crates/qualia-core-db/src/q42/source_catalogue/` + `q42/mod.rs` one-line `pub mod` | poet/, asset_import/, asset_envelope/ |
| C | `PFT-05` | poet ribbon dual-path for existing ALL_BOUND only: `ParaconsistentLogic.route`, `TemporalAndDescriptionLogic.ltl.evaluate`, `SymbolicAlgebra.eval` | qualia-core-db q42/, Host ids |

## Honesty / safety

- No network downloaders.
- Unknown licence → fail closed / `unverified`.
- No dataset bundling or redistribution claims without verified terms.
- Parent integrates + runs focused tests after lanes report.

## Parent integrate (2026-09-06)

- Removed leftover `crates/poet/src/browser/_pft05_patch.ps1` scratch from Lane C.
- `q42/mod.rs` has `asset_envelope` + `asset_import` + `source_catalogue`.
- Verify: `cargo test -p qualia-core-db --lib -- source_catalogue asset_import` → **19 passed**.
- Poet: `cargo test -p poet --lib -- tool_actions live_args registration` → **10 passed**.

## Status

- 2026-09-06: Swarm launched (A/B/C).
- **Lane A (`AST-02`)**: Done — `q42/asset_import/` (`ImportJob` + budgets/status/error). Unique `TempDir` RAII, Sentinel chunk pass budget, streaming `feed_chunk`, cancel, promote-on-success, immutable raw stage (hardlink/copy). Verified: `cargo test -p qualia-core-db --lib asset_import` → **11 passed**, 0 failed.
- 2026-09-06 **Lane B (AST-07):** `q42/source_catalogue/` added — static descriptors for FooDB, HMDB, CTD, Monarch, ABCkb, FoodAtlas, Phenol-Explorer, FOODBALL, PhInd, Cytoscape (+ ChEBI catalogue / CC BY 4.0 importer candidate). Statuses fail closed (`unverified`/`restricted`/`connector`/`catalogue`); no downloads, no redistributable claims, `assert_not_bundled` always true at this layer. `pub mod source_catalogue;` in `q42/mod.rs`. Tests: **8 passed** (`cargo test -p qualia-core-db --lib source_catalogue`).

### Lane C — PFT-05 (2026-09-06) — done

Ribbon dual-path tools bound to existing ALL_BOUND ids (no Host widen):

| Tool id | capability_scope |
|---------|------------------|
| `epistemic:paraconsistent_route` | `ParaconsistentLogic.route` |
| `code:ltl_evaluate` | `TemporalAndDescriptionLogic.ltl.evaluate` |
| `code:symbolic_eval` | `SymbolicAlgebra.eval` |

**Files:** `logic_chain_actions.rs` (new), `mod.rs`, `tool_actions.rs`, `tool_copy.rs`, `registration/register_epistemic_toolbox.rs`, `registration/register_code_toolbox.rs`. Args mirror `live_args` (epistemic / LTL Globally+property / `data-formula` expr). Visible copy is plain language only.

**Verify (Lane C):** `cargo test -p poet --lib -- tool_actions live_args chain_actions` → 13 passed; `cargo test -p poet --lib -- registration::` → 3 passed. Used private `CARGO_TARGET_DIR=target-poet-pft05` due to swarm cargo lock contention; directory removed after verify.
