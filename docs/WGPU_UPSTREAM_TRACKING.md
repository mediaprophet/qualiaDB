# wgpu / naga upstream issue tracking

A living record of every `wgpu`/`naga` issue this project has hit, **honestly categorized** so
we fork/PR only what is genuinely upstream's to fix — and don't waste effort forking for things
that were our own code or a spec limitation. Pinned wgpu: **30.0.0** (`crates/qualia-core-db/Cargo.toml`,
bumped from 29.0.3 on 2026-07-13 — see the "wgpu 30 upgrade" section at the end).
Last reviewed: **2026-07-13**.

The categories (from Timothy's investigation, which matched our reality precisely):
- **Upstream bug** — a real `wgpu`/`naga` defect. Candidate for the soft-fork → PR path (§ below).
- **App-side** — the WebGPU spec is strict (alignments, binding sizes, exclusive usages); the fix
  belongs in *our* code (`wgsl_forge`), not wgpu. Most of our "wgpu issues" were these.
- **Spec limitation** — the WGSL/WebGPU spec itself lacks the capability; cannot be fork-fixed; the
  answer is the native bypass (CUDA/Metal) or a spec extension.

---

## Issue ledger

| # | Symptom | Category | Status / resolution |
|---|---------|----------|---------------------|
| 1 | **Cooperative-matrix multiply (`coopMultiplyAdd`) returns all zeros** on the SPIR-V/Vulkan path | **Upstream bug** | Root cause = `vulkanMemoryModelDeviceScope` gating ([#9729](https://github.com/gfx-rs/wgpu/issues/9729)); fix [#9741](https://github.com/gfx-rs/wgpu/pull/9741). **Now on wgpu 30.0.0 (2026-07-13):** the crate-wide modernization is done and the `coopmat_usable()` probe was **measured on 30** — it **still returns zeros on this machine's DX12 backend** (`coopmat 0 vs cpu 2.925`). So #9741's memory-scope fix does **not** cover the DX12 path here (it is Vulkan-scoped; DX12 coopmat is a separate matter). The probe **correctly self-gates to `false`**, so the portable coopmat tile stays dormant and never produces wrong results; it self-activates the moment an adapter/backend computes coopmat (e.g. a Vulkan machine). Active tensor cores today remain **CUDA WMMA** (f16, `emit/cuda_c.rs`), reachable via the new `gemm_f32_tc_reduced` entry. |
| 2 | **df64 (double-single) collapses to f32** — driver/naga reassociates `c-(c-a)→a`, `fma(x,y,-(x*y))→0` | **Spec limitation (ambiguous naga/driver)** | WGSL has **no portable pragma** to forbid float reassociation; the naga→SPIR-V→NVIDIA-Vulkan path reassociates. Proven not a missing-fma (Veltkamp split gave byte-identical wrong result). Workaround in place: runtime `df64_usable()` probe gates the df64 tier (`dispatch.rs`). **Action:** low priority — a fix needs an fp-contraction control in WGSL/naga or a spec extension; CUDA-f64 + the exact CPU floor already cover f64. Track, don't fork. |
| 3 | **WGSL has no `f64`** (only f32/f16/i32/u32) | **Spec limitation** | WebGPU/WGSL language spec. Not a wgpu bug, not fork-fixable. Answer = **native CUDA-f64** (`gemm_f64_cuda`) + CPU floor. Permanent; revisit only if WGSL ever adds f64. |
| 4 | Ray-query **acceleration-structure limits default to 0** (`max_blas_geometry_count` etc.) | **App-side** | wgpu's conservative defaults; must be raised in `DeviceDescriptor`. **Fixed** (`execute/wgpu.rs` raises them from adapter values). Not a bug. |
| 5 | **"Buffer bound with conflicting usages" validation error** — same buffer read-write + read-only in one dispatch | **App-side (spec-correct)** | `read_write` is an exclusive usage per the WebGPU spec; wgpu correctly rejects it. **Fixed** in DAG-IR P4 (two-slab discipline + `copy_view` GPU→GPU hand-off, `graph_ops/executor.rs`). Not a bug. |
| 6 | `device.poll(Maintain::Wait)` no longer compiles | **API evolution** | wgpu 29 renamed `Maintain` → `PollType`. **Modernized** (CLAUDE.md §13). Not a bug. |
| 7 | **WebGPU module panics `"Unexpected error"` (webgpu.rs:85) on `pop_error_scope().await` when the scope caught *no* error** — aborts the wasm module during portal GPU init, blacking out `playground/anatomy.html` | **Upstream bug** (wgpu 30.0.0 × wasm-bindgen ≥0.2.123) | `GPUDevice.popErrorScope()` resolves to `GPUError \| null`, returning JS **`null`** on the no-error path. wgpu's `future_pop_error_scope` (`backend/webgpu.rs:1067`) feeds that through `js_sys::JsOption::into_option()`, which since **wasm-bindgen 0.2.123** treats `null` as a *present* value (only `undefined` is absent — see the `into_option` doc). So `null` → `Some(null)` → `Error::from_js(null)` → `panic!("Unexpected error")`. Confirmed live in Chrome (real WebGPU) 2026-07-15: the error passed to `from_js` logged as `ctor=null msg=null`, stack `MakeSendFuture::poll → from_js → panic`, from `PortalGpu::try_new_async`. **App-side mitigation shipped** (`render/gpu/mod.rs`): on `wasm32` we `drop(error_scope)` instead of `.pop().await` — the guard's `Drop` still pops the scope but never polls the buggy future, so it cannot panic. Native keeps full `.pop().await` error surfacing. **Proper upstream fix** (soft-fork candidate): make `future_pop_error_scope` treat a `null`/`undefined` pop result as `None` (guard before `from_js`), and/or harden `Error::from_js` to map an unrecognised `GPUError` to `Error::Internal` rather than panicking. |

**Net:** **two** genuine upstream bugs — coopmat (#1, fix merged upstream awaiting release) and the
`pop_error_scope` null panic (#7, app-side mitigation shipped, soft-fork of `future_pop_error_scope`
still the proper fix). Everything else was app-side (fixed) or a spec limitation (bypassed natively).
We still should **not** hard-fork wgpu; #7 is a one-function soft-fork when we do the next patch pass.

---

## The soft-fork workflow (the path to improvement — for #1, and any future upstream fix)

Do **not** hard-fork. Use a temporary, localized Cargo patch, exactly as scoped:

1. **Clone upstream locally** next to the workspace:
   `git clone https://github.com/gfx-rs/wgpu ../wgpu` (and `git checkout` the commit that includes the fix).
2. **Patch the workspace `Cargo.toml`** (root), on a **throwaway branch** (do not land this on the
   project's main pin until validated):
   ```toml
   [patch.crates-io]
   wgpu = { path = "../wgpu/wgpu" }
   # or, to pull the merged fix directly without a local clone:
   # wgpu = { git = "https://github.com/gfx-rs/wgpu", rev = "<commit-with-#9741>" }
   ```
3. **Test the actual pipeline**: run the coopmat GPU-certify test (`wgsl_forge` coopmat oracle) on the
   **A2000** and confirm `coopMultiplyAdd` now returns correct (non-zero, oracle-matching) results.
4. **If it works**: light up the WGSL coopmat tensor-core path (the code already emits it; it was gated
   off because execution returned zeros) as a peer to the CUDA WMMA path; keep CUDA WMMA as the floor.
5. **Un-pin when released**: watch crates.io for a wgpu release that includes #9741, then drop the patch
   and bump the pin. (We did **not** write the fix, so there is no PR to submit for #1 — it's merged.)

**Honest caveats:**
- Pinning to wgpu `main` brings in *all* of main's churn since 29.0.3 — possible new API breaks / instability.
  Hence the **throwaway-branch** discipline: validate coopmat there; only adopt if the rest of the build
  stays green. The earlier call ("don't pin the main project to git") still holds for the *production* pin.
- #9741 fixes the memory-scope **gating**; whether it fully resolves the *returns-zeros computation* on the
  A2000 must be **measured**, not assumed. The test in step 3 is the gate.

---

## The native-bypass strategy (already in place — Step 3 of the investigation)

We already follow the "route limiting ops through native bridges" strategy:
- **CUDA** (`execute/cuda.rs`, `emit/cuda_c.rs`) is the native bypass for the two things WGSL/wgpu can't
  do: **tensor-core GEMM (WMMA)** and **f64**. The DAG-IR **`CudaCLowerer` (plan P5)** generalizes this —
  the *same* compute graph lowers to CUDA-C with no per-id branches.
- `metal_bridge.rs` / `directml_bridge.rs` / `npu_ffi.rs` are the corresponding native paths for Apple /
  Windows-ML / NPU targets (future).
So wgpu carries the portable, general path; native bridges carry the silicon-specific hot paths. This is
the intended division and it is why the coopmat block was never a true blocker — only a missing *portable*
path that CUDA covered.

---

## Project posture for v1 (Timothy, 2026-06-29)

**We are working toward the first build, so experimental solutions are acceptable — this is the
first version, not the last.** Concretely, this *relaxes* the earlier "don't pin the production
project to git": for v1 it is fine to **pin wgpu to a git rev** to pick up a merged-but-unreleased
fix (e.g. #9741) *provided the full build + GPU suite stay green*, and to ship **experimental,
dormant-but-ready** kernels (probe-gated so they never produce wrong results). **Authoring PRs back
to upstream is sanctioned** when we carry a fix of our own. The bar stays: probe-gate anything that
might be wrong on a given adapter, and keep a correct CPU/plain floor.

## The tensor-core reality (wgpu 30)

Both tensor-core backends now have tiled orchestration and capability-selected dispatch:
- `emit/coopmat.rs::matmul_tc_wgsl_tiled` tiles **8×8×8** all-f32 cooperative matrices over
  `M/N/K`; a measured `coopmat_usable()` probe prevents use on backends that still return zeros.
- `emit/cuda_c.rs::WMMA_GEMM_16X16` tiles **16×16×16** warps with f16 input and f32
  accumulation (hardware-verified on the A2000).

The precision split is explicit: `gemm_f32_tc` selects only accurate f32 coopmat or the plain-f32
floor, while `gemm_f32_tc_reduced` opts into CUDA WMMA's reduced input precision. The stage-4
selector test verifies the accurate entry point stays below `1e-2` maximum error.

## Action items

- [x] **DAG-IR P4c — tiled tensor-core GEMM + capability-selected dispatch:** implemented for
      WGSL coopmat and CUDA WMMA, with measured runtime gating and precision-specific entry points.
- [x] **Upgrade to the release carrying the upstream work:** pinned to wgpu/naga 30.0.0 and
      modernized the crate-wide API (`PollType`, fallible mapped ranges, device descriptors).
- [ ] Re-certify the coopmat oracle on each backend/driver update. wgpu 30 DX12 on the A2000 still
      returns zeros, so the measured gate correctly leaves portable coopmat dormant there.
- [ ] If df64-reassociation (#2) ever blocks a real consumer, file a naga issue requesting an
      fp-contraction / no-reassociation control; until then the runtime probe is sufficient.

---

## Soft-fork attempt result (2026-06-29) — #9741 fix is real, but the unreleased commit has crate-wide API drift → WAIT for release

Attempted the Task #56 soft-fork (`[patch.crates-io]` wgpu+naga → gfx-rs/wgpu commit
`56535d7d` = the merged #9741 fix) to verify the WGSL coopmat multiply on the A2000. **Honest
outcome: NO-GO for now**, for a reason that is *not* the coopmat fix itself:

- **Feasibility confirmed:** wgpu at that commit is workspace version **29.0.0** (semver-compatible
  with our `29` pin). Bumping the local clone to `29.0.4` made Cargo prefer the patch cleanly; all
  wgpu sub-crates (wgpu/-core/-hal/-types, naga) resolved to the clone. **wgpu itself compiled.**
- **Blocker — crate-wide API drift (58 days of `main` past 29.0.3):** `qualia-core-db` fails to
  compile against the commit with ~25 errors from **public-API changes**, not the HAL fix:
  - `BufferSlice::get_mapped_range()` now returns `Result<BufferView, MapRangeError>` (was the view
    directly) → 7+ index/deref sites + ~17 `mismatched types`;
  - `RequestAdapterOptions` gained a required field `apply_limit_buckets`.
- **Lane boundary — decisive:** those call sites are in `platform/gpu.rs`, `tensor/volume_gpu.rs`,
  and **`gguf_bridge/output.rs` (the LLM lane, off-limits per AGENTS §10)** — *not* confined to
  `wgsl_forge`. Modernizing them to verify a probe-gated dormant kernel would (a) reach into another
  instrument's lane and (b) pin the **whole crate** to an *unreleased* wgpu API. Not worth it.

**Decision:** revert to the stable crates.io pin (done; tree clean) and **wait for the fix to ship
in a wgpu release** (Task #57) — then bump the pin and do the buffer-mapping/`RequestAdapterOptions`
modernization as one deliberate dependency bump (project rule §13), crate-wide, with the lane owners.
The coopmat path stays built + naga-validated + `coopmat_usable()`-probe-gated, so it self-activates
on that release with zero further forge work. Active tensor-core GEMM today remains **CUDA WMMA**
(certified). The recipe (commit `56535d7d`, version-bump trick) is recorded here for the release bump.
