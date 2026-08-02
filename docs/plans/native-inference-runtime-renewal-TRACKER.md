# Native Inference Runtime Renewal — Implementation Tracker

**Programme:** `native-inference-runtime-renewal-2026-07-26.md`
**Comparator audit:** `llama-cpp-vllm-qualia-inference-gap-audit-2026-07-26.md`
**Started:** 2026-07-26
**Current exactly matched 256-step release result:** quality-gated CUDA Q8 graph replay with
tile-32 paged attention, Q8-activation DP4A projections, and resident device embedding lookup
against Ollama's exact `e959...` GGUF blob and raw no-penalty greedy policy at **254.33 tok/s
median** (5 measured runs), **4.69 ms/token p95**, 1,280/1,280 graph launches, zero fallback
and zero measured hot-path allocation/compile/immutable upload. Dynamic H2D is exactly 12
bytes/token. The identically configured warmed five-run Ollama median is **229.89 tok/s**, so
Qualia leads by **10.63%** on this declared RTX A2000 envelope.

**2026-07-29 native regression audit:** the CUDA execution structure and exact-output gates
remain intact after the WASM work. The apparent default-path regression was an operational
selection defect: without `QUALIA_CUDA_Q8_PROFILE`, the exact A2000/SmolLM2 configuration used
the 167 tok/s incumbent schedule. Cold selection now promotes only the certified NVIDIA RTX
A2000 plus exact model shape, while explicit/custom settings remain authoritative and all other
adapters/shapes fail closed. A no-profile release smoke recorded the named profile, 388 live
graph nodes/token, zero fallback/allocation/compile/immutable upload and 12 H2D bytes/token.
Two explicit-profile 256-step checks reached 233.26 and 233.13 tok/s, but do **not** supersede
the certified 254.33 tok/s baseline because Chrome held 11–53% background GPU utilization
during the audit.

## Status vocabulary

| Status | Meaning |
|---|---|
| Not started | No implementation evidence |
| Investigating | Source/runtime evidence being collected |
| Implementing | Code is being changed |
| Verifying | Implementation exists; required gates are incomplete |
| Certified | All listed evidence and gates pass |
| Blocked | Exact external blocker and attempted alternatives are recorded |

## Programme dashboard

| Package | Status | Purpose | Exit evidence |
|---|---|---|---|
| R0 Measurement truth | Verifying | Separate raw decoder from product inference and prove execution | Fixed-step manifests, receipts, five-run median/p95 |
| R1 Prepared runtime | Implementing | Move immutable discovery/compilation/upload out of token work | Zero allocation/immutable H2D/compile in 256 steps |
| R2 Q8 CUDA + graph replay | Implementing | Native fast path for the actual A2000 Q8 comparator | Parity, captured replay, ≤2 host launches/token |
| R3 Paged KV | Implementing | Production block tables, prefix pages and online attention | Dense parity, no steady allocation |
| R4 Other native backends | Not started | Real HLSL/SPIR-V/Metal plans | Matching execution receipts and measured win |
| R5 P64 layouts | Not started | Directly uploadable calibrated device layouts | No activation-wide dequant; layout receipt |
| R6 Tokenizer ingress | Not started | Span-based, indexed and SIMD pretokenization | Token parity; <5% warm TTFT |
| R7 Graph-assisted inference | Not started | Retrieval, prefix reuse and constraints outside token kernel | Tokens avoided and model-only/product timing |
| R8 Serving scheduler | Not started | Continuous batching after latency efficiency | Matched-concurrency vLLM comparison |
| R9 Build/decomposition | Implementing | Cohesive subdirectory libraries and lean builds | File-size/module checks and feature profiles |
| R10 Temp/disk hygiene | Verifying | Bounded RAII scratch and explicit artifact promotion | Cleanup tests and receipt counters |
| R11 Browser/WASM restoration | Implementing | Restore Qualia's accelerated browser path without replacing or demoting native inference | Physical-phone adapter, residency, parity and exact-token receipts |

## Detailed ledger

### R0 — Measurement truth

| ID | Status | Deliverable | Evidence required |
|---|---|---|---|
| R0.1 | Investigating | Inventory current proxy, phase counters and fallback counters | Source map in audit |
| R0.2 | Verifying | `raw_decode` sublibrary and fixed-step runner | Unit tests and live fixed-step integration pass; release campaign pending |
| R0.3 | Not started | Product benchmark retained separately | Existing behaviour parity |
| R0.4 | Verifying | Execution path counters/receipt | Receipt in CLI output; fallback regression tests |
| R0.5 | Verifying | Versioned `BenchmarkManifest` | Schema round-trip and retained live debug manifest pass; release campaign pending |
| R0.6 | Verifying | Five-run median/p95 reporter | Deterministic statistics and two-run integration pass; five-run release evidence pending |
| R0.7 | Certified | Matched Ollama comparator | Exact blob hash, prompt, 256-step policy and five-run evidence retained for both runtimes |
| R0.8 | Verifying | Dispatch/fence/H2D/D2H/lifecycle telemetry | Live debug receipt: 355 dispatches/token, one fence/token, 384 D2H bytes/token |

### R1 — Prepared runtime

| ID | Status | Deliverable | Evidence required |
|---|---|---|---|
| R1.1 | Verifying | `inference/runtime/prepared/` public ABI | API tests pass; backend implementations ongoing |
| R1.2 | Investigating | Inventory wgpu resident plan cold/hot boundary | Source map; allocation profile |
| R1.3 | Certified | Inventory CUDA mega-pass token preparation | Prepared plan owns ranges/keys/kernel warmup; 256-step measured graph replay is zero-allocation |
| R1.4 | Verifying | `CudaDecodePlanState::{Unbuilt,Ready,Ineligible}` | Integrated and compiles; focused transition tests pending |
| R1.5 | Certified | Resident immutable norms/weights/parameters | Layer matrices, token embeddings, LM head, norms and static parameter packs cold-uploaded; only a 12-byte dynamic step pack/token |
| R1.6 | Not started | Authoritative backend ownership before decode | No resident-first shadowing; fallback receipt |
| R1.7 | Certified | Zero-heap `run_decode_step` gate | Warm actual-model autoregressive graph replay passes the strict thread-local allocation counter for 256 consecutive steps |

### R2 — Q8 CUDA and captured execution

| ID | Status | Deliverable | Evidence required |
|---|---|---|---|
| R2.1 | Verifying | Q8 WGSL/llama CUDA kernel gap profile | Nsight Systems whole-graph trace attributes 94.3% of relative kernel time to Q8 projections; privileged hardware-counter breakdown remains externally blocked |
| R2.2 | Verifying | Direct Q8 CUDA GEMV oracle implementation | 17-row/8-block live CUDA differential passes; adversarial campaign pending |
| R2.3 | Verifying | A2000-tuned Q8 CUDA schedule | Barrier-free kernels plus Q8-activation DP4A SwiGLU/QKV/residual/LM-head and device embedding reach 254.33 tok/s on Ollama's exact blob; broader shape/adversarial campaigns remain |
| R2.4 | Certified | Fused RMSNorm/QKV/RoPE/KV append | Actual-model first-layer and 32-layer CPU hidden parity pass |
| R2.5 | Certified | Fused FFN and residual stages | Actual-model first-layer and 32-layer CPU hidden parity pass |
| R2.6 | Certified | Device output norm + top-k/argmax | Actual-model CPU argmax parity; four-byte token readback |
| R2.7 | Certified | Full-model CUDA graph capture/replay | 48/48 steady tokens recorded as graph launches under stable graph hash |
| R2.8 | Certified | Small-Q8 performance gate | Latest schema-v3 exact-blob/policy release result is 254.33 tok/s versus Ollama 229.89 tok/s (+10.63%), clearing both the 90% recovery and +10% excellence gates |
| R2.9 | Certified | Hardware/model-aware cold tuning selection | Exact A2000/SmolLM2 auto-promotion and custom-override tests pass; no-profile release receipt names the certified profile and fails closed elsewhere |

### R3 — Paged KV and prefix reuse

| ID | Status | Deliverable | Evidence required |
|---|---|---|---|
| R3.1 | Certified | Audit existing disconnected `paged_kv.rs` | Confirmed no production consumers; monolith used per-layer allocation and heap-growing APIs |
| R3.2 | Verifying | `runtime/kv/paged/` decomposition | Config/pool/table/COW modules and four focused tests pass |
| R3.3 | Verifying | GPU block table and workspace plan | Prepared CUDA KV owns a resident identity block table; allocation instrumentation pending |
| R3.4 | Certified | GQA online-softmax paged attention | Tile-32 short attention plus numerically certified 4-/8-segment long-context merge consume real block tables without O(context) score buffers |
| R3.5 | Verifying | Prefix reference counts and copy-on-write | Graph prefix store owns page references across update/eviction; bounded scheduler admission/COW/cancellation tests pass; GPU batch executor pending |
| R3.6 | Certified | Long-context certification | Segmented schema-v3 manifests: 1K 200.18 tok/s; 4K 92.87 tok/s with 320 MiB KV, both zero fallback |

### R4–R8 — Expansion after base decoder certification

| ID | Status | Deliverable | Gate |
|---|---|---|---|
| R4.1 | Not started | Real HLSL/DXIL production plan | Executed receipt and incumbent win |
| R4.2 | Not started | Real SPIR-V/Vulkan production plan | Executed receipt and incumbent win |
| R4.3 | Not started | Native Metal production plan | Apple hardware parity/performance |
| R4.4 | Not started | Selective PTX kernels | `ptxas`, parity and ≥10% kernel gain |
| R5.1 | Not started | P64 backend layout descriptors | Byte-exact round trip |
| R5.2 | Not started | Calibrated Q8/Q4/F16 device pages | Layout-specific manifests |
| R6.1 | Verifying | Borrowed-span pretokenizer | Caller-buffered byte spans replace regex captures and per-piece Strings; model-wide token parity campaign pending |
| R6.2 | Implementing | Prebuilt merge/token indexes | Collision-detecting merge fingerprint index is built once; symbol merge storage still allocates and remains to be replaced |
| R6.3 | Verifying | SIMD scanners with scalar oracle | Runtime AVX2 ASCII category scanner and scalar Unicode oracle agree with the former SmolLM regex edge corpus; differential fuzzing pending |
| R7.1 | Verifying | `runtime/graph_assist/` query ABI | Caller-buffered hashed Quin query, policy receipt and four focused tests pass |
| R7.2 | Verifying | Prefix identity and scoped validity | Model/tokenizer/context/revision/fact-bound identity and fixed prefix-page registry tests pass |
| R7.3 | Not started | Graph constraints/automata | Output and performance evaluation |
| R8.1 | Verifying | Ragged request scheduler | Flat POD batch ABI, concatenated block tables, one-call backend contract, atomic identity checks and zero-allocation mock execution pass compile gates; request-indexed CUDA arena pending |
| R8.2 | Not started | Continuous batching | Matched vLLM throughput/p95 matrix |

### R9 — Module decomposition

| ID | Status | Deliverable | Evidence required |
|---|---|---|---|
| R9.1 | Certified | Directory/module policy in renewal plan and AGENTS.md | Policy links plus CUDA Q8/tuning/mega-pass directory conversions |
| R9.2 | Verifying | Create `inference/runtime/` directory skeleton | Default-feature check passes; full API integration ongoing |
| R9.3 | Not started | Split `gguf_bridge/resident_decode.rs` (1,725 lines) | No file >500 lines in new library |
| R9.4 | Implementing | Split `gguf_bridge/forward.rs` (1,528 lines) | Prepared CUDA plan moved to `gguf_bridge/cuda_decode_plan/`; legacy body removal pending |
| R9.5 | Not started | Split `inference_agent/decode.rs` (1,582 lines) | Native/WASM parity |
| R9.6 | Not started | Split benchmark probes by concern | Existing exports preserved |
| R9.7 | Verifying | Lean inference feature profiles | Dependency-free `qualia-inference-kernel` now owns paged-KV contracts, scalar oracle, segment policy and CUDA sources; four tests build/run in 1.42s. Full decoder/benchmark extraction remains |
| R9.8 | Certified | Split `cuda_lane/{mega_pass,q8,tuning}/` libraries | Entry/orchestration, attention, FFN, output, parameters, preparation, plan validation, tuning, kernels and tests are directory modules; every mega-pass file is below 500 lines |

### R10 — Temporary artifacts and disk hygiene

| ID | Status | Deliverable | Evidence required |
|---|---|---|---|
| R10.1 | Implementing | Artifact classes and cleanup policy | Renewal plan section 11 |
| R10.2 | Certified | `runtime/artifacts/` RAII run directory | Success and panic cleanup tests pass |
| R10.3 | Implementing | Byte-budgeted writes and bounded subprocess logs | Write budget test passes; subprocess migration pending |
| R10.4 | Verifying | Explicit validated artifact promotion | Atomic same-parent promotion test passes; SHA policy integration pending |
| R10.5 | Certified | Safe stale-run cleanup | Direct-child prefix, exact marker, symlink and canonical-parent gates; focused deletion test passes |
| R10.6 | Certified | Cleanup counters in execution receipts | Serialized in retained manifests; unknown counters distinguished by coverage mask |
| R10.7 | Implementing | Migrate lab/compiler temporary producers | Removed legacy root `cargo-check-workspace-{before,after}.log`; remaining producers still require migration |

### R11 — Browser/WASM restoration

This package is additive to the accepted native runtime. It must not replace, gate, or silently
demote the certified CUDA profile. wllama remains a lab comparator only and is not a Qualia
runtime dependency. Browser execution remains Qualia's Rust/WASM engine plus backend-specific
Qualia kernels.

Normative recovery review and implementation sequence:
[`wasm-wgpu-mobile-anatomy-review-2026-08-02.md`](../reports/wasm-wgpu-mobile-anatomy-review-2026-08-02.md),
tracked as R11 below.

| ID | Status | Deliverable | Evidence required |
|---|---|---|---|
| R11.1 | Verifying | Source/shipping-artifact synchronization | Browser reports the expected engine version and artifact SHA after every WASM source change |
| R11.2 | Verifying | Resident WebGPU load contract | All layer weights, LM head and norms upload once; any lazy/non-resident fallback fails visibly |
| R11.3 | Verifying | Structured browser capability negotiation | WebGPU, WebGL2, SIMD, worker and shared-memory capabilities are independent receipt fields; compatibility/core adapter attempts and independent LLM/Anatomy selections are covered by the browser contract tests; physical Pixel rerun remains open |
| R11.4 | Implementing | Directory-backed `gguf_bridge/browser/{webgpu,cpu}` boundary | Browser WebGPU top-1 and CPU-WASM packed-kernel concerns are now focused modules; remaining lifecycle/receipt extraction must keep routing-only `mod.rs` files and sub-500-line ownership |
| R11.5 | Verifying | Qualia-owned CPU-WASM contingency | Qualia kernels execute a real full-transformer token without WebGPU; browser model/init/decode now runs in a dedicated module worker with a zero-copy JS transfer and direct packed SIMD128 Q8_0 GEMV. Physical WASM token parity and multi-worker performance receipts remain open. LLM weights, KV cache and inference scratch are explicitly outside the 42 MiB semantic/SLG Sentinel arena. |
| R11.6 | Implementing | Browser TTFT/decode recovery | Compact GPU top-1 and packed CPU Q8_0 SIMD are implemented; exact-token five-run median/p95, stage timings and physical-Pixel promotion gates remain open |
| R11.7 | Verifying | Native non-regression gate | Accepted A2000 CUDA source/profile remains independently selectable and its exact-output/profile tests pass |
| R11.8 | Verifying | LAN physical-phone telemetry | Secure/COI environment, adapter, init stages, TTFT, completion, bounded memory/error events retained under marker-owned lab directory |
| R11.9 | Verifying | Honest Anatomy WebGPU/WebGL2 lifecycle | Rust/WASM WebGPU and WebGL2 paths now require non-zero uploaded geometry and a presented frame before success; both rendered the real XY body locally, including a 412x915 WebGL2 pass; physical Pixel verification remains open |
| R11.10 | Verifying | Versioned browser capability and execution receipts | Stable capability/Anatomy receipts plus `qualia.browser-execution.v1` now declare backend, vocab, compact/full readback bytes, independent LLM memory, Sentinel exclusion and CPU worker/SIMD/Q8 details; exact-token performance fields remain open |
| R11.11 | Verifying | Canonical `.hmc` LAN staging | The LAN server validates QBDL magic, size and SHA-256, stages missing `.hmc` names atomically from retained `.qualia` sources, and refuses startup/QR on failure; local hard-link staging tests pass |

## Evidence log

| Date | IDs | Evidence | Result |
|---|---|---|---|
| 2026-08-02 | R11.6 | Local wllama 3.5.1 source inspection | Confirms llama.cpp/server-context reuse, worker execution, conditional pthread pool, WASM SIMD, GPU-layer offload and versioned GLUE messaging; retained as an external comparator only, not a Qualia dependency |
| 2026-08-02 | R11.2, R11.6 | Local browser, SmolLM2-360M Q8_0 P64 | Coherent `Paris`; 318.8 MB/32 layer weights, 47.8 MB LM head and 64 norm slots resident once; TTFT remains slow and performance is not certified |
| 2026-08-02 | R11.3, R11.8 | Pixel LAN session `secure-phone-20260802-123000` | HTTPS secure context and COI pass; Chrome 150 exposes `navigator.gpu` but returns no adapter before model code |
| 2026-08-02 | R11.3 | `webgpu-adapter-order.test.mjs` | Android compatibility-first negotiation and desktop requested-then-default ordering pass |
| 2026-08-02 | R11.3, R11.10 | `browser-capability.test.mjs`, `online-llm-cpu-fallback.test.mjs` | Stable capability schema, independent WebGPU/WebGL2/CPU-WASM selection, adapter-attempt receipts and honest LLM fallback wiring pass |
| 2026-08-02 | R11.9 | Local browser, real XY Anatomy `.hmc` | WebGPU presents 25 organs / 5,222,191 triangles; forced WebGL2 presents the same body, and the 412x915 phone profile presents 9 organs / 1,615,213 triangles with an acknowledged frame |
| 2026-08-02 | R11.11 | `mobile-wasm-lab-assets.test.py`, session `local-browser-20260802` | Male/female QBDL packs validated and canonical `.hmc` hard links staged with SHA-256 receipts before LAN server startup |
| 2026-08-02 | R11.5 | `wasm_cpu_backend` real SmolLM2-360M Q8_0 test | Qualia-owned CPU transformer token completes without a GPU at a 1,024-token context; measured mutable LLM working set is 80.2 MiB, confirming inference is not constrained by the 42 MiB semantic Sentinel |
| 2026-08-02 | R11.4, R11.6 | `browser-webgpu-top1.test.mjs`; native vocab-scale top-k GPU differential | Browser output projection reduces 49,152 logits on-device and reads 48 `{score, token}` pairs (384 bytes/token) instead of 196,608 bytes; deterministic token parity passes for k=1/32/64 |
| 2026-08-02 | R11.4-R11.6, R11.10 | `qualia-cpu-worker.test.mjs`; `packed_q8_matches_dequantized_rows`; full `wasm-full` release build | CPU model ownership and decode moved off the UI thread; model bytes transfer into the worker; packed Q8_0 GEMV matches the dequantized oracle and compiles with explicit SIMD128; execution receipt declares one dedicated worker and the independent LLM memory domain |
| 2026-07-26 | R0.1, R1.2, R1.3, R2.1 | llama.cpp/vLLM/Qualia audit; live A2000 decode | Qualia 54.90 tok/s; 355 dispatches/token; priority corrected |
| 2026-07-26 | R0.4 | Decode-proxy execution-path counters | Implemented, broader receipt schema still incomplete |
| 2026-07-26 | R9.1, R10.1 | Programme policy added | Implementation gates established |
| 2026-07-26 | R9.2, R10.2–R10.4 | `inference/runtime/{artifacts,prepared,receipt}` | 7 focused tests pass; default-feature CLI check passes |
| 2026-07-26 | R0.2, R0.5, R0.6 | `inference_bench/raw_decode` + `llm raw-decode-bench` | Compiles; fixed-step runner/statistics/receipt implemented; live certification pending |
| 2026-07-26 | R0.2, R0.4–R0.8, R10.4 | Retained raw debug integration manifest | 36.91 tok/s diagnostic; WgpuDx12, zero fallback, 355 dispatches/token, one fence/token, artifact payload accounted |
| 2026-07-26 | R1.1, R1.3–R1.5, R9.4 | `gguf_bridge/cuda_decode_plan/` prepared CUDA lifecycle | Cold tensor discovery, matrix/norm/LM-head upload, KV allocation and kernel compilation; CLI check passes |
| 2026-07-26 | R2.2 | `inference/cuda_lane/q8/` oracle + native CUDA GEMV | Live CUDA differential passes (17 rows, 256 inputs, eight Q8 blocks/row) |
| 2026-07-26 | R2.4â€“R2.6 | Actual SmolLM2 Q8 scalar CPU differential | First layer, all 32 hidden layers, output RMSNorm, 49,152-row LM head and argmax pass |
| 2026-07-26 | R2.4â€“R2.6 | CUDA parameter ABI audit | Fixed undersized QKV (4 vs 5 words) and FFN (3 vs 4 words) buffers; all parameter write failures now fail closed |
| 2026-07-26 | R0.2, R0.4â€“R0.8, R2.8 | `raw-decode-cuda-parity-fixed-2026-07-26` | Debug median 92.36 tok/s, p95 13.07 ms, 195 dispatches/token, one fence/token, four D2H bytes/token, zero fallback |
| 2026-07-26 | R0.7, R2.6 | Ollama `qualia-smol-q8:latest`, raw greedy prompt, logprobs enabled | CUDA first 12 visible tokens match Ollama token segmentation/text and token 13 is EOS; Ollama ~158 tok/s in this comparison |
| 2026-07-26 | R0.7 | `raw-decode-wgpu-parity-fixed-2026-07-26` | WGPU median 36.27 tok/s and divergent greedy continuation; portable INT8-KV path requires separate correctness remediation |

| 2026-07-26 | R0.4, R0.8, R1.5, R2.7, R2.8 | `raw-decode-cuda-graph-receipt-2026-07-26` | Debug median 118.63 tok/s, p95 9.51 ms, 48/48 graph launches, one fence/token, 3,848 H2D and four D2H bytes/token, zero fallback; token output unchanged |
| 2026-07-26 | R3.1-R3.5, R9.2 | `runtime/kv/paged/` and production CUDA block-table bindings | Four fixed-capacity pool/table/prefix-COW tests pass; 32-layer actual-model position-zero parity passes |
| 2026-07-26 | R2.4-R2.8, R3.4 | `raw-decode-cuda-q8-norm-once-2026-07-26` | Paged online-softmax plus once-per-sublayer RMSNorm: 132.97 tok/s median, p95 7.89 ms, exact prior token stream, zero fallback |
| 2026-07-26 | R2.3 | `raw-decode-cuda-q8-four-rows-warp-2026-07-26` | Rejected: four rows/warp regressed to 99.33 tok/s despite parity; implementation reversed |
| 2026-07-26 | R7.1-R7.2 | `runtime/graph_assist/` | Hashed caller-buffer query, policy filtering, scoped prefix identity and fixed page registry; four tests pass |
| 2026-07-26 | R10.5-R10.6 | Marker-owned stale staging cleanup | Exact marker/prefix/canonical-parent cleanup test passes; cleanup counters wired into raw benchmark receipt |
| 2026-07-26 | R0.2, R0.4-R0.8, R2.7-R2.8 | `raw-decode-cuda-renewal-five-run-2026-07-26` | Current accepted source: five-run debug median 133.08 tok/s, p95 7.82 ms, 80/80 graph launches, zero fallback, exact established token stream |
| 2026-07-26 | R3.4 | Paged GQA scalar oracle | Deterministic randomized dense versus reversed-page online attention agrees within 2e-6 at five positions across a page boundary |
| 2026-07-26 | R1.3, R1.7, R2.7 | Strict hot-path allocation gate | Actual-model CUDA graph replay after capture performs zero host heap allocations for 256 autoregressive steps; removed per-token `QUALIA_INFERENCE_MODE` string allocation by publishing cold configuration to the atomic mode |
| 2026-07-26 | R3.5, R7.2, R8.1 | `runtime/kv/prefix/` + `runtime/scheduler/` | Registry-owned page references survive active requests and eviction; graph identity admission, COW and cancellation pass; scheduler hot operations pass strict zero-allocation measurement |
| 2026-07-26 | R6.1-R6.3 | `gguf_sharder/tokenizer/pretokenizer/` + cold merge-rank index | Regex-free borrowed spans, scalar Unicode reference, runtime AVX2 ASCII run scanner and collision-detecting O(1) merge-rank lookup; former-regex edge-corpus differential and strict zero-allocation tests pass; retained CUDA prompt SHA unchanged |
| 2026-07-26 | R0.4, R1.7, R6.1 | `raw-decode-tokenizer-span-cuda-2026-07-26` | Cold mode publication fixed after receipt caught unintended WGPU execution; rerun executed CUDA, prompt-token SHA exactly matched the accepted pre-change manifest, text unchanged, 133.77 tok/s one-run smoke |
| 2026-07-26 | R10.7 | Root temp hygiene | Removed the two unscoped legacy `cargo-check-workspace-{before,after}.log` files (24,302 bytes total); retained benchmark manifests remain explicit evidence |
| 2026-07-26 | R2.3, R2.8 | `raw-decode-cuda-q8-no-barriers-2026-07-26` | Accepted: direct cache-resident activation loads remove two block barriers per 32 columns; whole-model CPU parity passes; five-run debug median 175.84 tok/s, p95 6.07 ms, exact token stream, 80/80 graph launches, zero fallback (+32.1% over 133.08) |
| 2026-07-26 | R0.7, R2.8, R3.4 | Initial matched 256-token comparator | The original single Ollama observation (181.36 tok/s) was later superseded by the warmed five-run comparator; Qualia `raw-decode-cuda-q8-matched-ollama-256-2026-07-26` was 88.23 tok/s and exposed the long-context collapse |
| 2026-07-26 | R3.4 | `raw-decode-cuda-warp-sdpa-matched-256-2026-07-26` | Rejected: one-warp paged online attention preserved tokens but regressed matched throughput to 73.80 tok/s; implementation reverted. Next design must tile context in parallel |
| 2026-07-26 | R2.8, R3.4 | `raw-decode-cuda-tiled-sdpa-matched-256-2026-07-26` | Accepted tiled online-softmax design: tile-8 debug reached 158.89 tok/s with exact output, removing the scalar long-context collapse |
| 2026-07-26 | R2.8, R3.4 | `raw-decode-cuda-tiled16-sdpa-matched-256-release-2026-07-26` | Tile-16 release reached 162.07 tok/s, p95 6.92 ms, exact output; retained as the fallback schedule while tile-32 was tested |
| 2026-07-26 | R0.4, R1.7, R2.7-R2.8, R3.4 | `raw-decode-cuda-tiled32-sdpa-matched-256-release-2026-07-26` | Accepted attention baseline: 163.52 tok/s, p95 6.82 ms, 1,280/1,280 graph launches, zero fallback/hot allocation/compile/upload and exact established output |
| 2026-07-26 | R3.4 | `tiled_cuda_matches_scalar_paged_oracle_across_tiles_and_pages` | Live CUDA tile-32 kernel agrees with the scalar paged oracle at positions 0, 1, 15, 16, 31, 32 and 63 using a reversed/non-identity physical-page map |
| 2026-07-26 | R2.1, R3.4 | CUDA event timing fallback | Position-63 tile-32 attention measures 20.48 us median / 30.72 us p95 over 32 event-timed launches; timing API is explicit and fenced outside production replay |
| 2026-07-26 | R2.1 | `nsys-cuda-graph-2026-07-26` | Two-token CUDA graph node trace: Q8 SwiGLU 44.5%, residual GEMV 24.7%, QKV/RoPE 15.4%, LM-head GEMV 9.7% (94.3% combined); attention 1.5%, KV append 0.9%, RMSNorm 2.9%, argmax 0.3%. Absolute durations are trace-inflated, so percentages select work while CUDA events judge candidates |
| 2026-07-26 | R2.1 | Nsight Compute `SpeedOfLight` single-kernel attempt | External blocker: `ERR_NVGPUCTRPERM`; no `.ncu-rep` produced. Retained 386-byte diagnostic log, then used non-privileged Nsight Systems and CUDA events |
| 2026-07-26 | R9.7 | `cargo build -p qualia-cli --release` after tile-32 change | Passed in 5m54s; broad release code-generation cost is now an explicit lean-profile/build-boundary defect |
| 2026-07-26 | R0.7 | `ollama-qualia-smol-q8-matched-256-5run-2026-07-26/summary.json` | One warm-up plus five fixed 256-token greedy runs: 213.83 tok/s median, 4.72 ms/token p95; supersedes the unstable 181.36 single run |
| 2026-07-26 | R0.4, R2.7 | Exact captured-node telemetry | Receipt now stores the node count recorded beside the live graph; removed the stale 8-nodes/layer reconstruction that under-counted opt-in candidates |
| 2026-07-26 | R2.1-R2.3 | Q8 DP4A real-shape CUDA-event campaign | SwiGLU 1.688x, down+residual 1.412x, LM head 1.637x and warp8 QKV/RoPE 1.125x, each including activation quantization and a numerical/argmax gate |
| 2026-07-26 | R2.3, R2.8 | `raw-decode-cuda-dp4a-swiglu-matched-256-release-2026-07-26` | 195.10 tok/s; proved the first Q8-activation candidate but remained below the corrected Ollama median |
| 2026-07-26 | R2.3, R2.8 | `raw-decode-cuda-dp4a-resid-swiglu24-matched-256-release-2026-07-26` | 209.74 tok/s, p95 5.52 ms, exact 347 nodes/token; recovery complete but parity still open |
| 2026-07-26 | R2.3, R2.6-R2.8 | `raw-decode-cuda-dp4a-resid-swiglu24-lmhead-matched-256-release-2026-07-26` | 218.93 tok/s, p95 5.31 ms, exact 348 nodes/token; first corrected-comparator lead (+2.39%) |
| 2026-07-26 | R2.3, R2.6-R2.8 | `raw-decode-cuda-dp4a-resid-swiglu31-lmhead-matched-256-release-2026-07-26` | 228.94 tok/s, p95 5.14 ms, exact 355 nodes/token; 31 layers pass the quality gate while all 32 fail |
| 2026-07-26 | R0.4-R0.8, R1.7, R2.3, R2.6-R2.8 | `raw-decode-cuda-dp4a-qkv-warp8-resid-swiglu31-lmhead-matched-256-release-2026-07-26` | **Accepted:** 256.75 tok/s, p95 4.66 ms, 1,280/1,280 graph launches, exact 387 nodes/token, zero fallback/hot allocation/compile/upload; +20.07% over Ollama 213.83 |
| 2026-07-26 | R2.3 | Compounded quality rejections | All-layer SwiGLU+residual cosine 0.989742 and all-layer SwiGLU+down-only cosine 0.987979 failed the 0.990 gate; neither configuration was promoted |
| 2026-07-26 | R0.4 | `rejected-wgpu-selector-diagnostic-dp4a-resid-swiglu24-2026-07-26` | Receipt rejected a mislabeled run: `QUALIA_LLM_CUDA_DECODE=1` without `QUALIA_INFERENCE_MODE=cuda` executed WGPU-DX12; retained under an explicit rejected name |
| 2026-07-26 | R2.1 | `nsys-cuda-dp4a-accepted-2026-07-26` | Node-level trace after the first DP4A batch redirected work to SwiGLU/residual/QKV; graph-level empty capture was deleted and rerun with `--cuda-graph-trace=node`; Nsight temp stream was removed |
| 2026-07-26 | R9.7 | Release rebuild campaign | Passed at 6m38s, 8m01s and 5m54s while rebuilding unrelated workspace crates; direct evidence for a lean inference benchmark profile/crate boundary |
| 2026-07-26 | R2.3, R9.1, R9.8 | `cuda_lane/{q8,mega_pass,tuning}/` decomposition | Q8 DP4A kernels/tests, named tuning profile, cold preparation and plan types now live in subdirectory modules; full library check and named-profile whole-model test pass |
| 2026-07-26 | R0.4, R1.5, R1.7, R2.7-R2.8 | `raw-decode-cuda-device-embedding-profile-v1-matched-256-release-2026-07-26` | **Accepted:** resident Q8 embedding lookup reaches 256.92 tok/s, p95 4.65 ms, exact 388 nodes/token, 1,280/1,280 graph launches and 12 H2D bytes/token; schema-v2 receipt names `cuda-q8-a2000-smollm2-q8-v1`; zero fallback/hot allocation/compile/immutable upload |
| 2026-07-26 | R1.7, R2.3 | Device-embedding correctness/allocation gates | Exact CPU first-token argmax and whole-model cosine 0.99775851 pass; 256 resident token-id graph replays are strict zero-allocation |
| 2026-07-26 | R9.7 | Release rebuild after device embedding | Passed in 9m42s while rebuilding webizen-render, Solid bridge, vision and client-core; strengthens the narrow inference-kernel crate/profile requirement |
| 2026-07-26 | R9.1, R9.8 | Completed CUDA mega-pass directory decomposition | `mod.rs` 462 lines, attention 435, FFN 308, output 281, parameters 166, preparation 141, tests 106, plan validation about 60 and public types 40; clean library check, whole-model cosine 0.99775851/exact argmax, and 256-step strict zero-allocation replay pass |
| 2026-07-26 | R0.7 | Ollama exact-blob raw-greedy quality diagnosis | Default Ollama repeat penalties caused the apparent continuation mismatch; with `top_k=1`, `top_p=1`, `repeat_penalty=1` and `repeat_last_n=0`, Ollama and both Qualia backends produce the same repeated greedy continuation |
| 2026-07-26 | R0.7, R2.8 | Exact-blob/policy five-run comparator | Ollama `e959...` blob: 229.89 tok/s median, 4.39 ms/token p95; Qualia named CUDA profile on the same blob/prompt/policy: 256.01 tok/s, 4.66 ms/token p95, +11.36%, exact 388 nodes/token and 12 H2D bytes/token |
| 2026-07-26 | R0.5 | Benchmark manifest schema v3 | Raw receipts now require an explicit complete decode policy, preventing penalized sampling from being mislabeled as raw greedy comparison |
| 2026-07-26 | R3.6 | `raw-decode-cuda-context-{1k,4k}-profile-v1-release-2026-07-26` | 1K: 103.68 tok/s, 9.71 ms p95; 4K: 33.08 tok/s, 30.32 ms p95 and 320 MiB resident KV. Both prove exact context capacity, 388 nodes/token, 12 H2D bytes/token and zero fallback; 4K exposes the next attention bottleneck |
| 2026-07-26 | R9.7 | Build-boundary campaign | Latest release build passed in 7m47s; a subsequent test-only relink exceeded 10 minutes before executing one metadata test, independently confirming monolithic crate coupling |
| 2026-07-26 | R3.4, R3.6 | Segmented paged attention scalar differential | Live A2000 tile/segment kernels agree with the scalar reversed-page oracle through positions 1023, 2047 and 4095; targeted test passed after a 13m29s link |
| 2026-07-26 | R2.8, R3.6 | `raw-decode-cuda-segmented-{short-256,context-1k,context-4k}-schema3-release-2026-07-26` | Short 254.33 tok/s (-0.65% versus prior exact run); 1K 200.18 tok/s (1.93x prior); 4K 92.87 tok/s (2.81x prior); all schema v3, exact context, zero fallback and 12 H2D bytes/token |
| 2026-07-26 | R0.4 | CUDA graph node telemetry correction | Removed stale tuning-formula reconstruction; `CapturedCudaGraph::node_count` now calls `cuGraphGetNodes`, with a one-kernel regression test |
| 2026-07-26 | R8.1, R9.1 | `runtime/scheduler/{batch,request_table}.rs` | 32-byte item/16-byte output POD ABI, caller-owned flat page tables, exactly-one backend-call receipt, full-batch identity validation before mutation, and bounded files below 500 lines |
| 2026-07-26 | R9.7 | Segmented-attention build campaign | Targeted live test linked in 13m29s; release CLI rebuilt in 10m02s and again pulled vision, render, Solid and client crates |
| 2026-07-26 | R3.2, R3.4, R9.7 | `crates/qualia-inference-kernel/` lean boundary | Dependency-free directory crate now owns paged-KV configuration, scalar GQA oracle, segment policy and tiled/segmented CUDA sources. Four focused tests pass; crate compiled in 1.42s and `qualia-core-db --lib` compatibility check passed |
| 2026-07-29 | R0.4, R2.3, R2.7-R2.9 | Native-first post-WASM audit | Exact output and CUDA graph structure remain intact. Untuned default reproduced at 167.14 tok/s; explicit named profile reached 233.26/233.13 tok/s under material Chrome GPU contention, so current performance regression is inconclusive |
| 2026-07-29 | R2.9 | `raw-decode-native-auto-profile-device-gated-smoke-2026-07-29` | With the profile variable absent, release execution selected `cuda-q8-a2000-smollm2-q8-v1`: CUDA, 388 nodes/token, 12 H2D bytes/token and zero fallback/hot allocation/compile/immutable upload |

## Build-system findings

| Date | Command | Result | Tracking consequence |
|---|---|---|---|
| 2026-07-26 | `cargo test -p qualia-core-db --lib inference::runtime --no-default-features` | Failed after broad compilation: ungated wgpu/Forge/arkworks references remain | R9.7 remains open; `--no-default-features` is not a lean inference-core profile |
| 2026-07-26 | `cargo test -p qualia-core-db --lib inference::runtime` | 7 passed | Runtime artifact/receipt foundation verified |
| 2026-07-26 | `cargo test -p qualia-core-db --lib raw_decode` | 1 passed | Deterministic statistics verified |
| 2026-07-26 | `cargo check -p qualia-cli` | Passed | Raw CLI integration compiles |
| 2026-07-26 | `cargo test -p qualia-core-db --lib q8_whole_model_hidden_matches_cpu_at_position_zero` | Passed | Actual-model CUDA hidden/logit/token differential verified |
| 2026-07-26 | `cargo build -p qualia-cli` | Passed | Corrected prepared CUDA path integrated into CLI |
| 2026-07-29 | `cargo check -p qualia-core-db --lib` | Passed in 28.86s | Hardware/model-aware selection integrates cleanly |
| 2026-07-29 | Two focused Q8 selection tests | 2 passed | Certified target auto-promotes; custom environment is not overwritten |
| 2026-07-29 | `cargo build -p qualia-cli --release` | Passed twice in 7m36s and 7m03s | Release benchmark remains coupled to unrelated render/vision/client crates; R9.7 stays open |

## Current next action

First rerun the exact 256-step five-run Qualia/Ollama comparator after the browser/WASM GPU
workload stops; do not promote the contention-tainted 2026-07-29 samples. Then expand the lean
boundary from kernel/oracle ownership into the prepared decoder and benchmark runner, and run the
exact-blob multi-prompt token/logit campaign. Lower the bounded request table into a
request-indexed CUDA arena and real ragged batch executor before comparing continuous batching
with vLLM at matched concurrency.
