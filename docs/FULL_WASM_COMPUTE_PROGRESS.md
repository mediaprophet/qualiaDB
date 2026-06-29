# Full-WASM computational-engine exports + demos — progress log

Workstream: expose **everything in the computational engine that can run in WebAssembly** through the
full-wasm (`wasm-full`) playground bundle, then build live browser demos for it. Requested by Timothy
2026-06-30. Honest record per project-rule §9.

The "full-wasm version" = the `docs/playground` bundle, built by CI
(`.github/workflows/{pages,release,benchmarks}.yml`) with
`wasm-pack build --target web --features wasm-full`. `wasm-full` pulls `wasm-scientific`, so the solver
substrate (`crate::solvers::*`) is compiled into that bundle — it just lacked `#[wasm_bindgen]` exports.

---

## Step 1 — Make the wasm-full build compile (foundation) — DONE (2026-06-30)

**Status:** done, committed `34e094df`.

**What was built / fixed.** `cargo build --target wasm32-unknown-unknown --features wasm-full` was broken
(15 errors) — it had to compile before any export could be added:
- 14× the solver GPU fast-paths (`solvers/linear_algebra/gemm.rs`, `learning/clustering/kmeans.rs`,
  `learning/classification/svm.rs`, `transforms/fourier.rs`) referenced `crate::wgsl_forge::dispatch`
  un-gated, but `wgsl_forge` is `#[cfg(not(wasm32))]`. Gated each GPU block on
  `all(not(wasm32), feature="wgsl-forge")` so wasm falls to the existing CPU floor (a WebGPU-on-wasm forge
  is a separate, larger effort). Native behaviour byte-identical.
- 1× `gguf_bridge/init.rs` had a contradictory double `cfg` on the wasm-only `prefill_scratch_buf` field
  → never initialized on wasm (E0063). Fixed.

**Measured.** wasm32 `--features wasm-full` compiles clean; native solver suite **604/604 green** (no
regression). This was also a real latent breakage of the playground/CI bundle, now fixed.

**⚑ Human:** none this step.

---

## Step 2 — Export layer: exemplar + parallel domain drafts — IN PROGRESS (2026-06-30)

**Status:** exemplar landed + wasm-verified; 7 sibling domains drafting in parallel.

**What was built.** New `crate::wasm_bridge::engine` module (gated on `feature="wasm-scientific"`, each fn on
`target_arch="wasm32"`), wired into `wasm_bridge/mod.rs`. It wraps the **wasm-clean solver layer**
(`crate::solvers::*`) — NOT the `*Library` structs (they call `Instant::now()`, which panics on wasm) and
NOT `specialized_libs::*` (almost all `#[cfg(not(wasm32))]`; only `symbolic_algebra` is wasm-available).
- `engine/linalg.rs` (exemplar, compiles to wasm): `la_matmul`, `la_transpose`, `la_determinant`,
  `la_solve`, `la_eigen_symmetric`, `la_eigenvalues`, `la_svd`, `la_polynomial_roots`.
- Drafting in parallel: `stats`, `cas` (base symbolic_algebra only — advanced CAS is native-only),
  `numerics` (special fns / number theory / interpolation / optimization), `exact` (BigInt/BigRational),
  `units`, `transforms` (CPU DFT/STFT), `graph` (shortest-path / spreading-activation / KGE / fuzzy).

**Honest scope (what is NOT wasm-exportable, by construction):** the 9 `specialized_libs` domain wrappers
(finance, medical, chemistry, engineering, ML, full stats lib, crypto, QPU) are `#[cfg(not(wasm32))]`
native-only; the advanced CAS (integration/limits/series/ODE/trig, multivar, polynomial_algebra,
constructibility) is native-only; key-generating crypto needs browser-RNG wiring. These stay native/MCP.
Several of these already have live WASM science-page demos via separate hand-written exports
(clinical_risk, organic_chemistry, black_scholes, sat, ode_decay).

**Measured.** exemplar `engine/linalg.rs`: wasm32 `--features wasm-full` compiles. Other domains pending
integration + build verification.

**⚑ Human:** none yet. (A later step may ask which capabilities you want surfaced most prominently in the
demo UI.)

**Next:** integrate the 7 drafts, verify the unified wasm build, rebuild the `docs/playground` bundle with
`wasm-pack`, then build the live demo page(s) and wire `menu.json`.
