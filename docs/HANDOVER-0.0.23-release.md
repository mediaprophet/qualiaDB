# HANDOVER — 0.0.23 release prep + computational-engine demo coverage

**Written:** 2026-06-30, context about to reset. For the next session to pick up **cold** and continue
without re-deriving. **Branch:** `0.0.23` (pushed to `origin/0.0.23`; based on
`feature/p64-manifold-wal-eigensolver`). **Main** is `0.0.21-modality-first`. **Goal:** finish prepping
`0.0.23`, then merge to main + generate binaries.

---

## 0. The single most important framing (do not re-break it)

**The forge PRODUCES + CERTIFIES kernels and transcodes GGUF→p64. The ENGINE RUNS the p64.**
Throughput (tok/s) belongs to the **engine**, not the forge. The forge's `ForgeGraphExecutor` is a
**certification** harness (runs a graph node-by-node and diffs a CPU oracle) — it is **not** the
inference runtime. A prior session mis-framed this ("run the LLM on the forge / forge tok/s"); it was
corrected. Don't treat the forge as a runtime, and don't compare "forge ms/layer" to the engine as if
it were a runtime race.

## 0.1 Working norms (Timothy — non-negotiable)

- **Honesty over everything.** No overclaiming, ever. Measurement honesty: real numbers or "not
  measured"; never extrapolate a kernel figure to end-to-end. Demos that lie are the worst thing to ship.
- **Make the call; don't ask.** He directs; you own the execution. Don't hedge, don't survey-form him,
  don't waste tokens with long deliberation or re-litigating settled points.
- **Plans/docs in repo markdown** (like this file), never hidden plan-mode files.
- **There are no lanes for you** — the whole codebase is in scope.
- **Personal circumstances stay local/private — NEVER in the repo.**

---

## 1. State: what's done + pushed on `0.0.23`

All committed + pushed to `origin/0.0.23` (latest ~`10ba23a8`). Highlights:

- **Version bump to 0.0.23** across all 15 crate Cargo.toml + internal dep pins. Workspace compiles
  (debug green; **release workspace green, 8m10s**; cuda + wasm gates were still building at handover —
  CHECK their result).
- **LLM-on-forge = certification** (not a runtime): `decode_layer_graph` (RMSNorm·weight → Q-proj →
  real RoPE [interleaved+NeoX] → multi-head GQA attention → output proj → SwiGLU → residuals), built on
  new ops `Slice`, first-class `Rope`, and a real `MatMul.trans_b`. The **p64→forge bridge**
  (`graph_ops/p64_bridge.rs`) certifies the decode layer on **real SmolLM2-360M layer-0 weights** vs the
  CPU oracle at **max-rel 3.28e-6** on the A2000.
- **Inference backend selector**: `gpu_context::qualia_backend_override()` (`QUALIA_WGPU_BACKEND` =
  vulkan|dx12|metal|gl) + `recommend_inference_backend()`; `shared_gpu()` logs the backend.
- **Docs**: CHANGELOG 0.0.23 section; `docs/manuals/wgsl-forge.md` updated; `docs/HANDOVER-llm-on-forge.md`
  got a CORRECTION banner; demo honesty fixes (see §3); **`docs/forge-showcase.html`** created (the model
  for new demo pages) + nav entry in `docs/menu.json`.
- **Renderer SDK**: `render::gpu::PortalGpu` now runs on native and WASM wgpu 29. Native rendering
  uses `shared_gpu()`, supports depth/bloom/Tensor10D SOA/mesh/picking, and reads linear RGBA8 into a
  caller buffer. `webizen-render` is a workspace SDK adapter and routes scene PNG helpers through the
  volumetric engine. A real A2000 tensor+mesh render/readback and 41 renderer tests pass.

## 2. Honest findings (settled — don't re-litigate, build on them)

- **Engine decode ≈ 18.8 tok/s** (SmolLM2-360M Q8, RTX A2000 12GB, **Vulkan**, compute-bound: ~63%
  attention, ~37% FFN, ~19% fence). This is THE number to cite.
- **Vulkan is the live default backend** (verified — not DirectML). **DX12 device initialises but decode
  DEADLOCKS** (hung 35 min, no output) — Vulkan is currently the **only working** GPU inference path.
  (Possible real bug to chase later: DX12 fence/poll deadlock during decode.)
- **Ternary 1.58-bit FFN PTQ is a dead end**: PPL ≈ 6.5M on a non-ternary-trained model; AWQ helps but
  Q4_0-FFN-AWQ is +9.4% ΔPPL (≈2× over the 5% gate). **Q4_K_M is the shippable compression** (transcoded
  to p64 verbatim). The "~2.5× ternary FFN win" is **retracted** — never reintroduce it.
- The forge **cannot yet emit the LLM decode graph as shader source** — the cross-backend lowerers
  (`emit/{wgsl,cuda_graph,graph_msl,graph_hlsl}.rs`) lack `Slice`/`Rope`/`MatMul`. The decode layer only
  *runs* through the executor (cert harness). Real SPIR-V emission (`emit/spirv.rs`) exists for the
  gemm/gemv/fft/affine/top-k kit.
- Tensor cores (CUDA WMMA / wgpu coopmat) built + certified as kernels but **not on the decode path**;
  coopmat depends on an **unreleased** wgpu fix (#9741).

## 3. Demo honesty fixes already landed (so you don't redo them)

The demo site was audited; three "the demo lies" blockers were fixed in `0.0.23`:
- **B1** — `docs/edge-llm-showcase.html` shipped a **fabricated** benchmark table (~55/45/120/25/12 t/s on
  hardware never tested). Replaced with the one honest row (~18.8 t/s, SmolLM2-360M Q8, Vulkan, A2000).
- **B2** — DirectML was headlined as the default backend (edge-llm-showcase / edge-llm / advanced-features).
  Reworded: Vulkan is the default; DX12/Metal/GL selectable.
- **B3** — `v0.0.18` stamps → `0.0.23` (menu.json, footers, package.json). **Left alone on purpose:** the
  benchmark-record JSON (`comparative_benchmark_results.json` — it's recorded data) and api.html's "New in
  0.0.18" banner (history). No ternary / "faster than Ollama" claims exist anywhere (grep-verified).

---

## 4. THE NEXT TASK (Timothy's request) — computational-engine demo coverage

> **STATUS UPDATE 2026-06-30 — largely DONE.** Audited (5-agent sweep), then Timothy asked to go further:
> *"update the wasm exports for the full-wasm version, to incorporate everything that can function in wasm,
> then create demos for them all."* Delivered:
> - **`wasm_bridge::engine`** — **76 new `#[wasm_bindgen]` exports** wrapping the wasm-clean solver layer
>   (linear algebra, statistics, CAS, numerics, exact arithmetic, units, transforms, graph). Commits
>   `34e094df` (fix wasm-full build) → `7f320fe3` (exports) → `2fcf288f` (demo + bundle).
> - **`docs/compute-engine.html`** — all 76 as live in-browser cards across 8 tabs; rebuilt `docs/playground`
>   bundle; menu entry. **Verified end-to-end in a real browser: 76/76 run, 0 errors, results correct.**
> - Full record: [`docs/FULL_WASM_COMPUTE_PROGRESS.md`](FULL_WASM_COMPUTE_PROGRESS.md).
> - **Honest boundary:** the 9 `specialized_libs` domain wrappers (finance/medical/chemistry/engineering/
>   full ML/stats/crypto/QPU), advanced CAS, GPU forge, and key-gen crypto are `#[cfg(not(wasm32))]`
>   native-only and stay on the native/MCP path (several already demoed on the science pages). Optional
>   follow-ups Timothy may want: nonparametric stats, KGE link-prediction, hashing-only crypto (no RNG).
>
> The original audit's other findings (api.html stale 0.0.18 stamp + 5.9 tok/s; api-explorer crypto/ML
> over-claims) are honesty fixes still open — see the un-struck text below for the full audit.

Timothy: *"the 'computational engine' (all the math libraries, etc.) — I don't think that's got full
coverage online in the demos. Audit first, then build the additional pages."*

**Step 1 — audit demo coverage of the computational engine.** What capabilities exist vs. what the
`docs/` site actually shows. The earlier session-audit already flagged these **coverage gaps** (Forge is
now DONE via `forge-showcase.html`; the rest are open):
- **GPU solver substrate** — `solvers/` wired through the forge dispatcher: dense linear algebra
  (GEMM/GEMV, **df64** emulated-f64), Lanczos **eigensolver**, **SVD/PCA**, Cholesky; **FFT → transforms →
  audio DSP** (STFT/CQT/HRTF); clustering/classification (kmeans/GMM/SVM) via dispatch. **No demo page.**
- **The 9 specialized libraries** (`specialized_libs/`, 79 tests) — matrix, statistics, machine_learning,
  finance, medical, physics, chemistry, engineering, **cryptographic**; plus symbolic **CAS**. Real,
  MCP-exposed, no focused demo.
- **MCP API surface** (~55 tools, `mcp_server.rs`) — the best live entry point to the engine + libs.
  `docs/api-explorer/` exists; verify it actually exercises the engine tools and is honest.

**Step 2 — build the additional demo pages** (model them on `docs/forge-showcase.html`: same nav/Tailwind
structure, honest framing, real numbers only). Add each to `docs/menu.json` (version already 0.0.23).

**Honesty guardrail for these pages (must-NOT-show as working):** scaffold-only crypto — Kyber/NTRU/
SPHINCS, RSA/ECDSA — and zk-SNARK/Groth16/PLONK (`generate_proof_data` is a SHA-256 commitment, **not** a
proof); QPU = formulation/**simulation**, not physical quantum hardware. Real crypto that IS shippable:
Ed25519, ML-DSA-65 (FIPS-204), AES-256-GCM / ChaCha20-Poly1305 / XChaCha20-Poly1305, SHA-256/512 +
BLAKE3, HKDF. (See CLAUDE.md §8 for the authoritative real-vs-scaffold list.)

## 5. Remaining release-prep before main + binaries

- **Build gates (RESULTS):**
  - Debug workspace: **green**. Release workspace (`cargo build --release --workspace`): **green** (8m10s).
  - CUDA (`cargo build --release -p qualia-core-db --features cuda`): **green** (4m18s).
  - **WASM: FIXED** (2026-06-30). `cargo build -p webizen-lite-wasm --target wasm32-unknown-unknown` is
    **green**. Root cause settled: **a branch regression, NOT pre-existing** — the P64/thermal-WAL/
    eigensolver commits (`c5d6e188`, `f9be78e4`, both branch-only, confirmed *not* on
    `0.0.21-modality-first`) added four un-gated references to native-only modules that compile on
    wasm32 under `wasm-ontology` (which is `[]` — no `wasm-llm`/`wasm-scientific`/`gpu-runtime`/
    `wgsl-forge`). The fix (one cfg-gate each, no logic change):
    - `inference/mod.rs`: `sparse_cache` (uses `crate::solvers`) → `#[cfg(any(not(wasm32), feature="wasm-scientific"))]`;
      `thermal_wal` (uses `memmap2`, a native file-mmap) → `#[cfg(not(wasm32))]`.
    - `lib.rs`: `pub use q42::p64_weight as q42_weight` → gated to mirror the already-gated `p64_weight` re-export above it (`any(not(wasm32), feature="wasm-llm")`).
    - `audio/stft.rs`: the `wgsl_forge::dispatch::fft_f32` call → routed through a new `fft_interleaved()`
      helper that uses the forge when compiled in, else a self-contained naive CPU DFT floor
      (verbatim of the tested `wgsl_forge::oracle::dft_cpu`, identical `exp(-2πi·k·j/N)` convention).
    - `inference/inference_agent.rs` (only compiled on wasm under `wasm-llm`): the two `thermal_wal`
      usages gated `#[cfg(not(wasm32))]` so the native WAL telemetry doesn't leak into a wasm build.
    Verified: wasm build green, native `qualia-core-db` check green, `audio::stft` tests (7) pass on the
    native forge path. No longer a blocker.
  - Non-workspace `webizen-{desktop,render,runtime,studio,web}` crates were NOT built — build if they ship.
- **Then:** merge `0.0.23` → main (`0.0.21-modality-first`) and generate binaries (native binary =
  `qualia-cli`; WASM = `webizen-lite-wasm` / mobile harness via the `wasm-release` profile) — **after the
  WASM build is fixed**.

## 6. Key pointers

- **Model on disk:** `C:/LLM_Models/GGUF/smollm2-360m-instruct-q8_0.gguf` (also hard-linked at
  `docs/models/...` so `find_model` resolves it).
- **Engine decode bench:** `QUALIA_LLM_PROFILE_MODEL=<abs path> cargo test -p qualia-core-db --release
  --test llm_bench_a0 a0_decode_profile -- --nocapture` (gives tok/s + forward/attn/FFN breakdown +
  sync-vs-compute). Honors `QUALIA_WGPU_BACKEND`. **Do not force `dx12` for decode — it hangs.**
- **Backend probe:** `cargo test -p qualia-core-db --lib gpu_context::tests::report_inference_backend --
  --ignored --nocapture` (prints the selected backend).
- **Forge LLM cert (A2000):** `cargo test -p qualia-core-db --lib
  wgsl_forge::graph_ops::p64_bridge::tests::forge_decode_layer_on_real_p64_weights_matches_oracle --
  --ignored --nocapture` (3.28e-6 on real weights).
- **Code:** `crates/qualia-core-db/src/{wgsl_forge, specialized_libs, solvers, gguf_bridge, inference}/`,
  `mcp_server.rs`, `gpu_context.rs` (backend selection). Forge manual: `docs/manuals/wgsl-forge.md`.
- **Existing science demo pages to check/extend:** `docs/scientific-computing.html` (predates the forge
  dispatcher — verify it surfaces the new GPU math), `science-playground.html`, `modalities-showcase.html`,
  `logic-showcase.html`. New page template: `docs/forge-showcase.html`.
- CLAUDE.md is the orientation contract (the LLM-engine-is-not-Ollama warnings, the real-vs-scaffold §8,
  the project rules §9–§14). Read it before writing code.
