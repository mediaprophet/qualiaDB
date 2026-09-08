# Q1/Q2 incorporation wave swarm — 2026-09-06

**Branch:** `0.0.36-dev`  
**Methodology:** `docs/work-in-progress/VIBE_INCORPORATION_METHODOLOGY_2026-09-06.md`  
**Constraint:** `docs/work-in-progress/VIBE_HOST_CONSTRAINT_CORRECTION_2026-09-06.md`  
**Pipeline:** crate → **Host `Family.method`** → **Poet Live**  
**Workspace:** `C:\github\qualiaDB` · no worktrees · not committed unless owner asks

## What Q1 / Q2 mean

| Queue | Meaning | Work |
|-------|---------|------|
| **Q2** | Id already in `ALL_BOUND` but not cited in Poet Live | Poet Tool Chest dual-path only (fast) |
| **Q1** | `pub fn` in crates with no exact Host `*.fn_name` twin | **Host bind first** (paired catalogs + handler), then Poet Live |

This wave: **three Q2 Poet packets + one Q1 Host starter**. Re-run backlog after integrate.

## Lanes (disjoint)

| Lane | Packet | Owns (write) | Must not touch |
|------|--------|--------------|----------------|
| A | `Q2-ECON` | `econ_chain_actions.rs`, `register_econ_toolbox.rs`, econ-related `tool_actions`/`tool_copy`/`registration/mod` asserts | sheet, scientific, Host ids |
| B | `Q2-STATS` | `register_sheet_toolbox.rs`, sheet dual-path (new `stats_chain_actions.rs` if needed), live_args/tool_actions as required for Statistics.* | econ, scientific Host |
| C | `Q2-ML` | `register_ai_toolbox.rs` and/or scientific ML chain — cite existing `MachineLearning.*` Host ids (≥8 Live tools) | econ, sheet, ids.rs |
| D | `Q1-HOST` | New Host binds for a **curated** specialized_libs slice that is Host-missing — prefer `LinearAlgebra` / `SpecialFunctions` / unbound chemistry helpers **only if** methods are not already in ALL_BOUND. Pair `ids.rs` + `vibe/catalog/ids.rs` + invoke module + ≥3 handlers + tests. Optional one Poet Live cite. | poet econ/sheet/ai registration files Lane A–C own |

## Acceptance per lane

- Dual-path honesty (local sketch / live / denied / fault)
- `capability_scope` = exact Host id string
- Focused tests green; `product_integrity` if Poet registration changed
- Lane D: `vibe_catalog_contains_every_bound_invoke_id` green

## Scale honesty

Q2 has hundreds of uncited ids; Q1 has ~10k+ unbound pub fns. **Curated slices only** this wave — do not attempt to clear the entire backlog in one swarm.

## Parent integrate

Verify tests, refresh `vibe_incorporation_backlog.py`, update register + ledger.

## Lane reports (parent)

| Lane | Status | Agent | Delivered |
|------|--------|-------|-----------|
| A `Q2-ECON` | **done** | [Q2 Econ more Poet Live](bd29ffc0-a7f5-4b10-a12b-8defd09069d0) | +8 Live `Econ.*` |
| B `Q2-STATS` | **done** | [Q2 Statistics Poet Live](8db951db-559b-41d3-aaec-a89200a75c20) | +11 Live `Statistics.*` via `stats_chain_actions.rs` |
| C `Q2-ML` | **done** | [Q2 MachineLearning Poet Live](e5d1a7dd-6565-4c64-8717-5590fc255693) | +11 Live `MachineLearning.*` under `ai:ml` |
| D `Q1-HOST` | **done** | [Q1 Host bind starter slice](1df82c13-d31a-40c9-9e78-f73a01cf9943) | +7 `LinearAlgebra.*` (QR/Cholesky solve/BLAS-1); `math/qr_vector.rs` |

**Wave status:** integrated (parent-verified). Not committed.

**Parent verify:**
- poet `--lib econ` **25**; `--lib stats` **12**; `sheet_grid_binds_extended_statistics_caps` **1**; `ai_ml_chain` **1**; `product_integrity` **11**
- core-db `vibe_catalog_contains_every_bound_invoke_id` **1**; `qr_vector` **6**

**Backlog refresh:** `ALL_BOUND=904` · `pub_fns=19376` · `Q1=12308` · `Q2=818` · `PoetLive=86` (was ~56 Live / ~897 bound before this wave).
