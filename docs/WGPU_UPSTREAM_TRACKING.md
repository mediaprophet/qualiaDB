# wgpu / naga upstream issue tracking

A living record of every `wgpu`/`naga` issue this project has hit, **honestly categorized** so
we fork/PR only what is genuinely upstream's to fix — and don't waste effort forking for things
that were our own code or a spec limitation. Pinned wgpu: **29.0.3** (`crates/qualia-core-db/Cargo.toml`).
Last reviewed: **2026-06-29**.

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
| 1 | **Cooperative-matrix multiply (`coopMultiplyAdd`) returns all zeros** on the SPIR-V/Vulkan path | **Upstream bug** | Root cause = `vulkanMemoryModelDeviceScope` gating ([#9729](https://github.com/gfx-rs/wgpu/issues/9729)). **Fix [#9741](https://github.com/gfx-rs/wgpu/pull/9741) MERGED to `main` 2026-06-29, UNRELEASED** (latest crates.io is 29.0.3). Workaround in place: **tensor cores via CUDA WMMA** (`emit/cuda_c.rs` `WMMA_GEMM_16X16`). **Action:** soft-fork test-patch (§) → verify WGSL coopmat on the A2000 → if correct, light up the portable coopmat path; un-pin when a crates.io release ships the fix. |
| 2 | **df64 (double-single) collapses to f32** — driver/naga reassociates `c-(c-a)→a`, `fma(x,y,-(x*y))→0` | **Spec limitation (ambiguous naga/driver)** | WGSL has **no portable pragma** to forbid float reassociation; the naga→SPIR-V→NVIDIA-Vulkan path reassociates. Proven not a missing-fma (Veltkamp split gave byte-identical wrong result). Workaround in place: runtime `df64_usable()` probe gates the df64 tier (`dispatch.rs`). **Action:** low priority — a fix needs an fp-contraction control in WGSL/naga or a spec extension; CUDA-f64 + the exact CPU floor already cover f64. Track, don't fork. |
| 3 | **WGSL has no `f64`** (only f32/f16/i32/u32) | **Spec limitation** | WebGPU/WGSL language spec. Not a wgpu bug, not fork-fixable. Answer = **native CUDA-f64** (`gemm_f64_cuda`) + CPU floor. Permanent; revisit only if WGSL ever adds f64. |
| 4 | Ray-query **acceleration-structure limits default to 0** (`max_blas_geometry_count` etc.) | **App-side** | wgpu's conservative defaults; must be raised in `DeviceDescriptor`. **Fixed** (`execute/wgpu.rs` raises them from adapter values). Not a bug. |
| 5 | **"Buffer bound with conflicting usages" validation error** — same buffer read-write + read-only in one dispatch | **App-side (spec-correct)** | `read_write` is an exclusive usage per the WebGPU spec; wgpu correctly rejects it. **Fixed** in DAG-IR P4 (two-slab discipline + `copy_view` GPU→GPU hand-off, `graph_ops/executor.rs`). Not a bug. |
| 6 | `device.poll(Maintain::Wait)` no longer compiles | **API evolution** | wgpu 29 renamed `Maintain` → `PollType`. **Modernized** (CLAUDE.md §13). Not a bug. |

**Net:** exactly **one** genuine upstream bug (coopmat, #1) — and its fix is already merged upstream,
awaiting a release. Everything else was app-side (fixed) or a spec limitation (bypassed natively). We
should **not** hard-fork wgpu.

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

## Action items

- [ ] **Re-check crates.io for a wgpu release that includes [#9741](https://github.com/gfx-rs/wgpu/pull/9741)**; when published, bump the pin and delete any patch. (Tracked task.)
- [ ] **Soft-fork test-patch experiment** (steps 1–3 above) to verify WGSL coopmat on the A2000 with the merged fix; light up the portable coopmat path if green. (Tracked task — discrete experiment, needs Timothy's go since it perturbs the dep pin.)
- [ ] If df64-reassociation (#2) ever blocks a real consumer, file a naga issue requesting an fp-contraction / no-reassociation control; until then the runtime probe is sufficient.
