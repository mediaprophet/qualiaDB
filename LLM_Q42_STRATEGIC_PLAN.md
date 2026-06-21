# QualiaDB LLM / `.q42` — Strategic Plan & Honest Status

**Date:** 2026-06-21 · **Owner:** Timothy Charles Holborn (inventor/curator)
**Purpose:** A single, advisor-reviewable document that (1) states the *honest* current status of the
LLM stack — separating what genuinely works from what was claimed — and (2) defines the target
architecture and a staged plan so we can decide *what we are building toward* before spending more
engineering effort.

**Companion docs (existing):** [`STELLAR_MISSION.md`](STELLAR_MISSION.md) (mission → stellar roadmap),
[`WASM_LLM_ROADMAP.md`](WASM_LLM_ROADMAP.md), [`WASM_LLM_ENDGAME.md`](WASM_LLM_ENDGAME.md),
[`WASM_LLM_INFERENCE_DIAGNOSIS.md`](WASM_LLM_INFERENCE_DIAGNOSIS.md),
[`advanced-q42-llm-ideas.md`](advanced-q42-llm-ideas.md),
[`qualia-llm-future-updates.md`](qualia-llm-future-updates.md).

> **How this doc differs from the companions:** the companion docs are *progress logs* written from
> inside the work, and they mark milestones "✅ DONE" the moment a headless test passed. This document
> re-verifies those claims against the surfaces a *real user* touches (the deployed GitHub Pages site
> and the native CLI), and is deliberately conservative. Where the two disagree, the evidence in
> §A and the Appendix is authoritative.

---

## 0. TL;DR for advisors

- The core inference engine is **real and original** — native Rust → GGUF → WebGPU/WASM, no
  Ollama/llama.cpp/Python. On the **one** path it was tuned for (a locally-served SmolLM2-360M, driven
  by headless test scripts) it produces coherent output at ~5–6 tok/s. That is a genuine achievement.
- **But it does not currently work for a real visitor to the public site.** Every public LLM demo page
  fails at "Load model" because the model files are not hosted and the HuggingFace fallback URLs are
  wrong (HTTP 404). The "verified working" runs always served the model from `localhost`. This is the
  gap between the optimistic milestone logs and Timothy's lived experience that "it still doesn't work."
- The headline benchmark page **overclaims**: `benchmark.html` advertises "QualiaDB native WASM+WebGPU
  LLM, .q42 AOT, ~5.9 tok/s," while the harness it embeds states verbatim *"No Qualia engine wiring is
  done here yet."*
- The **native (non-WASM) path compiles and loads but its runtime inference is BROKEN** — a live run
  this session produced incoherent `<|endoftext|>` spam at ~24 min/prompt (TPS 0.00) on a SmolLM2-360M
  GGUF via DirectML/Vulkan (§A.3). The WASM optimisations *are* correctly `cfg`-isolated (so they didn't
  cause it by leaking into native), but native generation is non-functional today. Timothy's "the
  original non-WASM handling might be incompatible" concern is **confirmed at runtime** — it is now a
  V1-blocking defect, not a verification chore.
- The **advanced ideas** (`advanced-q42-llm-ideas.md`, STELLAR §2) split cleanly into *sound-but-unbuilt
  performance work* (ternary/KIVI/W4A4/speculative-decode) and *aspirational research-grade vision*
  (10D→5D multimodal physics, PGA/CAD, EMF sensing). None of it should start until V1 actually works on
  a real surface.
- **The decision this doc asks advisors to weigh in on (§E):** what is the *target* — a hardened,
  honestly-scoped V1 that real people can use, or a leap toward the full-scope "Large Physics Model"
  vision — and in what order, given a one-person team under resource constraint.

---

## 1. What the engine actually is (so reviewers share a mental model)

QualiaDB has its **own** in-process inference stack. There is no external model server.

| Layer | File | Role |
|------|------|------|
| GGUF parse + pointer map | `gguf_sharder.rs`, `gguf_parser.rs`, `gguf_tensor_index.rs` | parse header, build tensor index |
| Quant dequant | `ggml_quants.rs` | block layouts for the supported GGML types (§C.1) |
| GPU bridge | `gguf_bridge.rs` (~8.3k LoC) | maps weights, dispatches fused transformer blocks via `wgpu` |
| Shaders | `shaders/fused_transformer.wgsl`, `fused_attention.wgsl` | WGSL compute kernels (DirectML/Vulkan/Metal/WebGPU) |
| Agent orchestration | `llm_agent.rs` | `LocalLlmAgent`, native vs WASM split, decode loop |
| Governance gate | `orchestrator.rs`, `webizen.rs` | mandatory intent pre-flight + provenance post-flight |
| `.q42` weight container | `q42_weight.rs` | AOT GGUF → page-aligned zero-copy container + tokenizer + CRC |

**Two execution realities that matter for planning:**

1. **Native (host) path** uses synchronous GPU readback (`tokio block_on` on the mapped staging buffer).
   This is the path the desktop/CLI uses. It is `#[cfg(not(target_arch = "wasm32"))]`.
2. **WASM (browser) path** *cannot* read GPU buffers synchronously (WebGPU map is async-only), so on
   wasm **every GEMM result falls back to a CPU compute kernel** (`stack_gemm_quant`). The browser path
   still *encodes* GPU work but the math that actually returns is CPU. This is why browser throughput is
   modest and why "WASM scope ≠ full-scope performance" — Timothy's intuition here is **correct and
   architecturally load-bearing** (see §B).

---

## 2. The advanced-ideas material — sorted into built / sound-unbuilt / aspirational

`advanced-q42-llm-ideas.md` and `qualia-llm-future-updates.md` are wide-ranging. For planning we must
not treat them as one undifferentiated backlog. Sorted by evidential status:

### Built & in-tree today
- AOT **GGUF → `.q42`** weight container (`Q42W` magic), 16 KB page-aligned, tokenizer section, CRC
  integrity, dual-format boot gate (`q42_weight.rs`). **Proven natively** (byte-parity tests) and in the
  headless browser harness.
- OPFS model caching / `loadOrCompileQ42` (compile once, warm-boot zero-parse).
- Resident weights / decode super-arena / parallel Q/K/V projection (the 0.6 → ~5.9 tok/s win).

### Sound but UNBUILT (the real `.q42` v2 performance roadmap — STELLAR §2A)
- **Ternary / BitNet b1.58 FFN packing** — needs a *quantisation-aware-trained* model, not a
  post-training snap of an existing GGUF. Big TPS/energy lever; non-trivial.
- **KIVI asymmetric KV-cache** (2-bit keys / 4-bit values) → 100k-token context. Self-contained, high
  value, buildable on the current kernel.
- **W4A4 + activation-aware (AWQ) "concentration-alignment"** — calibration pass at ingest, scale factors
  in the header. Q8-equivalent math at 4-bit speed. Needs a calibration corpus.
- **Speculative decoding** (mmap a ~100M draft + the target; verify 4–5 tokens/pass). Leans on the
  existing zero-copy design.

  > ⚠️ **Important honesty note:** today the `.q42` compiler stores tensors as **opaque, byte-identical
  > copies of the source GGUF blobs** (same `ggml_type`, just page-aligned and re-laid-out). It does
  > **not** yet compress, re-quantise, or down-sample anything. So "ingest Q8 → get a smaller/faster
  > `.q42`" is **not true yet** — that is exactly what the §2A work would build. See §C.

### Aspirational / research-grade (STELLAR §2C–G; sourced largely from exploratory LLM conversations)
- **10D volumetric tensor folded into a bifurcated 5D NQuin** (Manifold-Coordinate = t∆ / deontic / momentum).
- **Multimodal as physics** — acoustic STFT/CQT manifolds, spectral tensors over RGB, full-EMF (Wi-Fi CSI,
  NIR, ultrasonic), Eulerian Video Magnification "motion microscope."
- **3D / PGA / CAD / photogrammetry** as constraint-satisfying geometry.
- **Heterogeneous CPU+GPU+NPU** (WebNN) routing; **federated LoRA-CRDT** energy-opportunistic training.

These are coherent *in principle* and align with the mission, but they are **unbuilt**, some cite
mid-2026 research that must be verified before relying on it, and they presuppose a working, performant
V1 substrate. They belong in the plan as a **designed-now / built-later** horizon (STELLAR already says
this) — not as current work.

---

## 3. Honest status — verified this session (2026-06-21)

### A.1 — Build & code health
- `cargo check -p qualia-core-db --lib` → **compiles clean** (0 errors; ~622 dead-code warnings). The
  native LLM path is **not** broken at the type level.
- The aggressive WASM optimisations are **`cfg`-isolated**: the Phase 5.5 Q/K/V projection rewrite is
  gated behind a runtime flag (`proj_row_stride`; default `0` = legacy in-shader projection, only the
  WASM `mc8` decode path overrides it). Native and WASM split cleanly inside
  `llm_agent.rs::infer_local_model_inner`. **Conclusion: native was not silently broken by the WASM
  surgery.**

### A.2 — Live deployment audit (real Chrome, mediaprophet.github.io/qualiaDB, v0.0.18)
| Page | Engine init | Model load | Verdict |
|------|-------------|-----------|---------|
| `benchmark.html#llm` | iframe shows **WebGPU ready** | embeds `benchmarks.html` | **Overclaim** — headline says "native 5.9 tok/s"; embedded harness says *"No Qualia engine wiring is done here yet."* |
| `llmdemo/index.html` | WebGPU adapter + Phase 5 WASM load OK | **404** on `/models/...gguf` AND **404** on the HF fallback (`HuggingFaceTB/...`) | **Dead on arrival** — cannot load a model |
| `online-llm-demo.html` | engine "v0.0.18 ready" | default remote model **404**; has a *local-file* upload fallback | Works **only** if the user manually supplies a GGUF file |
| `design-studio.html` | loads the **500 KB slim portal wasm with NO LLM exports** | n/a | No in-browser inference; explicitly *"Next sprint: inference handover."* Asset-recommendation/tensor-bake demo only |

**Root deployment defect (single biggest "it doesn't work"):** the GGUF model is gitignored and never
hosted; the HuggingFace fallback URLs are inconsistent and mostly wrong:
- `llmdemo` + `wasm-llm-benchmarks.js` → `huggingface.co/HuggingFaceTB/SmolLM2-360M-Instruct-GGUF/...` → **404**
- `online-llm-demo.html` → `huggingface.co/bartowski/SmolLM2-360M-Instruct-GGUF/...` → different URL, untested
- Result: the "verified working" milestone runs all relied on a model served from `localhost`, which
  does not exist in production.

**Three divergent WASM bundles are deployed (no single source of truth):**
| Bundle | Size | LLM exports? |
|--------|------|--------------|
| `docs/playground/qualia_core_db_bg.wasm` | 2.8 MB | ✅ full LLM + `.q42` (the only complete one) |
| `docs/pkg/qualia/qualia_bg.wasm` | 500 KB | ❌ none (slim portal) |
| `webizen-desktop/static/portal/pkg/qualia/qualia_wasm_bg.wasm` | 636 KB, **Jun 17** | ❌ has `initialize_webgpu_engine` only — **no `inferWasmStreaming`, no `compileGgufToQ42`** (stale + incomplete) |

The desktop **portal** — the actual product shell, and where the uncommitted `menu.json` now adds a
"Browser LLM" link — ships the stale Jun-17 bundle that *cannot* infer. If Timothy tested there, failure
was guaranteed.

### A.3 — Native inference (CLI), live run this session — **⚠ NATIVE IS RUNTIME-BROKEN**
- Command (after fixing a pre-existing clap `verbose` collision bug — see Defects): `qualia-cli llm
  comprehensive-test --vault-path docs/models SmolLM2-360M-Instruct-Q4_K_M`.
- **Load: OK.** DirectML on NVIDIA RTX A2000 12 GB (Vulkan adapter), GGUF mmapped, 32 layers / 15 heads,
  80 MiB KV cache, model active in **1.95 s**. So native *loading* works.
- **Generation: BROKEN.** Prompt "What is the capital of France?" → **TTFT ~142 s, total ~24 minutes,
  1 effective token, TPS 0.00**, output = `<|endoftext|><|endoftext|>…` (degenerate EOS spam,
  incoherent). The run was killed during the 2nd prompt.
- **Correction to an earlier provisional read:** native is *not* "intact, just unverified." It compiles
  and the WASM optimisations are cfg-isolated (still true), **but its runtime inference is broken** —
  catastrophically slow (~24 min/prompt) and incoherent. Timothy's "the original non-WASM handling
  might be incompatible" concern is **substantively confirmed at runtime.** Likely suspects: the native
  sync GPU-readback path; a wgpu(Vulkan)-vs-DirectML routing conflict (both initialise in the log); or
  the shared Phase-5 shader/projection changes interacting badly with the native dispatch. **This is now
  a V1-blocking defect, not a verification chore** (see Phase 1, re-scoped).

---

## B. The core architectural question: "limited WASM scope" vs "full scope"

Timothy's framing is correct and is the most important strategic point in this document:

> *the wasm build uses a limited scope, that isn't expected to have the performance the full scope is
> intended to deliver.*

**Why this is true, concretely:**
- **WASM is CPU-bound for the math** (§1.2): WebGPU's async-only buffer map forces a CPU GEMM fallback
  on every layer. The browser path therefore exercises the *correctness* of the manifold but caps
  *throughput*. ~5–6 tok/s on a 360M model is near the ceiling of that design without WebGPU
  compute-readback maturity.
- **Native is GPU-bound and uncapped** — it does true synchronous GPU readback and can scale to larger
  models and faster decode on the same shaders.
- **The full-scope vision (NPU via WebNN, heterogeneous routing, ternary kernels, larger context)
  targets *native/edge silicon first*, with the browser as the constrained, zero-install lane.**

**Implication for "what target are we heading toward?":** there are really **two products** sharing one
engine, and we should name them explicitly so effort and benchmarks are honest about which one they
serve:

1. **Lane A — Browser/Zero-Install Demo (WASM):** reach + trust + "runs on your hardware, nothing
   leaves the device." Modest perf by design. Honesty about scope is the feature. Good for SmolLM2-360M
   to ~1B.
2. **Lane B — Native/Edge Engine (desktop daemon, eventually NPU):** the performance + multimodal +
   physics ambitions live here. This is where ternary/KIVI/W4A4 and the §2 horizon actually pay off.

A single "5.9 tok/s" number presented without saying *which lane* is what produced the overclaim. Every
benchmark from here should be labelled Lane A or Lane B.

---

## C. Model ingestion strategy — which GGUF/safetensor to transpile, and the outcomes

This section answers Timothy's direct question.

### C.1 — What the engine can ingest **today** (verified in `ggml_quants.rs`)
**Supported GGML types:** `F32`, `F16`, `Q4_0`, `Q5_0`, `Q8_0`, `Q4_K`, `Q6_K`.
**NOT supported (compiler errors `unsupported tensor type`):** `Q5_K`, `Q2_K`, `Q3_K`, `Q4_1`, `Q5_1`,
`Q8_1`, and **all `IQ*` imatrix quants**.
**Safetensors:** **no ingest path exists** — `.q42` compilation is **GGUF-only** today. Safetensors
support would be new work (parse + tensor-name mapping).

> Practical gotcha already observed: SmolLM2-360M has `n_embd = 960`, not divisible by 256, so its
> "Q4_K_M" GGUF actually stores several tensors as `Q5_0`/`Q6_K`/`Q8_0` fallbacks. All are supported —
> but it means a model labelled "Q4_K_M" is not uniformly Q4_K. Q5_K_M-labelled models are the trap:
> they will contain `Q5_K` tensors the engine **cannot** load.

### C.2 — What `.q42` does with them today (the honest mechanism)
The compiler currently stores **opaque, byte-identical** copies of the source tensor blobs, page-aligned
(16 KB) and re-laid-out for zero-copy GPU binding, plus the tokenizer and CRC integrity. **It does not
re-quantise or compress.** Therefore:
- Ingesting a **Q4_K_M** GGUF → a Q4_K_M-fidelity `.q42` (≈ same size, ~258 MB for 360M).
- Ingesting a **Q8_0** GGUF → a Q8_0-fidelity `.q42` (larger, ~386 MB), higher precision.
- The win from `.q42` today is **boot speed + zero-parse + zero-copy binding**, *not* size or TPS.

### C.3 — Recommended policy (today vs after §2A is built)
- **Today:** ingest **Q4_K_M** for the browser lane (smallest supported footprint that stays coherent on
  small models) and **Q8_0** where fidelity matters (native lane, or as the high-fidelity source for the
  future down-sampling pipeline). **Avoid Q5_K/Q3_K/Q2_K/IQ** — unsupported. Prefer reputable GGUF repos
  with the exact filenames we host (see deployment fix in §D Phase 0).
- **After §2A (the down-sampling compiler):** the *correct* long-term strategy from
  `advanced-q42-llm-ideas.md` becomes real — **ingest the highest-fidelity source you can (Q8_0 or
  F16/safetensors)** once, and let the `.q42` compiler down-sample pathways itself (ternary FFN +
  Q4-attention + W4A4 calibration). At that point the source-quant question inverts: feed it the *most*
  precise weights, not the smallest. **Outcome target:** Q8-equivalent quality at 4-bit speed, 3–6×
  TPS, 100k context — *but only once the compression kernels exist.*

### C.4 — Distribution (resolves the §A.2 404s and aligns with STELLAR §I)
Pre-compile `.q42` (or at minimum host the GGUF) on a **stable, correctly-named** location — HuggingFace
repo and/or WebTorrent swarm — and point **one** canonical URL list at it across all pages. This both
fixes the dead demos and is the intended end-state ("end-user TTFT ≈ 0").

---

## D. Staged plan (each stage has a Definition-of-Done gate; honesty tags in brackets)

> Sequencing principle (Timothy's standing direction, echoed in STELLAR §3): **make V1 real on a surface
> people can actually use before advancing capability.** Design the advanced layers now so the
> foundation isn't gutted later; build them after First Light is robust.

### Phase 0 — Make the public demo actually work for a stranger *(small, highest leverage)*
1. Host the model(s) at a correct, stable URL; unify **one** model-URL list across `llmdemo`,
   `online-llm-demo`, `wasm-llm-benchmarks.js`, `api-explorer`. Fix the `HuggingFaceTB` 404.
2. Stop the overclaim: make `benchmark.html` either *actually* wire the Qualia adapter or change the
   headline to match the embedded harness's honest "not wired yet."
3. Decide the canonical browser bundle and ensure every LLM page loads it (kill the 3-bundle drift).
- **Gate:** a first-time visitor on stock Chrome can open one public URL, click load, and get coherent
  tokens — with **no localhost and no manual file upload.**

### Phase 1 — Fix native runtime + label both lanes *(re-scoped: native is broken, not merely unverified)*
1. **Native lane is a V1 blocker:** generation is incoherent (`<|endoftext|>` spam) at ~24 min/prompt
   (§A.3). Root-cause the native decode: (a) the sync GPU-readback path, (b) the wgpu-Vulkan vs DirectML
   dual-init routing conflict seen in the load log, (c) shared Phase-5 shader/projection interaction with
   native dispatch. Then add a committed CLI/integration test asserting coherent output + a sane
   tok/s floor, run in CI. (clap `verbose` defect already fixed this session.)
2. Browser lane: real-browser (not only headless) verification matrix — SmolLM2-360M across Chrome
   stable, behind the wgpu device-limit (see Phase 2).
3. Relabel every benchmark as **Lane A (WASM)** or **Lane B (native)**; retire single unlabelled tok/s
   numbers.
- **Gate:** a status table that an advisor can trust, with no surface marked "done" that a user can't reach.

### Phase 2 — Close the remaining V1 defects *(STELLAR §1)*
1. **wgpu 0.19.4 → 0.20+** to remove the `maxInterStageShaderComponents` device-limit rejection that
   breaks real Chrome (currently masked by a JS shim). Full GPU regression on native + WASM. *[real fix;
   medium effort; touches shared shaders]*
2. **1B+ prefill crash** (`dispatch_prefill_chunk` → invalid bind group) — unblock models beyond 360M.
3. `.q42`/ingest full-text truncation; OCR for image-only corpora (tesseract) — corpus completeness.
- **Gate:** ≥1 model >360M runs in-browser on stock Chrome with no shim.

### Phase 3 — `.q42` v2 performance *(STELLAR §2A — the real TPS/context jump)*
KIVI KV-cache → speculative decode → W4A4/AWQ calibration → ternary FFN (needs QAT model). Build the
**down-sampling compiler** so §C.3's "ingest high-fidelity, emit fast" becomes true.
- **Gate:** Lane B 3–6× TPS and/or 100k context on a named model, coherence held, governance gate intact.

### Phase 4 — Neuro-symbolic binding *(STELLAR §2B — the true differentiator)*
Tokenizer → ontology CBOR-LD header (the `.q42` "cold" section already reserved); deontic/ODRL token
masking in-shader. This is what no Big-Tech stack has.
- **Gate:** a deontic boundary demonstrably zeroes forbidden token IDs at the kernel, with provenance.

### Phase 5+ — File-format v2 (10D→5D) → multimodal-as-physics → 3D/PGA → heterogeneous compute → federated *(STELLAR §2C–H)*
The "Large Physics Model" horizon. **Designed-now, built-later.** Each item gets its own DoD when reached;
verify the cited research before relying on it (fiction/non-fiction discipline applied to our own roadmap).

---

## E. Decisions for advisors (and for Timothy's final call)

1. **Target identity.** Do we commit to the **two-lane** framing (A: honest zero-install browser demo;
   B: native/edge performance + physics engine), and benchmark/communicate accordingly? Or pick one lane
   as primary for now?
2. **Sequencing.** Confirm **V1-first** (Phases 0–2) before any §2 capability work — or accept the risk
   of advancing capability on an unverified base?
3. **wgpu upgrade.** Do the real 0.20+ upgrade now (removes the shim, unblocks real Chrome) or keep the
   shim as a stopgap and defer?
4. **Ingestion scope.** Stay **GGUF-only** for now, or invest in a **safetensors** ingest path to enable
   the high-fidelity-source → down-sample strategy (§C.3) sooner?
5. **Distribution.** HuggingFace hosting, WebTorrent swarm, or both, for the canonical `.q42`/GGUF?

---

## Appendix — evidence log & defects

### Live commands / results (2026-06-21)
- `cargo check -p qualia-core-db --lib` → 0 errors (native compiles).
- Live Chrome audit of 4 public pages → §A.2 table (404 model loads; benchmark overclaim; design-studio
  has no LLM bundle).
- Supported quant set read from `ggml_quants.rs::ggml_block_layout` / `ggml_row_bytes` → §C.1.
- Native CLI inference run → **_[result to be appended]_**.

### Defects discovered this session
1. **Deployment 404s** — models not hosted; HF fallback URL wrong (`HuggingFaceTB`). *Blocks all public demos.*
2. **`benchmark.html` overclaim** — headline contradicts the embedded harness's "no Qualia wiring yet."
3. **Bundle drift** — 3 deployed WASM bundles; portal ships a stale Jun-17 LLM-incomplete build.
4. **CLI clap bug** — global `verbose: u8` collides with subcommand `verbose: bool` → panic on
   `llm test` / `llm comprehensive-test`. (Patched locally with explicit arg `id`s to run the native test.)
5. **wgpu 0.19.4 device-limit** (pre-known, STELLAR §1) — still live; masked by `webgpu-limits-shim.js`.

### Verification posture going forward
Keep the project's Prime Directive #4 (no external LLM libs as yardsticks). Verify via in-tree CPU≡GPU
parity, tokenizer round-trip, and byte-parity tests — plus, newly, a **real-user reachability check** on
every "done" claim (the missing discipline that produced the status gap this document corrects).
