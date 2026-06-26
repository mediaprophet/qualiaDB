# Hardware-Backend Bridge — Progress Log

Workstream: implement `HARDWARE_BACKEND_AUTOSELECT_PLAN.md` (the "probe → benchmark →
employ the fastest method per function" bridge) + CLAUDE.md §13. **Allocated to this
instrument by Timothy, 2026-06-27** (he is the project owner/allocator; this overrides
the plan §0 AH-track caution). Branch `0.0.21-la`.

---

## 2026-06-27 — Step 1: bridge foundation (plan P1–P3 core). Status: DONE, verified (no GPU needed).

**What was built** — a new modular library `platform/compute_bridge/` (PROJECT RULE §11),
built **on** the existing real machinery (`device_benchmark`, `hetero_dispatch`, the
`HardwarePassport`) rather than rebuilding it:

- `kernel_class.rs` — the fixed 8-class kernel-shape taxonomy (`DenseLinear`,
  `ElementwiseMap`, `Reduction`, `Stencil`, `AllPairs`, `Fft`, `Scan`, `Divergent`).
  Every hot STEM function classifies into exactly one; routing is **per class**.
- `backend.rs` — the **open** `ProbeableBackend` registry (plan §2.2/§4): the
  expansion point. `BackendId` is a `Copy` string id; adding a backend (`cuda`, `npu`,
  …) is one `register()` call and **never edits the decision tree** — proven by a stub
  backend that registers and is iterated with no change to ranking/policy.
- `reference.rs` — correct CPU reference microkernels, one per class (GEMV, axpb, sum,
  3-pt stencil, N-body potential, **radix-2 FFT**, prefix-sum, Monte-Carlo). These are
  the always-present CPU path AND the correctness reference a GPU kernel must match
  (plan §5 step 4). FFT verified against a naïve DFT and round-trips under inverse.
- `matrix.rs` — the **per-class** capability matrix + the built-in `CpuBackend`
  (real `rayon` rows for every class) and `WgpuBackend` (real GPU number for
  `DenseLinear` via the existing GEMV probe; other classes honestly return no GPU rows
  until their kernels land — never faked).
- `policy.rs` — `ComputePolicy::select(class, bytes) → Plan` (the one shared dispatch
  surface): O(1), zero-heap, **never fails** (CPU is always a valid plan). Wraps
  `hetero_dispatch` for precision / VRAM-tiling / zero-copy. Honours a **measured** GPU
  win on amenable classes; CPU-biases the tie-break for `Divergent` within noise.

**Measured results** — 18/18 `compute_bridge` unit tests pass, **deterministically
without a GPU** (the plan's P3 verifiability: synthetic passports/matrices). Full crate
still green (run after commit). No GPU *execution* numbers yet — see next step; not
measured here, and not extrapolated.

**⚑ Where I need the human (curation-grade calls, plan §8):**
- **Per-class correctness tolerances** (ε for "GPU result matches CPU reference") — a
  curation call, esp. for `medical_computing`. I'll propose defaults; you ratify.
- **`--quick` vs full panel default** on low-tier machines (fast boot vs exact per-class
  winners). I'll default `--quick` on Tier-0 unless you say otherwise.
- **ROCm/oneAPI after CUDA** — same pass or on-demand (you greenlit CUDA P7).

**Next step** — Step 2: wire a **real GPU execution path** through the policy for the
first module (`DenseLinear` GEMV, the kernel that already exists), correctness-gated vs
the CPU reference (test runs-if-GPU-present, skips clean headless), and cache the
`ClassMatrix` in the passport (P2 cache, `PASSPORT_VERSION` bump). Then per the plan's
sequencing: `linear_algebra` → `machine_learning` → physics `Stencil`/`AllPairs` → …,
each module's hot functions routed through `ComputePolicy`.
