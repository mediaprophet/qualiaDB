# Act IV — Inference

> *The GGUF model is mapped into the process. The GPU runs the fused transformer.
> The Sentinel watches the logits.*

---

## Thesis

> **The language model is not a service. It is not a subprocess. It is a
> memory-mapped file in the same process as the graph engine, running on
> the GPU through `wgpu`, with a second thread that can refuse a token
> mid-generation.**

---

## Voice-over script

### Shot 1 — A black screen. A single line of text: **There is no Ollama.** [SLOW]

> There is no Ollama. [PAUSE]
> There is no llama.cpp. [PAUSE]
> There is no Python runtime. [PAUSE]
> There is no external daemon to query. [PAUSE]

### Shot 2 — A GGUF file is shown being opened. A `memmap2` region appears in the address space. [SLOW]

> The model is a GGUF file on disk. [PAUSE]
> It is mapped into the process with `memmap2`. [PAUSE]
> The OS page cache is the model's memory. [PAUSE]
> No copy. No allocation. No Python. [PAUSE]

### Shot 3 — The model is parsed. A `GgufTensorIndex` appears. [SLOW]

> The header is parsed. The tensor count is read. The tokenizer is read. [PAUSE]
> A tensor index is built — every tensor's byte offset, type, and shape. [PAUSE]
> The byte offsets are encoded into NQuin object fields, with the upper
> four bits set to the modality flag zero-b-one-zero-zero-one — the
> inference pointer flag. [PAUSE]

### Shot 4 — A WGSL shader is loaded: `shaders/fused_tensor_contraction.wgsl`. [SLOW]

> The transformer block runs on the GPU. [PAUSE]
> Sixty-four threads per workgroup. [PAUSE]
> Four thousand, ninety-six fused multiply-accumulates per thread. [PAUSE]
> The shader is `fused_tensor_contraction.wgsl`. [PAUSE]
> The backend is `wgpu`. It runs on DirectML on Windows, on Vulkan on
> Linux, on Metal on macOS, on WebGPU in the browser. [PAUSE]

### Shot 5 — The two-thread architecture is shown: LLM thread on the left, Sentinel thread on the right, with a LogitStream and a ControlStream between them. [SLOW]

> Token generation is not a simple loop. [PAUSE]
> It is a bifurcated compute. [PAUSE]
> The LLM engine thread produces logits and pushes them onto a
> wait-free ring buffer — `LogitStream`. [PAUSE]
> The Webizen Sentinel thread reads the logits in real time. [PAUSE]
> If it detects an anomaly — a zero-point-nine-nine byte signature, an
> anachronism, a forbidden token — it pushes a `DenyRollback` onto
> `ControlStream`. [PAUSE]
> The LLM recalculates. [PAUSE]
> This happens mid-generation, not post-hoc. [PAUSE]

### Shot 6 — A token is sampled. The Sentinel does not object. The token is emitted. [SLOW]

> Most of the time, the Sentinel does not object. [PAUSE]
> The token is sampled. The token is emitted. [PAUSE]
> The graph engine receives the token. [PAUSE]

### Shot 7 — A token is sampled. The Sentinel detects an anomaly. `DenyRollback` is pushed. The token is regenerated. [SLOW]

> Sometimes the Sentinel objects. [PAUSE]
> The token is rolled back. [PAUSE]
> A new token is sampled. [PAUSE]
> The audit log records the rollback, with the reason, with the
> operator's DID, with the timestamp. [PAUSE]

### Shot 8 — The graph engine receives the final token. The output is validated. [SLOW]

> The output is then validated. [PAUSE]
> It must have at least one provenance NQuin citation. [PAUSE]
> An ungrounded output is rejected. [PAUSE]
> The model is never invoked without a pre-flight check. [PAUSE]
> The pre-flight check is `validate_intent`. It reads the N3Logic
> Rights Ontology. If it returns `Deny`, the model is never invoked. [PAUSE]

### Shot 9 — Title card: **In-process. Bifurcated. Governed.** [SLOW]

> This is the inference engine. [PAUSE]
> It is in the same process as the graph. [PAUSE]
> It is bifurcated. [PAUSE]
> It is governed. [PAUSE]

---

## On-screen notes

- **Shot 1:** Black. White text. The phrase **There is no Ollama** is centered, in monospace.
- **Shot 2:** A real `memmap2` call. The address space is shown as a horizontal bar. The GGUF file's bytes are visible inside the bar.
- **Shot 3:** `GgufTensorIndex` is shown as a table. Each row is a tensor. The columns are: name, offset, type, shape.
- **Shot 4:** The WGSL shader source is shown on the left; the GPU dispatch is shown on the right. The 64-thread workgroup is highlighted.
- **Shot 5:** A two-column diagram. Left: LLM engine thread. Right: Sentinel thread. Between them: two wait-free SPSC ring buffers (`rtrb`). The labels are real.
- **Shot 6:** A token being sampled. The arrow is green. The Sentinel does nothing.
- **Shot 7:** A token being sampled. The arrow turns red. `DenyRollback` is pushed. The arrow loops back. A new token is sampled. The arrow is green.
- **Shot 8:** The output box. A provenance NQuin is attached. The arrow into the graph engine is green.
- **Shot 9:** Title card.

---

## Source code anchors

- `crates/qualia-core-db/src/inference/gguf_sharder.rs` — `GgufTensorIndex`, `GgufTokenizer`, `dequantize_token_embedding_into`.
- `crates/qualia-core-db/src/inference/inference_agent.rs` — `AgentBackend::Local`, `AgentRuntime`, `validate_intent`.
- `crates/qualia-core-db/src/llm_agent.rs` — `infer_local_model`, the Phase 8 bifurcated loop.
- `crates/qualia-core-db/src/shaders/fused_tensor_contraction.wgsl` — the actual WGSL.
- `crates/qualia-core-db/src/inference/directml_bridge.rs` — `DmlDevice`, `dequantize_q4_k_block`.
- `crates/qualia-core-db/src/inference/inference_kernel_parity.rs` — `quantize_q8_0_from_f32`, `quantize_q4_k_from_f32`, `max_ulp_diff`.
- `crates/qualia-core-db/src/inference/inference_eval.rs` — `perplexity`, `kl_divergence`, `QualityVerdict`.
- `crates/qualia-core-db/src/inference/compute_universe.rs` — `Phase8Channel`, `ContextInjectRing`.
- `crates/qualia-core-db/src/q42/p64_weight.rs` — `compile_gguf_to_p64`, `P64TensorIndex::validate_against_gguf`.
- `crates/qualia-core-db/src/wgsl_forge/` — typed IR, deterministic emission, Naga validation, CPU/GPU differential checking.
- `AGENTS.md §2-B` — *real autoregressive loop, no longer mocked* (Codex, 2026-06-06).

---

## Duration

Approximately 150 seconds. This is the act where the engine speaks.
