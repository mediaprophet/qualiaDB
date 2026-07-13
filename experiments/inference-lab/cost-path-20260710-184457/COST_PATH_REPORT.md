# Cost-path instrumentation (2026-07-10)

## Tools used (existing lab stack)
- `qualia-cli llm lab audit-path`
- `qualia-cli llm lab roof`
- `qualia-cli llm lab micro --n-in 256 --n-out 64`
- `qualia-cli llm lab timeline --model <soa.p64> --tokens 16` with `QUALIA_LLM_GPU_PROFILE=1`
- Clean control: `llm decode-proxy … --tokens 32` without GPU profile

## Model / mode
- File: `C:\LLM_Models\P64\llama-3.2-3b-instruct-q4_k_m.soa.p64`
- Mode: `QUALIA_INFERENCE_MODE=cuda`, `QUALIA_LLM_FFN_FUSION=1`, resident mega-pass ON
- GPU: NVIDIA RTX A2000 12GB, Vulkan primary

## Results

### Clean decode (no profile)
DECODE_PROXY tok_s=**6.9188** backend=auto tokens=32

### Timeline (GPU profile on)
- tok_s: **6.9431** (profile did not materially change rate)
- wall_ms: 84375 (includes cold load + CUDA weight residency + prefill + decode)
- resident: hits=411 fallbacks=0

### Host phase_ns
| phase | ns | notes |
|-------|---:|-------|
| load_ns | 1.50e8 | ~150 ms |
| prefill_ns | 1.88e10 | **~18.8 s for 14 tokens** (~0.75 prefill tok/s) |
| decode_ns | 3.01e10 | ~30.1 s |
| decode_tokens | 209 | (proxy internal; rate matches 6.94 tok/s) |
| decode_forward_ns | 3.01e10 | **~100% of decode wall** |
| decode_output_ns | 5.54e4 | sampling is noise |

### GPU phase_ns (TIMESTAMP_QUERY; absolute may overcount vs host; **ratios are the point**)
| phase | ns | calls | ns/call | share of profiled GPU |
|-------|---:|------:|--------:|----------------------:|
| fused_block | 5.85e10 | 411 | ~142 ms | **~94.6%** |
| attention | 2.84e9 | 448 | ~6.3 ms | ~4.6% |
| gemm | 5.24e8 | 196 | ~2.7 ms | ~0.8% |

Resident plan log: **28 layers, 289 passes/token, fused_ffn=true**.

### Audit-path (pre-plan)
- resident_decode/prefill/weights: yes; coop_gemv: yes; cuda_caps: yes; timestamps: yes
- ffn_fusion in resident: NO until plan built (audit is static; plan log later shows fused_ffn=true)
- note: full hidden-on-device CUDA stack still open

### Roof / micro
- Roof dense GEMV N=512: CPU native ~6.4 GFLOP/s "wins"; upload_gbps not measured (-1)
- Q4_K SoA micro 256x64: CUDA ~0.98 ms vs CPU ~0.03 ms — **launch overhead dominates tiny shapes**

## Bandwidth back-of-envelope (honest)
- ~2e9 B weights/token if full pass; at 6.94 tok/s → ~14 GB/s effective
- A2000 GDDR6 peak ~288 GB/s → Qualia uses on order of **~5% of peak** if fully weight-bound
- Same-host Ollama ~70 tok/s on llama3.2:3b-instruct-q4_K_M → ~order **~50%** of peak class — not a hardware ceiling

## Honest verdict
Software path efficiency gap, not "card is slow." Cost is almost entirely resident mega-pass **fused_block** (weight matvec/dequant stack at 289 dispatches/token), not sampling/sentinel/output.
