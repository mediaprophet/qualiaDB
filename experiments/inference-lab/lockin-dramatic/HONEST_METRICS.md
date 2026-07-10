# Native decode performance record

No external product is used as a success criterion. Numbers below are from this machine,
this binary, and the named files only.

## Hardware / tool
- GPU: NVIDIA RTX A2000 12GB (Vulkan primary mega-pass; CUDA optional lane)
- CLI: `target/release/qualia-cli.exe llm decode-proxy <model.p64> --tokens N`
- Metric: `DECODE_PROXY tok_s=` under a fixed token budget

## Models present (`C:\LLM_Models\P64\`)
| File | ~MB | Role in this work |
|------|----:|-------------------|
| `llama-3.2-3b-instruct-q4_k_m.soa.p64` | 2069 | Primary (SoA Q4 layout) |
| `llama-3.2-3b-instruct-q4_k_m.p64` | 1925 | Non-SoA twin (not looped this session) |
| `smollm2-360m-instruct-q8_0.p64` | 370 | Small-model regression |
| `smollm2-360m-instruct-q8_0.f16.p64` | 693 | F16 variant |
| `gemma-4-E2B-it-Q4_K_M.p64` | 3270 | Present; not yet in decode loop |

## Measured 2026-07-10 (this session)

| model file | mode | tokens | tok_s |
|------------|------|-------:|------:|
| `llama-3.2-3b-instruct-q4_k_m.soa.p64` | `QUALIA_INFERENCE_MODE=cuda`, resident mega-pass ON | 32 | **6.59** |
| `smollm2-360m-instruct-q8_0.p64` | `fast-verify` | 32 | **60.3** |
| `smollm2-360m-instruct-q8_0.p64` | `portable` | 32 | **26.8** |

Resident plan on 3B SoA: 28 layers, **289 passes/token** (dual K+V), `fused_ffn=true`.

## Real engineering delivered
- Dual K+V GEMV (shared activation)
- Residual-fused O/down GEMV
- Fused FFN expansion (gate+up+SiLU)
- CUDA sticky activation **overwrite** (fixed permanent-slab leak)
- CUDA multi-weight slab up to **2 GiB** with soft-fail (no thrash clear-all)
- Fused CUDA Q/K/V kernel (one act stream for three projections)
- Q6_K block-coop path in WGSL (logits family)

## Not claimed
- Any third-party product throughput figure as a goal or proof
- That the CUDA layer-by-layer path beats the resident mega-pass today (it does not)
- That 3B SoA is near hardware peak (it is still weight/dequant bound on the mega-pass)

## Primary command (production-shaped path)
```
$env:QUALIA_INFERENCE_MODE='cuda'
$env:QUALIA_LLM_FFN_FUSION='1'
# leave QUALIA_LLM_CUDA_DECODE unset
.\target\release\qualia-cli.exe llm decode-proxy C:\LLM_Models\P64\llama-3.2-3b-instruct-q4_k_m.soa.p64 --tokens 32
```

## Local A/B vs Ollama (same machine, 2026-07-10)

**Purpose:** distinguish *software path* limits from *hardware ceiling* on this GPU.
This is diagnostic evidence on **this host only**, not a product claim about any vendor.

### Method
| Side | Model | Decode measure |
|------|--------|----------------|
| **Ollama** 0.31.1 | `llama3.2:3b-instruct-q4_K_M` (installed) | API `eval_count / eval_duration` (generation only) |
| **Qualia** release CLI | `llama-3.2-3b-instruct-q4_k_m.soa.p64` | `DECODE_PROXY tok_s=` fixed `--tokens 32` |
| GPU | NVIDIA RTX A2000 12GB | same card for both |

Fairness notes:
- Same architecture family and quant class (Llama 3.2 3B instruct **Q4_K_M**).
- Qualia uses a **SoA-repacked** P64 of that quant; Ollama uses its native GGUF of the same quant tag.
- Token budget 32 on both; Ollama decode rate excludes prompt eval; Qualia decode-proxy is generation-oriented fixed budget.
- Ollama numbers are warm-run after one discard load; Qualia runs include process cold start each CLI invocation (slightly harder on Qualia).

### Results

**Llama 3.2 3B Q4_K_M class**

| System | Decode tok/s (runs) | Mean |
|--------|---------------------|-----:|
| Ollama `llama3.2:3b-instruct-q4_K_M` | 71.8, 69.9, 69.8 | **~70.5** |
| Qualia `…q4_k_m.soa.p64` resident mega-pass | 6.84, 6.74, 6.71 | **~6.8** |

Ratio: Ollama / Qualia ≈ **10.4×** on this GPU for this model class.

**Smol ~360M class (secondary)**

| System | Decode tok/s |
|--------|-------------:|
| Ollama `qualia-smol-q8:latest` | ~191 then ~239 (warm) |
| Qualia `smollm2-360m-instruct-q8_0.p64` fast-verify | ~59–62 |

### Verdict
- **Not a hard hardware ceiling.** The A2000 sustains ~70 decode tok/s on the same 3B Q4_K_M class under Ollama.
- **Qualia’s ~6.8 is a software/path efficiency gap** (kernel + dispatch + quant path), not “the card can’t go faster.”
- Rough memory-bound ceiling for 3B Q4 decode on this GPU is still higher than Ollama’s ~70; Ollama is already in a credible efficiency band; Qualia is ~10× behind that local reference.
