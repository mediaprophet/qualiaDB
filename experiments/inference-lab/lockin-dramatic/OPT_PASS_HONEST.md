# Decode optimisation pass — honest outcome (2026-07-10)

## Formats (what the path is)
| Artifact | Magic / role |
|----------|----------------|
| **P64** (`.p64` / `.soa.p64`) | `p64\0` — weight container (tensors, tokenizer, 10D manifold table, CRC-32C). GGUF is AOT-compiled into this. |
| **Q42** (`.q42`) | `Q42\0` — semantic SuperBlock / Quin graph volume. Not the weight path. |
| **D10** | 10-dimensional manifold coordinate **records embedded inside P64** (`ManifoldCoordinate10D`, 64-byte). Not a separate LLM weight format. |

Normative: `docs/manuals/standards/p64-weight-container-standard.md`, `crates/qualia-core-db/src/q42/p64_weight.rs`.

## What was attempted (this session)
1. Barrier-free full-act Q4 SoA (16 KiB LDS) → **regressed** (~6.3 vs ~6.9) — occupancy collapse on A2000.
2. Multi-row GEMV default-ON + parallel reduce + low-LDS block-tile rewrite → **still not faster** (A: 6.36 multirow / B: 6.68 single).
3. Shared **non-exclusive** CoopGemvBGL for SG / residual_sg / multirow / warp — fixes exclusive-pipeline bind mismatch.
4. Residual **subgroup** reduce entry (`coop_gemv_residual_sg`).
5. Dual K+V kept as low-LDS ping-pong.

## Measured (named file only)
- Model: `C:\LLM_Models\P64\llama-3.2-3b-instruct-q4_k_m.soa.p64`
- Mode: `QUALIA_INFERENCE_MODE=cuda`, FFN fusion on, multirow **off** (default)
- Plan: 28 layers, **289 passes/token**, fused_ffn=true
- **DECODE_PROXY tok_s ≈ 6.74** (32 tokens) — same band as pre-pass (~6.6–6.9)
- Same-host Ollama class earlier: **~70 tok/s** on llama3.2:3b-instruct-q4_K_M

## Honest conclusion
**P64-encoded weights alone do not beat Ollama.** The container is fine; the **execution path** is still a WebGPU mega-pass of ~10 dispatches/layer × 28 layers (289 total) of one-workgroup-per-row Q4 matvecs at ~5% of GDDR peak.

Dispatch fusion and SoA layout are real engineering. They are **not** yet a memory-throughput engine competitive with llama.cpp CUDA kernels.

### What would actually be required to surpass Ollama-class on this GPU
1. **CUDA-native full-layer (or full-token) kernel** for `.soa.p64` Q4_K_SoA: one (or few) launches per layer, weights permanently resident, no 289 wgpu dispatches.
2. Fused dequant+GEMV at register/tile level matching llama.cpp `mul_mat_q` class efficiency.
3. Prefill path same treatment (currently ~0.75 prefill tok/s in timeline).
4. Steady-state warm daemon (not cold CLI) for product metrics.

Until (1)–(2) land, claiming “faster than Ollama because P64” would be a lie.

### Defaults after this pass
- Multirow / FFN multirow: **opt-in only** (`QUALIA_LLM_MULTIROW=1`, `QUALIA_LLM_FFN_MR=1`)
- Residual SG + shared BGL: **on** when adapter has subgroups
- Production measure command unchanged (see HONEST_METRICS.md)
