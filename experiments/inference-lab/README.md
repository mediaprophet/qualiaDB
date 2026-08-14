# Inference superiority lab

**Plan:** `docs/plans/inference-superiority-lab-and-toolset-plan.md`

Native-only instruments. Optional Ollama is a **yardstick** (A-gap), never a product dependency.

## Browser and GGUF diagnostics

Use the supported tools in [`tools/diagnostics`](../../tools/diagnostics/README.md)
for bounded GGUF header/tensor inspection and browser/WebGPU smoke receipts.
They are the supported replacement for one-off root-level scripts and require
an explicit evidence output directory; do not retain `node_modules`, console
dumps, or browser profiles in this lab.

## Commands

```powershell
# Path wiring audit (no GPU required for flag dump)
qualia-cli llm lab audit-path

# Device roof calibration (GPU)
qualia-cli llm lab roof

# Isolated Q4 SoA GEMV microbench
qualia-cli llm lab micro --n-in 256 --n-out 64

# Host+GPU phase timeline (one short decode)
qualia-cli llm lab timeline --model C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.p64 --tokens 4

# Ablation matrix (writes CSV)
qualia-cli llm lab ablate --model C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.p64 --tokens 8 --out experiments/inference-lab/runs.csv

# Full yardstick vs Ollama
.\scripts\compare-qualia-vs-ollama.ps1 -BothSizes -Tokens 32
```

## Autonomous multi-hour self-improve → lock-in

Recursive **measure → discrete config search → re-measure elites/neighbors → plateau/budget stop → lock-in package**.

This is **not** an LLM rewriting kernels. It searches toggles already in the engine (resident, coop GEMV, kv int8, mode, backend), re-samples winners to reduce variance, and emits the derivatives you need to freeze a configuration.

```powershell
# Short smoke (minutes, no Ollama)
cargo run -p qualia-cli --release -- llm lab auto `
  --model C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.p64 `
  --hours 0.15 --tokens 8 --max-generations 2 --no-ollama

# Multi-hour run with A-gap yardstick (leave overnight)
.\scripts\lab-auto-improve.ps1 `
  -Model C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.p64 `
  -Hours 8 -Tokens 16

# Or direct CLI
qualia-cli llm lab auto `
  --model C:\LLM_Models\P64\Llama-3.2-3B-Instruct-Q4_K_M.soa.p64 `
  --hours 4 --tokens 16 `
  --ollama-model qualia-3b:latest `
  --out experiments/inference-lab/lockin
```

### Lock-in package (`experiments/inference-lab/lockin/`)

| File | Purpose |
|------|---------|
| `BEST_CONFIG.json` | Winning toggles + measured tok/s |
| `apply-best.ps1` | Env vars to re-apply the winner |
| `METHODOLOGY.md` | What was searched, audit notes, next engineering (T-A1/T-A2) |
| `runs.csv` | Every trial (A-gap when Ollama probe works) |
| `LOCKIN_SUMMARY.txt` | One-page result |
| `audit_path.txt` / `device_roof.txt` / `micro_q4k.txt` | Baseline instruments |
| `auto_improve.log` | Full generation log |

Apply after lock-in:

```powershell
. .\experiments\inference-lab\lockin\apply-best.ps1
```

## CSV schema (`runs.csv`)

See plan §5.1. Header is written automatically by `lab ablate`, `lab auto`, and `compare-qualia-vs-ollama.ps1 -Csv`.

## Acceptance

No claim of speed win without a new CSV row. **A-gap** = `ollama_tok_s / qualia_tok_s` on fixed pairs.

Search alone cannot close T-A1 (FFN fusion in resident mega-pass) or T-A2 (CUDA full layer stack) — those need code. Re-run `lab auto` after each engineering change to re-lock.

## Native package campaign (autonomous import → profile)

Qualia is **not** only inference: this loop attests **P64 + execution profile** packages as one toolchain step.

```powershell
cargo build -p qualia-cli --release
.\scripts\llm-native-campaign.ps1 `
  -Model C:\LLM_Models\GGUF\smollm2-360m-instruct-q8_0.gguf `
  -OutDir C:\LLM_Models\P64 -Tokens 16
# Optional: -ImportDir … -DeleteSourceOnSuccess -LabHours 0.25 -MaxModels 3
```

Each successful explore writes `{winner}.execution-profile.json` + `.apply-profile.ps1`.  
Progress log: `docs/plans/native-package-campaign-PROGRESS-LOG.md`.

## Machine GPU capability (native tiers over the WGSL floor)

WGSL/wgpu is the portable floor. CUDA-C/PTX, HLSL+DXC, MSL, subgroups and coopmat are higher
tiers *when the host has them and the measurement is coherent*.

```powershell
qualia-cli llm lab gpu-cap `
  --model C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.f16.p64 `
  --tokens 16 --out C:\LLM_Models\P64
```

Probes the toolchain (nvcc / DXC CLI / xcrun) and adapter features, then runs a
backend × mode decode matrix — **one child process per cell**, because `shared_gpu` is
process-wide and `QUALIA_WGPU_BACKEND` cannot be re-pointed in-process. Only coherent rows
can win. Writes `machine-gpu-profile.json` + `apply-machine-gpu.ps1` to `--out`.

Measured on the SmolLM f16 winner (A2000, 16 tokens, all coherent):

| Backend | portable | fast-verify | cuda |
|---------|---------:|------------:|-----:|
| vulkan  | 47.8 | **87.5** | 46.9 |
| dx12    | 34.5 | 73.6 | 36.2 |

```powershell
. C:\LLM_Models\P64\apply-machine-gpu.ps1
```

## Native shader emitter matrix

The forge emits dedicated, non-empty bodies for all supported kernels across
WGSL, HLSL, MSL, and CUDA-C. Each native emitter mirrors the certified WGSL math
with target-native intrinsics.

```powershell
# Verify a single kernel/target
qualia-cli shader generate gemm --target hlsl

# Verify all 15 kernel/target combos (PowerShell)
$kernels = @("gemm","gemv","fft","ternary-gemv","p64-project")
$targets = @("hlsl","msl","cuda-c")
foreach ($k in $kernels) { foreach ($t in $targets) {
    qualia-cli shader generate $k --target $t
} }
```

| Kernel | HLSL | MSL | CUDA-C |
|--------|:----:|:---:|:------:|
| `gemm` | ✅ | ✅ | (graph IR) |
| `gemv` | ✅ | ✅ | (graph IR) |
| `fft` | ✅ `reversebits` | ✅ `reverse_bits` | ✅ `__brev` |
| `ternary-gemv` | ✅ `StructuredBuffer<uint>` | ✅ `device const uint` | ✅ `__global__` |
| `p64-project` | ✅ `GetDimensions` | ✅ `record_count` | ✅ `P64Words64` |

Structural test: `cargo test -p qualia-core-db --lib -- native_emitters_produce_non_empty_bodies`
