> **Direction update (2026-07-09):** active decision architecture is
> [
ative-inference-explorer-eval-plan.md](native-inference-explorer-eval-plan.md)
> (executor vs explorer + evaluation programme). This file remains the workstream ledger.
# Native Inference Pipeline Remediation Plan

**Status:** plan (ready to execute)  
**Date:** 2026-07-09  
**Principal steer:** models convert to **p64 + q42 helpers** (GGUF/safetensors = import only); engine runs native; forge owns conversion + calibration + (eventually) training; platform auto-selects the fastest available path.  
**Progress log (when execution starts):** `INFERENCE_P64_PIPELINE_PROGRESS_LOG.md`  
**Related:** `docs/manuals/qualia_db_functionality_manual.md` (ecosystem overview), `docs/manuals/p64-q42-inference-pipeline.md`, `docs/manuals/wgsl-forge.md`, `docs/manuals/inference-tuning.md`, `docs/manuals/model-compression.md`, `docs/plans/acceleration-integration-map.md`, `docs/WGPU_UPSTREAM_TRACKING.md`, `docs/manuals/standards/p64-weight-container-standard.md`

---

## 0-A. Optimization toolkit (use these — do not reinvent)

**Principal note (2026-07-09):** the codebase already carries a large, deliberate optimization
stack. Further perf work must **compose and wire** that stack into the production decode path,
not hand-roll parallel infrastructure. The ecosystem overview is
[`docs/manuals/qualia_db_functionality_manual.md`](../manuals/qualia_db_functionality_manual.md);
the *operating manuals* for optim are the specialized docs below.

### Manuals that define the toolkit

| Manual | What it is for |
|--------|----------------|
| `docs/manuals/qualia_db_functionality_manual.md` | Full-ecosystem map (engine, client, desktop, extensions) — **what exists** |
| `docs/manuals/p64-q42-inference-pipeline.md` | Canonical runtime path: source → P64 → residency → prefill/decode; **Forge boundary** (§10) |
| `docs/manuals/wgsl-forge.md` | Generate → Naga validate → CPU/GPU oracle → **certify** → **tune** → adapter-keyed cache |
| `docs/manuals/inference-tuning.md` | Production decode toggles (resident decode/weights, FFN fusion, coop GEMV, GPU top-k, …) + smoke gates |
| `docs/manuals/model-compression.md` | Container-agnostic PTQ / prune / distill — convert-time weight policy |
| `docs/plans/acceleration-integration-map.md` | Inventory: which shaders migrate to certified forge kernels |
| `docs/WGPU_UPSTREAM_TRACKING.md` | Coopmat zeros, CUDA WMMA bypass, P4c tiled tensor-core GEMM |

### Concrete toolkit surfaces (already in-tree)

| Layer | Surface | Role in closing the 15× gap |
|-------|---------|------------------------------|
| **WGSL Forge** | `wgsl_forge/` + `qualia-cli shader list-kernels\|generate\|validate\|certify\|tune\|profile-hardware\|auto-tune-all` | Deterministic kernels, schedules, oracle gates, adapter manifests |
| **Forge P64 bridge** | `wgsl_forge/graph_ops/p64_bridge.rs` | Layer math certify vs CPU oracle on real P64 weights |
| **Forge CUDA / WMMA** | `execute/cuda.rs`, `emit/cuda_c.rs` (`WMMA_GEMM_TILED_*`), `dispatch::gemm_f32_tc` | Tensor-core path on NVIDIA (must be **wired into prefill/heavy GEMMs**, not left oracle-only) |
| **Forge calibration** | `wgsl_forge/calibration/` (corpus, KV dictionary, package) | Offline data-driven optim; go/no-go packages |
| **HardwarePassport** | `platform/device_benchmark.rs`, `hardware_passport.rs`, `qualia-cli llm passport` | Measured circuit rank (DX12/Vulkan/Metal/CPU); cache + env pin |
| **Decode toggles** | `inference_bench` / `docs/manuals/inference-tuning.md` | Resident decode, resident weights, FFN fusion, coop GEMV, GPU top-k — already ON by default |
| **Decode profile harness** | `tests/llm_bench_a0.rs` (`a0_decode_profile`, a1a–a1d) | Before/after tok/s + waits/token + path counters |
| **P64 convert / layouts** | `compile_gguf_to_p64_with_layout` (Verbatim / F16Expand), CLI `llm convert` | Conversion-time freedom llama.cpp GGUF readers lack |
| **Q42 model helper** | `q42/model_helper.rs` (`.q42.cbor-ld`) | Stop sets, chat family, layout provenance — not JSON |
| **Ternary / AWQ P64** | `compile_gguf_to_p64_*_ffn*`, `QUALIA_LLM_TERNARY_FFN` | Smaller traffic; ΔPPL-gated |
| **Model compression lib** | `specialized_libs/.../ModelCompression` | PTQ/prune independent of GGUF |
| **Thermal / residency** | `ThermalGovernor`, `residency_planner`, `orchestrator` ModelLifecycle | Power/VRAM budgets for off-grid and 12 GB cards |
| **Phase-8 / Sentinel** | SPSC LogitStream / ControlStream | Speculative accept + mid-gen deny (quality + optional draft speedup) |
| **10d / manifold** | `container_10d/`, manifold LTL/ASP | Structured context inject (accuracy @ fixed token budget) |

### Operating rule for remaining workstreams

1. **Measure with the harness** (`a0_decode_profile` / comprehensive-test) and **passport** — not narrative.
2. **Certify with Forge** before claiming a kernel is production-ready (oracle + schedule + adapter id).
3. **Prefer toggles and paths already default-ON** before inventing new dispatch graphs; fix ineligibility/fallback first.
4. **Layouts live at convert time** (P64 + helper), not as ad-hoc GGUF reinterpretation in the hot loop.
5. **Forge certifies; `QTensorEngine` owns production decode** until a certified kernel is explicitly swapped in behind a toggle + auto-fallback (`p64-q42-inference-pipeline.md` §10).

### CLI cheat-sheet (operator / agent)

```powershell
# Hardware rank (measured GEMV matrix)
cargo run -p qualia-cli --release -- llm passport --reprobe --apply-env-hint

# Convert import → native package
cargo run -p qualia-cli --release -- llm convert <model.gguf> --out <dir> --layout verbatim   # or f16

# Forge: certify / tune a kernel on this adapter
cargo run -p qualia-cli --release -- shader list-kernels
cargo run -p qualia-cli --release -- shader certify <kernel> --manifest out.json
cargo run -p qualia-cli --release -- shader tune <kernel> --max-candidates 48 --cache-dir .qualia/wgsl-forge
cargo run -p qualia-cli --release -- shader profile-hardware
cargo run -p qualia-cli --release -- shader auto-tune-all --budget-ms 120000

# Decode profile (honest tok/s + fences)
$env:QUALIA_LLM_PROFILE_MODEL='C:/LLM_Models/P64/smollm2-360m-instruct-q8_0.p64'  # prefer p64
cargo test -p qualia-core-db --release --test llm_bench_a0 a0_decode_profile -- --nocapture
```

---

## 0. Reality check (evidence, not theatre)

Measured on this machine (RTX A2000 12 GB, 2026-07-09):

| Path | Result |
|------|--------|
| Ours · DX12 · llama-3.2-3B Q4_K_M | ~4–5 tok/s decode · ~9 tok/s prefill · coherent on known facts; open prompts still weak |
| Ollama/llama.cpp CUDA · same GGUF | ~70 tok/s |
| Vulkan · 3B Q4_K_M | hang / 0 tok in 600s (DX12 is the working Windows default now) |
| Metric display | **fixed** in CLI (`tokens_generated`); re-measure still needed on 3B |
| `.p64` on disk | **productized** (`llm convert` + vault prefer-p64); smollm2 converted on this machine |

### What exists vs what decode actually uses *(updated 2026-07-09 mid-execution)*

| Capability | Library status | On decode path? |
|------------|----------------|-----------------|
| Subgroup GEMV (`coop_gemv_sg`) | Built when adapter has `SUBGROUP` | **Yes** (if feature present) |
| Resident decode / weights / FFN fusion / GPU top-k | Toggles default ON (`inference-tuning.md`) | **Yes** when eligible |
| CUDA WMMA tiled | Forge `WMMA_GEMM_TILED_*` + `gemm_f32_tc` | **No** — forge/oracle only |
| Forge CUDA executor | `execute/cuda.rs` | **No** — not called from `gguf_bridge` |
| `compile_gguf_to_p64` + F16Expand | CLI `llm convert --layout verbatim\|f16` | Convert yes; f16 path needs measure |
| Multi-stop + `.q42.cbor-ld` helper | Q42T v2 + ModelHelper CBOR-LD | **Yes** (merge on activate) |
| HardwarePassport | `llm passport` + gpu_context reads cache | **Yes** (when env unset) |
| Vault prefer `.p64` | scan/resolve/discover | **Yes** |
| Forge trainer | Calibration only | **No** LoRA train yet |
| `.10d` aligned sections | Real (mesh/tensor/provenance) | **Not** weight payload yet |

**Honest bound:** subgroup reduction helps a little; it is **not** the 15× gap. Closing that gap needs (a) conversion-time weight layouts in p64, (b) tiled GEMM / native tensor-core path for the heavy matmuls, (c) fewer submit/fence boundaries, (d) correct stop/sampling metadata. Conversion alone (byte-identical p64) changes speed by **zero**.

**Stale docs to fix when executing:** `inference-ecosystem-optimization-EXECUTION.md` still says “Vulkan only / DX12 hangs” — that flipped 2026-07-09 (DX12 default; Vulkan hangs on large Q4_K).

---

## 1. Target architecture (locked by principal)

```
┌─────────────────┐     forge convert      ┌──────────────────────────────┐
│ GGUF / safetensors │ ──────────────────► │ .p64  (weights, GPU layout)  │
│ (import only)      │     + certify        │ .q42  (tokenizer, stop set,  │
└─────────────────┘                        │       chat family, sampling, │
                                           │       provenance NQuins)     │
                                           │ .10d  (optional GPU pages /  │
                                           │       mesh / tensor sidecars)│
                                           └──────────────┬───────────────┘
                                                          │ activate
                                                          ▼
                                           ┌──────────────────────────────┐
                                           │ Engine runs NATIVE containers │
                                           │ Backend = measured best for   │
                                           │ this host (passport cache)    │
                                           └──────────────────────────────┘
```

**Rules:**
1. Engine activation prefers sibling `.p64` over `.gguf`. Foreign containers are **import sources**, never the steady-state runtime format.
2. Conversion may rearrange bytes for GPU (SoA Q4_K, f16 pages, upload descriptors). That freedom is the point of p64.
3. q42 carries behaviour metadata so quality bugs (wrong stop tokens, missing template) cannot reappear as engine guesswork.
4. Forge owns: convert → calibrate → (later) train LoRA → certify kernels. Engine owns: load p64, run selected backend, Webizen gates.

---

## 2. Implementation requirement: subgroups + tensor cores (explicit)

You are right that much of this was *started*. It is **not finished** as a decode product.

| Layer | Required work | Acceptance |
|-------|---------------|------------|
| **Subgroup GEMV** | Keep `coop_gemv_sg` as default when `SUBGROUP`; measure occupancy vs shared-memory fallback on A2000; remove dead paths only after identity tests | Identity vs fallback; profile shows which path is live |
| **Tiled GEMM orchestration (P4c)** | Full M×N×K loop over WMMA 16×16 (CUDA) and coopmat 8×8 (WGSL, probe-gated) with shared-memory K staging | Forge certify + CPU oracle; not a single-tile unit test alone |
| **Decode/prefill wiring** | Heavy GEMMs (prefill QKV/FFN, output proj when batch>1) route through capability-selected MatMul: `coopmat → CUDA WMMA → plain WGSL` | Decode path call graph shows forge/native MatMul, not only scalar GEMV |
| **CUDA pipeline product** | `feature = "cuda"` builds; runtime probe; decode/prefill can select CUDA when it wins the passport | End-to-end tok/s on NVIDIA with CUDA selected when faster than wgpu |
| **WGSL coopmat** | Stay probe-gated until wgpu ships #9741 (or verified soft-fork); never silently return zeros | `coopmat_usable()` false → plain path |

**This is a first-class workstream (W-K), not a footnote.** Prior “done” claims applied to primitives, not to the inference product.

---

## 3. Workstreams (priority order)

### W0 — Measurement honesty (1 session)

| Item | Fix | Acceptance |
|------|-----|------------|
| `Tokens: 1` metric | Count emitted decode tokens (or prefill+decode with clear labels); never report `token_ids.len()` if that buffer is last-id only | CLI comprehensive-test tok/s within ~10% of wall-clock tokens/time |
| Baseline table | Re-run smollm2-360m **and** llama-3.2-3B on DX12 with fixed prompts; record GPU util via `nvidia-smi dmon` during decode | Numbers in progress log; no extrapolation 360M→3B |
| Regression arithmetic | Document expected traffic scaling (~9× weights 360M→3B) so 4–5 tok/s is not misread as a 360M regression | One table in progress log |

### W1 — Quality stops + sampling metadata (1 session)

| Item | Fix | Acceptance |
|------|-----|------------|
| Multi-stop decode | Stop on `eos` **and** family chat-end ids (`<|eot_id|>`, `<|im_end|>`, …) resolved from vocab or q42 | Open prompt ends after answer; no “quiz” continuation from max-token runoff |
| Persist in q42 | Q42T v2: stop id set, chat family, default `SamplerConfig` | Converted model stops correctly without engine special-cases |
| Cap honesty | Max-new-tokens reported separately from stop reason | UI/CLI shows `stop=eot` vs `stop=max` |

### W2 — Productize p64/q42 (the designed pipeline) (2–3 sessions)

| Item | Fix | Acceptance |
|------|-----|------------|
| CLI | `qualia-cli model convert <gguf\|safetensors> --out dir` → `.p64` + `.q42` | Command exists; round-trip activate |
| Desktop/scheduler | Import job + progress events | User can convert without CLI |
| Vault | Scanner accepts `.p64`; activation prefers p64 sibling | App never requires live GGUF after convert |
| Boot cost | Parse-once + table/hardware CRC-32C (today’s multi-parse table-less CRC is slower than GGUF) | p64 activate ≤ GGUF activate on same weights |
| Hard cap | Document 4 GiB u32 offset limit; design **p64 v5 u64 offsets** for FP16 3B+ | Spec + issue; do not claim f16-3B container until v5 |

**Conversion-time layouts (in order of payoff):**

1. **Q42T metadata** (quality) — stop/template/sampler — *no speed claim*.
2. **f16 pre-dequant pages** for models that fit (≤~2B on 12 GB with headroom) — use existing `unpack2x16float` path; measure GEMM ×.
3. **Q4_K SoA / word-aligned planar** — coalesced loads instead of per-byte extract — measure ×.
4. **Upload descriptors + NQuin provenance sidecar** — governance + zero re-scan.

Until (2)(3) land, treat p64 as **product correctness + future hook**, not a tok/s miracle.

### W3 — Kernel / native backends (the 15× work) (multi-session)

| Track | Work | Acceptance |
|-------|------|------------|
| **W-K1** | Finish P4c tiled WMMA GEMM (CUDA) | Forge certify on A2000; microbench ≫ plain f32 GEMM |
| **W-K2** | Wire prefill + large matmuls to MatMul selector | Prefill tok/s uplift measured |
| **W-K3** | Decode GEMV: keep subgroup; add layout-aware kernels matching p64 SoA/f16 | Decode uplift measured vs Q4_K byte path |
| **W-K4** | CUDA end-to-end optional path for NVIDIA (`cudarc` already in forge) | Passport can pick CUDA; fallback wgpu on fail |
| **W-K5** | Vulkan hang isolation (2 GiB storage binding vs ~1.93 GiB weights is lead suspect) | Matrix: model size × backend; either fix or document hard skip |

### W4 — Platform capability passport + auto-select (your explicit requirement)

**Design:** on first run (and when hardware/driver fingerprint changes), probe **all available** compute paths, rank by measured throughput on representative kernels, persist selection, use it for inference.

| Target | Backend candidates | Priority |
|--------|-------------------|----------|
| **Windows x64** | wgpu DX12, wgpu Vulkan, CUDA (if toolkit/driver), CPU | **Primary** |
| **Apple** | wgpu Metal, Metal Performance Shaders bridge (`metal_bridge`), CPU | **Primary** (parity with Windows) |
| **Mobile** (iOS/Android later) | Metal / Vulkan / GLES via wgpu where available; CPU floor | **Primary architecture**, ship when mobile shell is real |
| **Linux** | wgpu Vulkan, CUDA, CPU | **Secondary** (must not block Windows/Apple) |

**Reuse / extend existing code:**
- `platform/device_benchmark.rs` — GEMV matrix (today: wgpu adapters + CPU; NPU “not probed”).
- `wgsl_forge::backend::resolve_execution_backend` — preferred → fallback chain for forge targets.
- `wgsl_forge` `profile-hardware` / `auto-tune-all` — adapter-keyed manifests.
- GPU context env `QUALIA_WGPU_BACKEND` — manual override must remain.

**Build:**

```
HardwarePassport {
  host_fingerprint,   // PCI ids / Metal device / driver versions
  probed_at,
  circuits: [ { id, backend, ms_gemv, ms_gemm_tile, upload_gbps, decode_proxy_tok_s } ],
  selected_inference_backend,
  selected_forge_target,
  disqualified: [ { id, reason } ],  // e.g. Vulkan hang on Q4_K > threshold
}
```

| Step | Deliverable | Acceptance |
|------|-------------|------------|
| W4.1 | Extend probe: DX12 vs Vulkan vs (CUDA if feature) vs CPU; short GEMV + one GEMM tile + one upload | Matrix JSON written under app data |
| W4.2 | Optional 8–16 token decode micro-bench on small model (smollm2) for ranking when time budget allows | Rank uses decode proxy when available |
| W4.3 | Persist passport; load at activate; env override wins | Second launch does not re-probe unless fingerprint changes |
| W4.4 | Disqualify backends that hang/timeout (record reason) | Vulkan-large never selected after one failed probe |
| W4.5 | Apple Metal path in same passport schema | CI or Mac host produces passport with Metal selected when fastest |
| W4.6 | Surface in CLI: `qualia-cli hardware passport` / `reprobe` | Operator can force re-rank |

**Principle (already in device_benchmark):** rank by **measured** throughput, not a static “discrete GPU always wins” hierarchy.

### W5 — Prefill / fence (continue existing W1/W3)

- Keep resident decode; prove waits/token ≈ 1–2 with honest profile.
- Prefill arena: one submit per chunk; cold TTFT before/after.
- Do not claim fence work as tensor-core work.

### W6 — Container family: p64 ↔ .10d

**Decision options (pick one in implementation, document in p64 standard):**

| Option | Meaning | When |
|--------|---------|------|
| **A (recommended)** | p64 remains weight container; adopts 10d’s `AlignmentTier` + provenance section discipline | Fastest path to GPU-friendly pages without merging formats |
| **B** | Weights become a 10d section type (`SectionType::ModelWeights`) alongside mesh/tensor | One family for all GPU assets; larger migration |
| **C** | Hybrid: p64 for mmap weights; 10d holds kernel blobs / forge manifests / mesh context | Good if forge artifacts should travel with the model |

Default plan: **A now**, leave B as a later unification if dual loaders hurt.

### W7 — Forge trainer (scoped)

| Phase | Scope | Acceptance |
|-------|-------|------------|
| T0 | Document: calibration ≠ training (honest) | Manual updated |
| T1 | LoRA backward for existing forward kernels (rank-r, A2000-sized) | Unit: loss decreases on tiny corpus; adapter loads in `adapter_manager` |
| T2 | Certify backward vs CPU oracle (forge discipline) | Oracle gate green |
| T3 | CLI `forge train-lora` producing adapter artifact next to p64 | Round-trip: train → load → generate |

Not in v1: full SFT of 3B base weights.

---

## 4. Novel levers (measurable, ecosystem-native)

These are not “run GGUF faster”; they use Qualia capabilities foreign runtimes lack.

| Lever | Mechanism | Metric |
|-------|-----------|--------|
| **Conversion-time layout** | p64 SoA / f16 pages / precomputed bind layouts | GEMV/GEMM µs, tok/s |
| **Speculative decode + Sentinel** | Draft small model / topology draft; Sentinel deny-rollback already in Phase 8 | accepted tokens / wall time |
| **Prefix / RAG KV reuse** | Semantic retrieval → pinned prefix KV; avoid re-prefill | TTFT on multi-turn |
| **Ontology-routed short context** | Existing ontology router → smaller effective context | prefill tokens/turn, quality @ fixed budget |
| **Ternary / AWQ FFN p64 variants** | Already compiled in tests; productize when ΔPPL gate holds | tok/s + ΔPPL ≤ 5% |
| **ThermalGovernor + passport** | Prefer efficiency curve under battery/off-grid | tok/s per watt (nvidia-smi / IOKit) |
| **Manifold / 10d context inject** | Geometric state as structured context, not raw tokens | task accuracy @ fixed token budget |
| **Dict-KV / calibration packages** | Forge calibration already real — ship as install step | decode ms after calibrate vs cold |
| **GPU top-1 stay-on-device** | Avoid 513 KB full-vocab readback when greedy | ms/token output phase |
| **Chunked / sampled vocab** | Full sort of 128k logits is optional; top-k block reduce already exists | sampler path ms |

Each lever needs a **before/after on the same harness** — no narrative wins.

---

## 5. Sequencing DAG

```
W0 measurement honesty ─────────────────────────────┐
W1 stop tokens + q42 metadata ──► quality usable     │
W2 convert CLI + vault + CRC ──► designed pipeline   ├──► W3 kernels (layouts need W2)
W4 passport probe (can start parallel; uses W0 harness)
W5 fence/prefill (parallel with W3 once W0 green)
W6 10d alignment discipline (with W2 layouts)
W7 trainer (after W2 + stable forward kernels)
```

**Do not** “convert everything to p64” before W2 boot/CRC fix.  
**Do not** promise 70 tok/s from container swap alone.

---

## 6. Acceptance thresholds (product)

| Gate | Threshold |
|------|-----------|
| Metric honesty | Reported tok/s within 10% of wall-clock |
| Quality | Known-fact prompts correct; open prompts stop on chat-end; no max-token “quiz” as default failure mode |
| Pipeline | Fresh machine: import GGUF → convert → activate p64 → chat, without keeping GGUF required |
| Windows | DX12 passport path stable on 3B Q4_K; Vulkan either fixed or auto-disqualified |
| NVIDIA optional | CUDA path selectable and faster or explicitly “not faster” with numbers |
| Cross-platform | Same passport schema on Windows + Metal; Linux probed, not blocking |
| No regression | smollm2-360m decode not worse than last honest baseline (~19–20 tok/s class on this GPU) after kernel work |

---

## 7. File map (implementation anchors)

| Concern | Path |
|---------|------|
| Decode loop / EOS | `inference/inference_agent.rs` |
| Chat template / vocab | `inference/gguf_sharder.rs` |
| Sampler | `inference/sampler.rs` |
| GEMV / GEMM dispatch | `gguf_bridge/{gemm,init,forward,output}.rs` |
| Resident decode | `gguf_bridge/resident_decode.rs` |
| Subgroup shader | `shaders/coop_gemv_subgroup.wgsl` + `fused_transformer.wgsl` |
| p64 compile / index | `q42/p64_weight.rs` |
| q42 format draft | `docs/manuals/standards/q42-format-internal-draft.md` |
| 10d sections | `container_10d/` |
| Forge CUDA / WMMA | `wgsl_forge/execute/cuda.rs`, `emit/cuda_c.rs`, `emit/coopmat.rs` |
| Backend resolve | `wgsl_forge/backend.rs` |
| Circuit benchmark | `platform/device_benchmark.rs` |
| GPU default backend | `gpu_context.rs` |
| CLI shader/forge | `qualia-cli/src/shader.rs` (extend with `model convert`) |
| Model lifecycle | `qualia-client-core` model_* modules |

---

## 8. Explicit non-goals

- Ollama / llama.cpp HTTP as a backend.
- Deleting the GGUF reader (import stays).
- Claiming tensor cores “done” without tiled GEMM on the decode/prefill call graph.
- Blocking Windows delivery on Linux polish.
- Training full base models in-forge in v1.

---

## 9. First execution slice (when authorized to code)

1. **W0 + W1** — fix token metric + multi-stop decode (immediate human-visible quality).
2. **W2.CLI** — `model convert` + activate p64 for smollm2 (prove designed pipeline once).
3. **W4.1** — passport probe DX12/Vulkan/CPU on this box; persist; default from matrix.
4. **W-K1** — tiled WMMA GEMM in forge; wire one prefill matmul; measure.

Report each step in `INFERENCE_P64_PIPELINE_PROGRESS_LOG.md` with real numbers and ⚑ only for principal decisions.

---

## 10. Summary for the principal

- **Design is right:** convert to p64/q42; don’t run foreign containers as steady-state.
- **Today:** conversion is a byte-preserving library with no product surface; decode is wgpu (DX12), not CUDA; subgroups are on; **tensor-core full GEMM is not on the product path**.
- **Plan:** honest metrics → stop-token/q42 quality → productize convert → kernel/CUDA/tiled GEMM → **auto passport** across Windows / Metal / mobile architecture (Linux secondary) → 10d alignment → LoRA trainer.
- **Novel gains** come from conversion layouts + Qualia governance/context, not from pretending GGUF is already optimized.
