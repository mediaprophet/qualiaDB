# Q1/Q2 incorporation wave 40 — Linear Algebra Host-widen — 2026-09-08

**Branch:** `cursor/poet-q1q2-wave22-bb54` (from `0.0.37`)  
**Status:** Complete (verified)  
**Prior:** wave 39 Complete — curated helper-aware Q2 leftover = 0  
**Authority:** owner Host-widen (Vibe is an app/REPL language; catalog grows)

## What this wave is

Grammar stays closed. `ALL_BOUND` / `capability.invoke` grows for app support.
Linear algebra is the first Host-widen family. Curated Q2 leftovers stay at 0;
this is **not** “bind all remaining CoreDb `pub fn`s.”

## Lanes

| Lane | Target | Deliverable |
|------|--------|-------------|
| A Docs | VibeScript core §0 / §11.5 · host-constraint 2026-09-08 | catalog-grows, REPL/apps |
| B Host gemm | `LinearAlgebra.gemm` → solver `gemm`; drop CPU size reject | `gemm_host.rs` |
| C Host app | `LinearAlgebra.{dot,norm,trace,identity,inverse}` | `math/la_app.rs` + catalogs |
| D Poet Live | `scientific:la_dot` … `la_inverse`; surface-aware `gemm_live` | `linalg_app_chain_actions.rs` |
| E Polish | machine schemas, dual-path scan, Poet-JSON Host tests, Econ stub honesty | coverage / tool_dual_path |

**Results (2026-09-08):** gemm_host **3** · la_app **6** · catalog **1** · poet `wave40` **1** · policy **1** · dual-path **1** · integrity **11**.

`LinearAlgebra.gemm` now calls the engine solver. CUDA `caps()` probe is fail-closed (`catch_unwind`) so missing `libcuda` does not abort Host/REPL.

```
cargo test -p qualia-core-db --lib gemm_host
cargo test -p qualia-core-db --lib la_app
cargo test -p qualia-core-db --lib vibe_catalog_contains_every_bound_invoke_id
cargo test -p poet --lib wave40
cargo test -p poet --lib every_registered_nonplacement_tool_has_an_explicit_policy
cargo test -p poet --test product_integrity
```
