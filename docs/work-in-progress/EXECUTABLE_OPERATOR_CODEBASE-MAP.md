# Executable operators — Q4_K / Q4_K_SOA codebase map

**Packet:** EOS-002  
**Branch:** `0.0.40-inference`  
**HEAD inspected:** `69a310a48` plus uncommitted MoE/FTW/`gguf_bridge` tree (not edited by this packet)  
**Date:** 2026-09-19  
**Scope:** every current Q4_K and Q4_K_SOA dequant, GEMV, resident-decode, and P64 flag path.  
**Not in scope:** NVFP4 MoE (`inference/moe/nvfp4.rs`) — different codec; listed only as a boundary.

Classification:

| Class | Meaning |
|---|---|
| **Live product** | Reached by native or WASM decode/prefill of a Q4_K or Q4_K_SOA checkpoint |
| **Cold conversion** | P64 compile-time layout transform; not a decode kernel |
| **Lab / microbench** | Measurement or A/B; not the sticky product default |
| **Test-only** | Fixtures, parity, ignored disk gates |
| **Advisory** | Capability strings, docs, UI labels |

Line numbers are from this tree on 2026-09-19. Re-check before W1 if those files move.

---

## 1. Three things named Q4_K (do not mix in W1)

| Layout | Type id | Block | Bytes | Where defined | W1? |
|---|---|---|---|---|---|
| **GGML Q4_K (AoS)** | `GGML_TYPE_Q4_K = 12` | 256 weights | **144** | `ggml_quants.rs:15`, `ggml_block_layout` `:69–72` | **Yes — fixture** |
| **Qualia Q4_K_SOA** | `GGML_TYPE_Q4_K_SOA = 112` | 256 weights | **160** | `ggml_quants.rs:20–32` | No — later schedule |
| **DirectML/Metal namesake** | none (not a GGML type) | 32 weights | **20** | `directml_bridge.rs:245–277`, `metal_bridge.rs:23–56` | **No — different codec** |

The DirectML/Metal helper `dequantize_q4_k_block` is `[f16 scale][f16 min][16 nibble bytes]` with `x = q * scale + min`. That is **not** GGML Q4_K (`x = d*sc*q - dmin*m`, 6-bit packed scales, 144 B). Call sites: `gguf_bridge/embedding.rs:192–197` (Windows DirectML opt-in `QUALIA_DIRECTML=1`, `gguf_bridge/init.rs:1068–1084`) and `gguf_bridge/prefill_async.rs:1145–1174`. EOS-013 must copy `ggml_quants::dequant_q4_k`, not `directml_bridge::dequantize_q4_k_block`.

Stock Q4_K superblock (144 B), as decoded by `dequant_q4_k` (`ggml_quants.rs:500–551`):

```
[0..2)   f16 d
[2..4)   f16 dmin
[4..16)  12-byte packed 6-bit sub-scales/mins  (get_scale_min_k4, :490–498)
[16..144) qs nibbles, 128 B
```

Eight sub-blocks of 32. Even sub-block uses low nibble; odd uses high nibble of the same `qs` byte. Dequant:

```
x = d * sc[s] * q - dmin * m[s]
```

`q4k_block_to_soa` (`:265–287`) copies `qs` to `[0..128)` and **pre-expands** the eight `(d·sc, dmin·m)` pairs to f16 at `[128..144)` and `[144..160)`. Same represented weights; different schedule. Swarm D1/D2: Batch A reconstructs the 144-byte form. Do not transcode to SOA in the converter.

Partial rows: `dequant_q4_k` already stops at `n_elems` (`:516–547`). EOS-013 must keep that.

---

## 2. Canonical CPU oracle (W1/W3 comparison target)

This is the function EOS-013 reconstruction and EOS-030 lookup must match for **stock Q4_K**.

| File | Line | Symbol | Class | Notes |
|---|---|---|---|---|
| `crates/qualia-core-db/src/inference/ggml_quants.rs` | 237 | `dequantize_row_into` | Live product | Dispatches type 12 → `dequant_q4_k`, type 112 → `dequant_q4_k_soa` |
| same | 218 | `dequant_matrix_row_into` | Live product | Row slice then `dequantize_row_into` |
| same | 500 | `dequant_q4_k` | Live product | **Authoritative AoS decode** |
| same | 324 | `dequant_q4_k_soa` | Live product | SoA decode (same numeric identity after expand) |
| same | 490 | `get_scale_min_k4` | Live product | 6-bit pack; WGSL `get_scale_min_k4$S` mirrors this |
| same | 265 | `q4k_block_to_soa` | Cold + lab | 144→160 lossless layout |
| same | 292 | `expand_q4k_tensor_to_soa` | Cold conversion | Full tensor expand |
| same | 693 | `quantize_f32_to_q4_k_soa_tensor` | Cold conversion | Requant when source is not Q4_K |
| `crates/qualia-core-db/src/gguf_bridge/mod.rs` | 1085 | `stack_gemm_quant` | Live product | Per-row `dequant_matrix_row_into` then f32 dot. CPU fallback for any quant GEMV, including Q4_K. WASM Q8_0 special-cases; Q4_K uses this loop. |
| `crates/qualia-core-db/src/inference/inference_kernel_parity.rs` | 169–180 | `q4_k_bytes`, `quantize_q4_k_from_f32` | Test-only / lab fixture | Documents the same superblock; simplified quantizer vs ggml search; round-trip tested against `dequant_q4_k` |

**W1 fixture:** synthetic 144-byte superblock (or `quantize_q4_k_from_f32` output) + `dequant_q4_k` as oracle. Do not require a GGUF on disk.

---

## 3. Live product decode / GEMV dispatch

### 3.1 GPU type gates

| File | Line | Symbol | Class | Notes |
|---|---|---|---|---|
| `crates/qualia-core-db/src/gguf_bridge/quant_support.rs` | 9 | `ggml_gpu_quant_supported` | Live product | Native: Q4_K **or** Q4_K_SOA **or** Q6_K only. WASM: wider, includes Q4_K and SOA |
| same | 40 | `ggml_gpu_attention_shader_supported` | Live product | Includes Q4_K and SOA |
| same | 62 | `ggml_gpu_gemm_supported` | Live product | Native GEMM shader types; includes Q4_K and SOA |

### 3.2 Native GEMM (wgpu)

| File | Line | Symbol | Class | Notes |
|---|---|---|---|---|
| `gguf_bridge/gemm.rs` | 165 | `dispatch_gemm_raw_into` | Live product | CUDA SOA GEMV first (`:184–187`); else wgpu fused transformer; else `stack_gemm_quant` (`:433`) |
| `gguf_bridge/gemm.rs` | 81 | `promote_matrix_to_f16_resident` | Live product (opt-in) | Densifies Q4_K/SOA/Q6_K/Q8_0 to F16 when `ffn_f16_enabled` |
| `shaders/fused_transformer.wgsl` | 36–37, 80–83, 267–270, 449 | `dequant_weight` / coop Q4_K | Live product | Stock Q4_K header cache + SOA single-row path |
| `shaders/fused_ffn.wgsl` | 45–47, 95–98, 267–349, 458–538 | fused FFN dequant | Live product | Dual-matrix Q4_K and SOA |
| `shaders/fused_attention.wgsl` | 69–70, 130–133, 153 | `dequant_q4_k_elem` | Live product | Attention Q/K/V/O |
| `shaders/dequant_template.wgsl` | 21–59 | `dequant_q4_k_elem$S` | Live product | Instantiated per FFN role; **same** `get_scale_min_k4` as CPU |
| `shaders/dual_gemv.wgsl` | 1, 29 | Dual Q4_K_SOA GEMV | Live product (SOA only) | K+V pair |

### 3.3 CUDA SOA (not the W1 source layout)

| File | Line | Symbol | Class | Notes |
|---|---|---|---|---|
| `wgsl_forge/emit/cuda_c.rs` | 149–166 | `Q4K_SOA_GEMV_SRC` / `Q4K_SOA_GEMV_ENTRY` | **Live product** | Device kernel source for `try_q4k_soa_gemv` (`cuda_lane/gemv.rs:21`). 160 B SOA superblock. Not Lab-only. |
| `wgsl_forge/emit/cuda_c.rs` | 649–650 | `Q4K_SOA_GEMV_RESID_SRC` | Live product | Residual-add variant |
| `inference/cuda_lane/gemv.rs` | 7–21 | `try_q4k_soa_gemv` and fused FFN/QKV | Live product when `feature = cuda` and weights are type 112 | Dispatches `Q4K_SOA_GEMV_SRC` |
| `inference/cuda_lane/device.rs` | 167–170 | `preload_q4k_soa_weights` | Live product | Device slab fill |
| `inference/cuda_lane/attention.rs` | 39+ | SOA attention | Live product | |
| `inference/cuda_lane/mega_pass/**` | `mod.rs:76`, `output.rs:63`, `ffn_stage.rs:47` | mega-pass | Live product | Comment: all weights must be Q4_K_SOA |
| `inference/cuda_lane_stub.rs` | 17–144 | stubs | Live product (non-cuda builds) | Returns false / 0 |
| `gguf_bridge/ffn.rs` | 234–254 | `try_q4k_soa_ffn_block(_residual)` | Live product | SOA-only fast path |
| `gguf_bridge/attention.rs` | 832+, 1271, 1398 | `try_q4k_soa_qkv` / `gemv` | Live product | SOA-only |
| `gguf_bridge/forward.rs` | 559–668 | all-layer SOA gate for mega-pass | Live product | Fails over if any of Q/K/V/O/gate/up/down is not SOA |
| `gguf_bridge/resident_decode.rs` | 84–95, 1079–1155, 1305, 1653 | fused FFN quant support + SOA preload | Live product | Fused FFN **also** lists stock Q4_K (`:94`) |
| `gguf_bridge/output.rs` | 178, 625 | coop GEMV if SOA | Live product | |
| `gguf_bridge/prefill_arena.rs` | 824–859 | Q4_K and SOA in prefill | Live product | Dual KV if K and V are SOA |
| `gguf_bridge/cuda_decode_plan/build.rs` | 141, 209 | `preload_q4k_soa_weights` | Live product | |

### 3.4 WASM CPU

| File | Line | Symbol | Class | Notes |
|---|---|---|---|---|
| `gguf_bridge/wasm_cpu/forward.rs` | 6, 36 | `stack_gemm_quant` | Live product | Q4_K goes through CPU dequant GEMV when GPU is unused |
| `quant_support.rs` | 11–24 | WASM type allow-list | Live product | Includes Q4_K and SOA |
| `shaders/wasm/fused_transformer.wgsl` | 36–37, 80–83, 267–270, 584 | WASM GPU copy of native transformer dequant | Live product | Loaded from `gguf_bridge/init.rs` (~113, 701, 704) |
| `shaders/wasm/fused_ffn.wgsl` | 45–47, 99–102 | WASM GPU FFN dequant | Live product | Same type-12 / type-112 split |
| `shaders/wasm/dequant_template.wgsl` | (same helpers as native) | WASM GPU template | Live product | Do not treat native WGSL as the only shader copy |

---

## 4. P64 flags and cold SOA conversion

| File | Line | Symbol | Class | Notes |
|---|---|---|---|---|
| `q42/p64_weight/layout.rs` | 34–35 | `P64_FLAG_Q4K_SOA` | Live product (container) | Set when any 2-D matrix is type 112 |
| same | 110–111 | `P64_VIEW_FLAG_Q4_K`, `P64_VIEW_FLAG_SOA` | Live product (container) | Independent view bits |
| `q42/p64_weight/compiler.rs` | 40, 223–256, 322, 457 | `P64ConvertLayout::Q4kSoa` | Cold conversion | Q4_K source → `expand_q4k_tensor_to_soa` (lossless). Non-Q4_K source → dequant f32 → requant SOA (**not** bit-identical). **Off-limits for W1–W2.** |
| `qualia-cli/src/llm_testing.rs` | 299–354, 1891, 1915 | CLI `soa` / `q4k-soa` layout | Live product (tooling) | Selects `P64ConvertLayout::Q4kSoa` |

Swarm D2: W1/W2 must not call `expand_q4k_tensor_to_soa` or the compiler. Read constants only.

---

## 5. Prepared runtime / modes (advisory vs live)

| File | Line | Symbol | Class | Notes |
|---|---|---|---|---|
| `inference/runtime/prepared/capability.rs` | 61, 83 | `supported_quants` | Advisory | Strings `"Q4_K_M"` on CPU and CUDA profiles. Does **not** name type 12 vs 112. W3 identity work should not treat this string as layout. |
| `inference/inference_modes.rs` | 136–142 | CUDA mega-pass vs wgpu | Live product | Mega-pass needs all-Q4_K_SOA; mixed Q4_K_M (Q8_0 V, Q6_K down) falls back |
| `inference/inference_agent/types.rs` | 20 | `quantization: String` | Advisory | e.g. `"Q4_K_M"` |
| `inference/inference_agent/local_agent.rs` | 41 | default `"Q4_K_M"` | Advisory | |
| `qualia-client-core/src/model_lifecycle.rs` | 375 | default `"Q4_K_M"` | Advisory | |
| `inference/lab/audit_path.rs` | 40 | Q4_K_SOA/F16 fallback flag | Lab | |

---

## 6. Lab, Forge PTX, tests

| File | Line | Class | Notes |
|---|---|---|---|
| `inference/lab/micro.rs` | 6–96 | Lab | Builds SOA from stock via `q4k_block_to_soa`; `run_q4k_soa_microbench` → `try_q4k_soa_gemv` |
| `inference/lab/device_roof.rs` | 80–107 | Lab | Same SOA expand for roof probes |
| `wgsl_forge/emit/ptx.rs` | 104–111, 227, 625 | Lab / Forge | `q4k-gemv` and `q4k-soa-wmma` emitters |
| `wgsl_forge/emit/cuda_graph.rs` | 40, 227, 288, 400–408 | Lab / Forge | Graph lowering to `q4k_soa_wmma_gemv` |
| `wgsl_forge/emit/cuda_c_fused.rs` | 21–33 | Live product / Forge | Fused QKV+RoPE on SOA; used with CUDA lane, not merely a lab sketch |
| `wgsl_forge/ir/graph.rs` | 85 | Advisory IR | `DType::Q4K` |
| `qualia-cli/tests/differential.rs` | 266–269 | Test-only | PTX kernel name pairs |
| `inference/cuda_lane/mega_pass/tests.rs` | 5–68 | Test-only | SOA convert + mega-pass |
| `q42/p64_weight/tests.rs` | 26–27 | Test-only | F32 synth has no `P64_FLAG_Q4K_SOA` |
| `q42/p64_weight/legacy_tests.rs` | 220, 317, 386 | Test-only / ignored disk | SmolLM2 Q4_K_M GGUF paths |
| `inference/gguf_sharder/tests.rs` | 7, 255–329 | Test-only / ignored disk | Gemma / SmolLM2 Q4_K_M |
| `tests/llm_bench_a0.rs` | 48–50, 122, 1674+ | Test-only / hardware | SmolLM2 and Llama-3.2-3B Q4_K_M generation |
| `ggml_quants.rs` | 752–994 | Test-only | SOA round-trip vs AoS; SmolLM2 row-bytes |

Optional W4 checkpoint already referenced (must not be required for W1/W2):

- `SmolLM2-360M-Instruct-Q4_K_M.gguf` (several `find_model` / `docs/models/` paths)
- `C:/LLM_Models/GGUF/hugging-quants/Llama-3.2-3B-Instruct-Q4_K_M-GGUF/llama-3.2-3b-instruct-q4_k_m.gguf`

---

## 7. Adjacent, not Q4_K

| Path | Why it is not this map |
|---|---|
| `inference/moe/nvfp4.rs` | NVFP4 expert GEMV (Qwen). Different type. Uncommitted M1/M2. |
| `inference/moe/dispatch.rs`, `gguf_bridge/moe_ffn.rs` | MoE wiring; off-limits D8 |
| Q8_0 / Q6_K / Q4_0 / Q5_0 dequant in the same files | Sibling codecs; W1 fixture is Q4_K only |
| UI labels (`poet` docks, resource catalog, `AI_INSTRUCTIONS.md`) | Advisory names `"Q4_K_M"` |

---

## 8. What W1/W2 should hook vs avoid

**Read and match (oracle):**

- `dequant_q4_k` + `get_scale_min_k4` (`ggml_quants.rs`)
- Superblock constants 256 elems / 144 bytes
- WGSL `dequant_q4_k_elem$S` only as a cross-check that CPU and shader agree on nibble/scale packing

**Compare against in W3 (existing executor):**

- `stack_gemm_quant` on type 12 (CPU)
- Optionally `fused_transformer.wgsl` type-12 coop GEMV (same represented weights)

**Do not edit in W1/W2:**

- `gguf_bridge/**`, `inference/moe/**`, `p64_weight/{layout,compiler,mod}.rs`, sticky decode/prefill (D8)
- `q4k_block_to_soa` / P64 SOA compiler (D2)

**Do not treat as the first fixture:**

- Type 112 SOA tensors
- `"Q4_K_M"` capability strings
- Mixed-quant GGUFs (attention Q4_K, other tensors Q6_K/Q8_0) as a single-tensor experiment

---

## 9. Inventory method

```
rg GGML_TYPE_Q4_K / Q4_K_SOA / P64_FLAG_Q4K / dequant_q4_k / try_q4k_soa_ / q4k_block_to_soa
```

over `*.rs` and `*.wgsl`. Call sites above were opened and classified. This packet did not edit `.rs` files and did not rerun GPU tests.
