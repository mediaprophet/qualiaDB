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

## Step 2 — Export layer: exemplar + parallel domain drafts — DONE (2026-06-30)

**Status:** done, committed `7f320fe3`. 76 exports across 8 modules; unified wasm-full build green.

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

---

## Step 3 — Rebuild bundle + live demo page + end-to-end verification — DONE (2026-06-30)

**Status:** done. Demo page + rebuilt bundle + menu entry.

**What was built.**
- **Rebuilt the `docs/playground` bundle** with the CI's exact command/flags
  (`wasm-pack build --target web --release --features wasm-full`, 8 MB stack / 4 GB max-memory): 2.81 MB →
  **3.26 MB**; all 76 new exports verified present in `qualia_core_db.js`.
- **`docs/compute-engine.html`** — a data-driven page: all **76 functions** as live, editable "try-it" cards
  grouped into 8 domain tabs (Linear Algebra 8 · Statistics 13 · Computer Algebra 6 · Numerics 25 · Exact 8
  · Units 5 · Transforms 7 · Graph 4). Each card calls the in-browser export and shows the JSON result +
  timing. Honest hero + a "what runs here / what is **not** here (native-only)" card.
- **`docs/menu.json`** — added "Computational Engine" under Sciences (highlighted).

**Measured (real, in-browser, via a served local copy + headless eval):**
- WASM loads; **all 76 functions execute with 0 errors** across the 8 tabs.
- Correctness spot-checks: `matmul`=[58,64,139,154]; `det`=−2; `eigvals`[[2,1],[1,2]]=[1,3];
  `poly_roots(x²−5x+6)`=[2,3]; `factorial(100)`= the exact 158-digit value; `2^100`=
  1267650600228229401496703205376; `100 °C→°F`=212; `c`=299 792 458 m/s (dim [L·T⁻¹]); `factorize(360)`=
  2³·3²·5; `d/dx(x³−2x²+5)`=3x²−4x; `describe`→ mean 5 / var 4; `DFT[1,1,1,1]`=[4,0,0,0]; shortest path
  0→2 = 3 via [0,1,2]. All correct. Per-call time sub-millisecond (first matmul ~12 ms incl. warm-up).

**⚑ Human:** none required. Optional direction if you want it: (a) add the few wasm-clean capabilities the
drafters flagged but left out (nonparametric tests, KGE link-prediction, more `units` quantity ops); (b) a
hashing-only crypto module (SHA-256/512 / BLAKE3 / HKDF need no RNG and would build) — say the word.

**Done.** The full computational engine now runs in the browser, demonstrated and verified. CI rebuilds the
same bundle on push; the committed bundle makes the demo work immediately on Pages.

---

## Step 4 — Follow-ups Timothy approved: crypto-hash + stats/graph extras — DONE (2026-06-30)

**Status:** done. 76 → **85 exports**.

**What was built.** Timothy: *"yup do it"* (the offered follow-ups).
- **`engine/crypto.rs`** (new domain, 5 fns): `crypto_sha256`, `crypto_sha512`, `crypto_sha3_256`,
  `crypto_blake3`, `crypto_hkdf_sha256` — the deterministic, **RNG-free** RustCrypto primitives
  (`sha2`/`sha3`/`blake3`/`hkdf`, un-gated → wasm-clean) the native crypto lib uses. Input as `{text}` or
  `{hex}`. **Key generation / signing (Ed25519, ML-DSA) is still excluded — it needs a browser RNG.**
- **stats extras (3):** `stats_mcnemar_wasm`, `stats_friedman_wasm` (nonparametric), `stats_fisher_f_wasm`
  (F-distribution pdf/cdf/quantile) — the wasm-clean fns the drafter had flagged.
- **graph extra (1):** `graph_kge_predict` — KGE link prediction (rank candidate tails for a fixed
  head/relation).
- Demo page: new **Cryptography** tab + the 3 stats / 1 graph cards (85 cards, 9 tabs). Bundle rebuilt
  (3.26 → 3.32 MB).

**Measured (real, in-browser):** **all 85 functions run, 0 errors.** Correctness spot-checks:
`SHA-256("hello world")` = `b94d27b9…e2efcde9` (the canonical value); BLAKE3 likewise; HKDF → 32 derived
bytes; `McNemar(12,5)` = 36/17 = 2.1176 (dof 1); `F(3; 5,10)` cdf 0.9344, quantile(.95) = 3.326;
`kge_predict` ranks by DistMult score [9, 3.5, 1.5]. All correct.

**⚑ Human:** none. Remaining honest exclusions are unchanged (native-only specialized libs, advanced CAS,
GPU forge, key-gen crypto). If you later want AEAD (AES-GCM / ChaCha20-Poly1305) in-browser with
caller-supplied key+nonce, that's wasm-clean too — say the word (it carries a nonce-reuse footgun, so I left
it out by default).
