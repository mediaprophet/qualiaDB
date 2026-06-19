# 📋 QUALIA ENGINE — MASTER EXECUTION ROADMAP

**Date:** 2026-06-19 · **Owner:** Qualia (Timothy Charles Holborn, inventor/curator)
**Status:** Phase 4 (AOT Ingestion) secured. Pivoting to throughput & neuro-symbolic integration.
**Companion docs:** [`WASM_LLM_ENDGAME.md`](WASM_LLM_ENDGAME.md) · [`wasm_llm_planning.md`](wasm_llm_planning.md) (per-step log) · [`qualia-llm-future-updates.md`](qualia-llm-future-updates.md) (V2 vision)

---

## ✅ PART 1 — THE SECURED BASELINE (DONE)

| Phase | Result | Commits |
|-------|--------|---------|
| **Phase 2B — TTFT gate** | CLOSED. TTFT ~3957 ms (< 4500 ms). Eliminated 208 MB/token weight re-upload; decode super-arena → 2 submits/layer; eager resident upload at init. | `850ac3b1` (3w–3y), `886ea831` |
| **Phase 3 — OPFS caching** | CLOSED. Stream 250 MB+ models to disk via `pipeTo` (no Cache.put, no V8 blob). Warm hit ~290 ms, 0 network. | `2a606127`, `9a4c8e83` |
| **Phase 4 — `.q42` AOT compiler** | CLOSED. Self-contained 16 KB-page container (weights + hyperparams + tokenizer + CRC integrity). Compile GGUF→`.q42`, cache in OPFS, boot zero-parse from `Q42W`. "Paris" verified strictly from a `.q42`. | `6a00cc2f`, `b232cd88`, `345ceaf2`, `6fecdfba`, `886ea831` |
| **Benchmark wiring** | Qualia is a live engine on `docs/benchmarks.html` (`.q42` AOT / GGUF toggle), reporting load/TTFT/tok-s/heap. | `2d77f7fe` |

Verification posture: native byte-parity + tokenizer round-trip tests (no browser, no external LLM
libs) + headless-Chrome end-to-end. Hot loop stays zero-heap; AOT compile is the cold-path ingest tier.

---

## 🚧 PART 2 — IMMEDIATE BLOCKER: playground WASM refresh

**Task:** refresh the shared `docs/playground/qualia_core_db.*` artifact so `llmdemo/index.html` can use
the OPFS `.q42` AOT pipeline — **without breaking the 6 other pages that import it**
(`science-playground.html`, `scientific-computing.html`, `zero-heap-compliance.html`, `benchmark.html`,
`playground/benchmark.html`, `llmdemo/index.html`).

**Resolved (was "decision needed"):** the playground artifact was built with the **`wasm-playground`**
feature (→ 6 science exports: `clinical_risk`, `geometric_algebra_operation`, `ode_solver`,
`organic_chemistry`, `sequence_alignment`, `thermodynamics_mcmc`) which the `docs/pkg/qualia` build
(`package-qualia-wasm.ps1`) omits. Neither current build is a superset of the other.

**Safe refresh build (true superset — keeps the 6 science exports, adds q42/AOT):**
```bash
RUSTFLAGS="-C target-feature=+simd128 -C link-arg=-zstack-size=8388608 -C link-arg=--max-memory=4294967296" \
  wasm-pack build crates/qualia-core-db --target web --out-dir pkg-playground --release -- \
  --no-default-features --features portal,wasm-llm,wasm-logic,wasm-scientific,wasm-playground
# deploy (NO name rename — playground imports the raw qualia_core_db name):
cp pkg-playground/qualia_core_db.{js,d.ts} docs/playground/
cp pkg-playground/qualia_core_db_bg.wasm{,.d.ts} docs/playground/
```
**Acceptance gate:** the refreshed `docs/playground/qualia_core_db.d.ts` must be an export **superset**
of the pre-refresh one (no science export dropped) AND contain `compileGgufToQ42` + `q42FormatVersion`;
load-check all 7 dependent pages parse/import clean before committing. The old artifact is git-tracked →
revertable. Then port `llmdemo/index.html` to `loadOrCompileQ42` (it keeps its OPFS-GGUF cache until then).

---

## ⚡ PART 3 — DISPATCH FUSION (the throughput gate)

**Problem:** sustained decode ~0.6 tok/s — a GPU **frontend dispatch** bottleneck (~400 tiny `M=1`
dispatches/forward starve the Ampere hardware; invariant to submit count and weight upload, per the
Part-3w/3x profiling). Data movement is already solved.

**Execution:**
1. **Timestamp profiling** — wire `wgpu::QuerySet` timestamps into the decode hot path
   (`encode_attn_ffn_tail_gpu` / `encode_prefill_q_ffn_tail_fused`), bracketing Attention vs FFN vs
   Elementwise to isolate per-block µs on the WebGPU/Ampere backend. (Was Part 3y Phase 2; deferred.)
2. **Kernel fusion** — collapse elementwise ops into their preceding GEMMs:
   - Target 1: RMSNorm fused into the QKV projection.
   - Target 2: SiLU + residual add fused into the gate/up/down GEMM passes.
3. **Gate:** > 2.0 tok/s sustained on SmolLM2-360M, coherence held (`Paris.`).

---

## 🧠 PART 4 — NEURO-SYMBOLIC HORIZON

1. **Chunked zero-copy bind (Phase 4.5)** — replace the cache-hit `getFile().arrayBuffer()` with chunked
   OPFS → WebGPU `STORAGE` buffer mapping; bind `.q42` blobs (already 16 KB-aligned) directly into the
   arenas, skipping the V8 heap and even the resident-arena copy.
2. **CBOR-LD ontology binding** — implement the `.q42` "cold" section (`cold_offset`/`cold_len`, already
   reserved in the header); map tensors to W3C ontologies (FIBO, RadLex, …) via the NQuin
   subject/predicate/object/context scaffold (parsed once at ingest, off the hot loop).
3. **Deontic / ODRL shader taint** — wire the WGSL shaders to read the `NQuin.metadata` bitfield
   (`Q42_META_DEONTIC_TAINT` etc., already defined); a tensor violating an ODRL/SHACL boundary is driven
   to zero in-shader — compliance enforced at the silicon level.

---

## 🌌 PART 5 — MULTI-MODAL & GEOMETRIC STRIDING (V2 HORIZON)

Transition "chatbot" → physics/CAD core. (Background: `qualia-llm-future-updates.md`.)
1. **Physical wave-function ingestion** — extend `.q42` tensor roles to host EMF / acoustic multivectors
   (STFT/CQT), aligned to the existing 48-byte striding. New `role` values + `NQuin.metadata` modality
   flags — the V1 format is already forward-compatible for this (no gutting required).
2. **Projective Geometric Algebra (PGA)** — host multivectors (points/lines/planes/motors) so the engine
   natively reasons over CAD geometry / kinematics / 3D space without flattening physics into tokens.

---

## ▶️ HOW TO PROCEED

Part 2's build command is resolved (above). Recommended order: **refresh playground wasm (superset) →
port `llmdemo` → Part 3 Dispatch Fusion**. Parts 4–5 are the post-throughput horizon.
