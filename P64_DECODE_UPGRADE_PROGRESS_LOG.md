# P64 + Decode Upgrade — Progress Log

**Plan:** [`docs/plans/p64-decode-upgrade-plan.md`](docs/plans/p64-decode-upgrade-plan.md)  
**Rule:** Update the plan when new opportunities are found (§0).

---

## 2026-07-10 — P0 + P1 complete

### Step / phase
- **P0** done.
- **P1** done (C0–C4).

### What was built
- Living plan: `docs/plans/p64-decode-upgrade-plan.md` (§0: update plan on new opportunities).
- This progress log.
- **C0:** stop re-sorting known tensors by GGUF offset; keep layer-major blob order.
- **C1/C2:** standard document version 0.2 / container version 4; document `dtype=112` Q4_K_SOA + §7.3 layer-major.
- **C3:** header `reserved` actually read/written (was always zeros on read).
- **C4:** `P64_FLAG_Q4K_SOA` + `P64_FLAG_LAYER_MAJOR` set on compile.
- Test: `p64_layer_major_flag_and_header_reserved_round_trip`.

### Measured results
- `cargo test -p qualia-core-db --lib p64`: **12 passed**, 0 failed, 4 ignored.
- Decode tok/s not re-measured this step (container-only); baseline remains ~6.7 on 3B SoA.

### ⚑ Where I need the human
- **Re-convert** production models with `--layout q4ksoa` (or lab convert path) to pick up layer-major + flags on disk. Existing `.soa.p64` files remain valid but lack the new flags until recompiled.

### Next step
- **P2:** `DECODE_PROXY` regression lock smol + 3B SoA after rebuild.
- **P3:** CUDA-native layer GEMV chain (E1) — primary path to close the ~10× gap.

### Plan deltas
- Plan §7 marked P0/P1 done.

---

## 2026-07-10 � P2 + P3 partial (pipeline-opt P64 + CUDA kernels)

### Step / phase
- **P2** done (resident regression).
- **P3** partial (kernels + preload; CUDA_DECODE still slow).

### What was built
- **P64 layer-pack:** page-align only at layer boundaries; 256 B within layer (`P64_FLAG_LAYER_PACK`).
- **P64 layer schedule table:** `P64LayerScheduleEntry[n_layer]` at `role_table_offset` (`P64_FLAG_LAYER_SCHEDULE`).
- **Standard** updated (�7.4).
- **CUDA:** `Q4K_SOA_GEMV_ROWS=16`, parallel multi-acc reduce; `preload_q4k_soa_weights` at plan (~168 matrices).
- **Bind-group fixes** for CoopGemvBGL 5-slot (output, fused_tail).

### Measured results
| Path | tok/s (3B soa.p64, 32 tok) |
|------|---------------------------:|
| Resident mega-pass (default) | **6.73�6.84** |
| CUDA_DECODE lab | **0.74** |
| Preload | **168** SoA weights |

### ? Where I need the human
- Re-convert GGUF?P64 for layer-pack+schedule on disk.
- Next invest: device-side SDPA/KV (plan P4).

### Plan deltas
- §7 P2 done, P3 partial; §8 CUDA host-SDPA bottleneck.

---

## 2026-07-10 — P4 partial (device SDPA/KV only)

### Step / phase
- **P4** partial (E2 architecture landed; tok/s bar not met; E3/E4 open).

### What was built
- CUDA-C: `rope_interleaved`, `kv_slot_write`, `sdpa_decode_gqa` (`wgsl_forge/emit/cuda_c.rs`).
- `ensure_device_kv_cache` + permanent f32 device KV (~224 MiB on 3B) on multi-weight slab (2.5 GiB prefer).
- `try_q4k_soa_attention_device`: sticky-x QKV → RoPE → KV write → GQA SDPA → O-proj; **one** residual D2H.
- `CudaPipeline::dispatch_async` — same-stream order, fence on `read_buffer_*` only.
- CUDA_DECODE forces f32 KV (`set_kv_int8(false)` + `QUALIA_LLM_KV_INT8=0`) so device indices match host layout.
- Wire: `attention.rs` prefers device path; `resident_decode` reserves KV before weight preload when planning.
- FFN block SwiGLU/down also `dispatch_async`.

### Measured results
| Path | Model | tokens | tok/s |
|------|--------|-------:|------:|
| Resident mega-pass (default) | 3B SoA | (prior) | **~6.7–6.8** |
| CUDA_DECODE + device SDPA (P4) | 3B SoA | 16 | **0.71–0.73** |
| Device KV | — | — | **224 MiB** resident |
| Evidence | log | — | `cuda_attn\|device_sdpa\|first_hit` + `cuda_lane\|kv\|resident` |

Honest: P4 closed the *host-SDPA* design hole but **did not** close the ~10× gap. Layer-by-layer CUDA remains far behind resident mega-pass.

### ⚑ Where I need the human
- None required to accept this partial. Direction call if you want P4 continued as sticky full-token residual (sample-only D2H) vs stay on resident default and treat CUDA as densify-only.

### Next step
- **P4 remainder:** E3 logits large-tile; E4 prefill TC; optional full sticky residual stream (true device chain).
- **P5:** warm daemon + passport stretch when CUDA is no longer a regression.

### Plan deltas
- §7 P4 → partial; §8 P4 E2 + remaining gap.

---

## 2026-07-10 — Default decode faster (parallel RMSNorm)

### Step / phase
- **P4 continue** — make **default** decode better (not CUDA_DECODE lab path).

### What was built
1. **Parallel RMSNorm** in `shaders/wasm_elementwise.wgsl`: `@workgroup_size(256)` strided sum-of-squares + tree reduce + weighted write. Replaces single-thread loop over 3072–4096 dims (~57 RMS passes/token).
2. **No CUDA SoA preload** unless `QUALIA_LLM_CUDA_DECODE=1` — resident path no longer doubles ~1.8 GiB into the CUDA slab.
3. **Triple QKV** shader + wire (`triple_gemv.wgsl`) — **opt-in only** (`QUALIA_LLM_TRIPLE_QKV=1`); A/B lost vs dual+Q on A2000.

### Measured results (3B SoA, 32 tok, resident default)
| Config | tok/s |
|--------|------:|
| Prior baseline | **~6.70** |
| Parallel RMS + no CUDA preload | **~8.89–8.94** |
| + triple QKV (opt-in) | **~8.57** (worse — not default) |

~**+33%** on the product path. Plan gap to Ollama (~70) still large.

### ⚑ Where I need the human
- none this step

### Next step
- Prefill / logits (E3–E4), or further fused_block kernel density; keep CUDA_DECODE lab-only until sticky full-token residual exists.

### Plan deltas
- §7 P2 notes + P4 partial updated; §8 default-path win recorded.

---

## 2026-07-10 — E3 logits + dual_gemv subgroup

### Step / phase
- Continue decode density (architecture free).

### What was built
1. **E3:** Full-vocab logits buffer (131072); one GEMV+topk when vocab ≤ cap. Force multi-row logits when rows > 60k (device WG limit 65535).
2. **dual_gemv:** `subgroupAdd` reduce (was shared-memory tree).
3. **ffn_mr reduce:** subgroupAdd path (still opt-in; A/B ~tie).
4. Reverted default-on multirow residual / ffn_mr (A/B lost or neutral).

### Measured results (3B SoA, 32 tok, resident default)
| Config | tok/s |
|--------|------:|
| Prior (after RMSNorm) | **~8.9** |
| + full logits + dual subgroup | **~8.96–9.06** |
| Timeline | fused_block still dominates GPU time |

Honest: incremental vs prior 8.9; **~+33–35% vs original 6.7**. Not Ollama-class.

### ⚑ Where I need the human
- none

### Next step
- Fused_block density (attention SDPA/GEMV) or sticky CUDA full-token if it can beat ~9.
- E4 prefill still open.

### Plan deltas
- §7 P4 notes ~9.0; §8 E3 + dual_gemv.

---

## 2026-07-10 — E4 prefill: one pass + fuse

### Step / phase
- **E4** partial (structural density; true dense batch GEMM still open).

### What was built
1. **Prefill single compute pass** — was `begin_compute_pass` per op (~420/chunk); now one pass for all layers (mirror resident_decode).
2. **Residual-fused O/down** in prefill (drop separate add dispatches).
3. **Fused FFN** + **dual K+V** on SoA prefill (same as decode).
4. Separate K/V projection buffers for dual write.

### Measured results
| Metric | Value |
|--------|------:|
| Decode tok/s (3B SoA, 32) | **~8.9–9.1** (held) |
| Prefill arena | `10 dispatches/layer, 1 compute pass/chunk, fused_ffn+dual_kv` |
| Timeline prefill_ns (passport, multi-trial aggregate) | ~19s / 14-token prompts (includes multi-run measure; not a single-prompt TTFT) |

Honest: decode plateau ~9; prefill architecture now matches decode density. Remaining prefill win = real M×K×N TC GEMM for B>1, not more dispatch shaving.

### ⚑ Where I need the human
- none

### Next step
- True batched prefill GEMM (E4 remainder) and/or fused_block decode kernel density.

### Plan deltas
- §7 P4 E4 noted; §8 prefill fix.

---

## 2026-07-10 — dual multi-row A/B (no default change)

### Step / phase
- Fused_block density experiments (dual multi-row, selective down multirow).

### What was built
1. `coop_gemv_dual_mr` (4 rows/WG, subgroup reduce) + `dual_gemv_mr_pipeline`.
2. Wire opt-in via `QUALIA_LLM_DUAL_MR=1` (decode + prefill).
3. Tried default-on dual_mr + down multirow → **~8.75 tok/s** (regression).

### Measured results
| Config | tok/s |
|--------|------:|
| Restored default (dual 1-row) | **~9.13** |
| dual_mr + down multirow default | **~8.75** (lose) |

### ⚑ Where I need the human
- none

### Next step
- True M=B prefill GEMM or fundamentally denser FFN dequant (not more multirow packing). Effective BW ~5% of peak → occupancy/latency wall.

### Plan deltas
- §8 dual multi-row A/B lose.

---

## 2026-07-10 — fused_block: barrier-free global-act Q4_SOA

### Step / phase
- **fused_block density** (product path). Full-act LDS experiment first (lose), then barrier-free global.

### What was built
1. **A/B lose:** full-act / chunked-4096 LDS in `coop_row_dot` + `dual_gemv` → **~8.51 tok/s** (occupancy).
2. **Win path:** restore barrier-free single-row Q4_SOA:
   - `fused_transformer.wgsl` `coop_row_dot`: each lane loads `input[block*256+t]` from global; **no FMA-loop barriers**.
   - `dual_gemv.wgsl`: same for K+V dual (and dual_mr); dropped 16 KiB `dual_full_act`.
   - `fused_ffn.wgsl` expansion + SG + multi-row: same global-act pattern; dropped `coop_ffn_full_act`.
3. Subgroup reduce paths unchanged (still default where adapter has SUBGROUP).

### Measured results (3B SoA, 32 tok, resident default)
| Config | tok/s |
|--------|------:|
| Prior peak (dual 1-row + subgroup) | **~9.13** |
| Full-act LDS (reject) | **~8.51** |
| Barrier-free global-act | **9.70–9.86** |

~**+7–8%** vs prior peak; **~+45%** vs original ~6.7. Still far from Ollama ~70.

### ⚑ Where I need the human
- none this step

### Next step
- Attention densify inside fused_block (SDPA/RoPE) or true M=B prefill GEMM. Avoid more multirow packing (A/B lose). CUDA_DECODE remains lab-only.

### Plan deltas
- §7 P4 notes ~9.8–9.9; §8 full-act lose + global-act win.

---

## 2026-07-10 — Plan rewrite (progress + beat-Ollama backlog)

### Step / phase
- **Plan maintenance only** (no kernel change this entry).

### What was built
Rewrote [`docs/plans/p64-decode-upgrade-plan.md`](docs/plans/p64-decode-upgrade-plan.md):

1. **Scoreboard** — 6.7 → 9.7–9.9 trajectory vs Ollama ~70; G0–G5 bars (G4 = beat Ollama).
2. **Done / rejected** tables (container, resident, CUDA lab, failed A/B).
3. **Gap analysis** — ~5% peak BW → not bandwidth-bound; need architecture change.
4. **Comprehensive worklist** bands:
   - **R** revolution (fused layer, CUDA sticky residual, TC/mmq, flash SDPA, P64 decode profile, warm daemon)
   - **H** high (Q density, vertical FFN, prefill GEMM, KV quant, single residency, logits sample path)
   - **M** medium (passport warp select, micro-opts, RoPE fuse, prefetch)
   - **L** hygiene (reconvert, u64 offsets, metrics, A/B harness)
   - **X** non-goals
5. **Phases P5–P8** aimed at G4; **next 5 actions** (L5 timeline split → H2 FFN fuse → R1/R3 spikes).
6. One-page principal summary.

### Measured results
- Not re-measured (docs only). Current peak remains **9.70–9.86 tok/s**.

### ⚑ Where I need the human
- Direction call if he wants **P5 resident-first** vs **P7 CUDA sticky-first** as the allocated instrument lane. Default in plan: P5 next, R2 only after sticky design wins a spike.

### Next step
- Execute plan §8 item 1 (L5 fused_block phase split) unless Timothy reallocates.
