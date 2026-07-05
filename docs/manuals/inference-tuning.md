# Inference tuning & runtime toggles

Reference for the native LLM engine's runtime knobs and the pre-push smoke gate. All toggles are
process-global environment variables read once per decode; unless noted, unset = the production
default. Vulkan is the default GPU backend on every platform.

## Backend selection

| Env var | Values | Default | Effect |
|---|---|---|---|
| `QUALIA_WGPU_BACKEND` | `vulkan` \| `dx12` \| `metal` \| `gl` | wgpu auto (Vulkan on this HW) | Pins the wgpu backend. |
| `QUALIA_DXC_PATH` | path to `dxcompiler.dll` | unset → vendored DXC beside the exe, else wgpu `Auto` | Bespoke override for the DX12 DXC compiler. DX12's legacy FXC compiler cannot compile `fused_attention.wgsl` (barrier after a varying-length SDPA loop, error X4026), so DX12 needs DXC. |

**DX12 is turnkey** — `dxcompiler.dll` + `dxil.dll` (v1.8.2502+) are vendored in `vendor/dxc/` and
`build.rs` copies them beside the built binaries, so `gpu_context` finds them and DX12 uses DXC with
no configuration. Just pin the backend (verified: coherent decode, Vulkan-parity ~18.5 tok/s, A2000):
```powershell
$env:QUALIA_WGPU_BACKEND='dx12'   # QUALIA_DXC_PATH only needed for a non-vendored DXC location
```
DXC compiler resolution order: `QUALIA_DXC_PATH` → `dxcompiler.dll` beside the executable (vendored) →
wgpu `Auto` (static-DXC → PATH-DXC → FXC). `dxil.dll` must sit alongside `dxcompiler.dll` (DXIL signing).

## Decode fast-path toggles (all native, default ON)

| Env var | Default | Effect |
|---|---|---|
| `QUALIA_LLM_RESIDENT_DECODE` | on | W1: whole token (32 layers + output norm + logits top-1) in ONE submit / ONE fence, hidden state resident in VRAM. Auto per-model fallback to the legacy per-layer path. |
| `QUALIA_LLM_RESIDENT_WEIGHTS` | on | Each weight uploaded to VRAM once (keyed by GGUF byte offset) and reused every token, vs re-uploading per GEMM. |
| `QUALIA_LLM_FFN_FUSION` | on | Whole pre-norm FFN (gate/up/SiLU·mul/down) in one submit with intermediates kept in VRAM. |
| `QUALIA_LLM_COOP_GEMV` | on | Cooperative one-workgroup-per-row GEMV (coalesced reads + shared-memory reduction) instead of the naive 1-thread/row kernel. |
| `QUALIA_LLM_PREPROJECT_ATTN` | on | K/V projection through the cooperative GEMV, reusing the attention shader only for RoPE + KV-cache writes. |
| `QUALIA_LLM_FUSE_ATTN_O` | on | Q-attention writes to a GPU buffer that o_proj consumes in the same encoder — one fewer round-trip per layer. |
| `QUALIA_LLM_GPU_TOPK` | on | Output projection + GPU block argmax (top-1) with a tiny candidate readback, vs a full-vocab logit readback + CPU argmax. |

## Opt-in / correctness paths (default OFF)

| Env var | Default | Effect |
|---|---|---|
| `QUALIA_LLM_TERNARY_FFN` | off | Resident 2-bit GPU ternary-FFN path (only meaningful for a ternary `.q42` container; PTQ quality is D20-gated). |
| `QUALIA_LLM_CPU_ATTENTION` | off | Route attention through the CPU SDPA reference instead of the GPU shader (correctness cross-check; slower). |

## Sampling (not an env var)

Decode is greedy argmax by default. Exact seeded sampling (temperature → repetition/frequency/
presence penalties → top-k → top-p → seeded draw) is requested per call:
- **In-process:** `llm_bench::set_sampler_config(Some(SamplerConfig{..}))` (restore with `None`), or
  `llm_bench::decode_sampled_blocking(model, prompt, n, cfg)`.
- **Over MCP:** the `llm_infer` tool takes a `sampler_cbor` field — a hex-encoded CBOR map of
  `SamplerConfig` (decoded via the shared `hex_decode`). `temperature <= 0` or absent ⇒ greedy
  (byte-identical to the pre-sampler path).

## Profiling / debug

| Env var | Effect |
|---|---|
| `QUALIA_LLM_PROFILE_MODEL` | Absolute GGUF path for the `a0_decode_profile` / `w2_gpu_phase_profile` benches. |
| `QUALIA_LLM_PROFILE_DECODE` | Enables the one-shot empty submit→wait fence baseline in the decode loop. |
| `QUALIA_LLM_PROFILE_DECODE_TOKENS` | Overrides the profile's decode-token count (default 16). Raise it to watch waits/token fall as the fixed prefill fence cost amortizes — the structural proof the resident decode loop is ~1 fence/token. |
| `QUALIA_LLM_DEBUG_DECODE` | Per-layer residual-magnitude + top-5 logit diagnostics (first tokens/layers). |

## Path-visibility (W9)

`a0_decode_profile` prints which decode path ran and its counters — `resident single-fence (W1)`
with hit/fallback counts, plus `GPU submit→wait round-trips/token`. Programmatic accessors on
`llm_bench`: `resident_path_counts()`, `output_path_counts()` (top-k vs argmax fallback),
`sampled_token_count()`, `gpu_wait_count()`, each with a `reset_*`.

## Pre-push smoke gate

Fast checks before pushing an inference-lane change (no full release build for the unit half):
```powershell
# pure/unit (fast): sampler chain, prompt-lookup proposer, gguf_bridge unit tests
cargo test -p qualia-core-db --lib sampler prompt_lookup gguf_bridge

# decode correctness (needs the model + a release build): coherence + resident==legacy identity
$env:QUALIA_LLM_PROFILE_MODEL='C:/LLM_Models/GGUF/smollm2-360m-instruct-q8_0.gguf'
cargo test -p qualia-core-db --release --test llm_bench_a0 a1c_q8_gemm_decode_coherent -- --nocapture
cargo test -p qualia-core-db --release --test llm_bench_a0 a1d_resident_decode_matches_legacy_text -- --nocapture --test-threads=1
```
`a1a` (argmax vs top-1) and `a1c` guard decode coherence (#48); `a1d` guards resident==legacy token
identity; `a2a` guards sampler determinism. Keep them green.
```
