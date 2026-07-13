# Qualia WGSL Forge

WGSL Forge is Qualia’s deterministic shader generator, validator, correctness
certifier, and hardware tuner. It does not use an LLM. Kernel meaning is represented
separately from its execution schedule so optimisation cannot silently rewrite the
mathematics being tested.

Forge is a native/default tooling feature. It is excluded from the no-default-feature
WASM-lite profiles, so Naga and the tuning machinery do not inflate ontology-only
browser builds.

## Commands

```powershell
# Discover kernels
cargo run -p qualia-cli -- shader list-kernels

# Generate portable WGSL
cargo run -p qualia-cli -- shader generate affine-f32 `
  --workgroup 64 --items 2 --vector-width 4 --out affine.wgsl

# Parse and semantically validate with Naga (no GPU required)
cargo run -p qualia-cli -- shader validate `
  --kernel affine-f32 --workgroup 128 --items 2 --vector-width 4

# Create a real pipeline, compare against the CPU oracle, and profile
cargo run -p qualia-cli -- shader certify affine-f32 `
  --length 4099 --workgroup 64 --items 2 --vector-width 4 `
  --manifest affine-certification.json

# Search a bounded schedule space and retain a reproducible tuning record
cargo run -p qualia-cli -- shader tune affine-f32 `
  --length 65537 --max-candidates 48 `
  --manifest affine-tuning.json --cache-dir .qualia/wgsl-forge
```

## What “certified” means

A generated shader advances through explicit evidence levels:

1. deterministic source generation;
2. Naga parse and semantic validation;
3. target-adapter pipeline creation;
4. CPU/GPU differential oracle;
5. robust timing samples;
6. certification manifest with adapter and source identity.

Parsing alone is never called certification.

## Scheduling

The first kernel supports combinations of:

- workgroup sizes `32, 64, 128, 256`;
- `1, 2, 4, 8` items per invocation;
- scalar, `vec2`, and `vec4` local execution.

Every generated variant includes bounds handling for lengths that are not multiples of
the schedule width. Adapter limits prune illegal variants before compilation.

Tuning uses deterministic grid ordering followed by successive halving. Correctness is
a hard gate; only oracle-valid candidates can be ranked. Median latency ranks
candidates, with p95 latency as the first tie-breaker.

## Timing

When the adapter exposes WebGPU timestamp queries, Forge measures GPU pass time.
Otherwise it labels and uses completion-clock timing. Warm-ups run before samples.
Manifests retain timing source, sample count, minimum, median, and p95.

Timing results are not portable. Cache identity incorporates:

- adapter vendor/device/name/type/backend/driver;
- kernel semantic and generated-source hashes;
- WGSL Forge schema and crate version;
- selected schedule.

## P64 and 64-bit values

P64 remains the canonical disk representation. Portable WGSL layouts use paired
`u32` words for 64-bit fields and a 16-word/64-byte descriptor layout when a complete
P64 cache line is required. Native WGSL `u64` is never assumed.

Disk layout and execution layout are deliberately distinct. Future generated P64
kernels may choose AoS or SoA execution views while retaining byte-exact P64 storage.

## Current production boundary

The initial `affine-f32` kernel proves the complete toolchain. Existing inference
shaders are not automatically replaced. Ternary dequantisation, GEMV, fused FFN, and
top-k kernels should migrate only after their generated versions pass equivalent
oracle-backed certification on supported backends.

## LLM decode kernels — certification status (2026-06-29)

The forge now builds and **certifies** a full multi-head (GQA) transformer decode layer against a
CPU oracle on **real SmolLM2-360M weights** (`decode_layer_graph` + `graph_ops/p64_bridge.rs`;
max-rel 3.28e-6 on the A2000): RMSNorm·weight, real RoPE (interleaved + NeoX), GQA attention,
output projection, SwiGLU, residuals — built on on-device `Slice`, a first-class `Rope` op, and a
real `MatMul.trans_b` that consumes the native `[out,in]` GGUF/p64 weight layout.

Stated honestly: this is **certification** (the executor runs the graph node-by-node and diffs the
oracle) — **not** an inference runtime. The engine (`gguf_bridge`) runs inference (~18.8 tok/s decode
on SmolLM2-360M Q8, A2000, Vulkan, compute-bound). And the decode graph currently only *runs* through
the executor: the cross-backend lowerers (`emit/{wgsl,cuda_graph,graph_msl,graph_hlsl}.rs`) do **not**
yet emit `Slice`/`Rope`/`MatMul`, so the forge cannot yet *generate* the decode shader as source for
the engine to adopt. Real SPIR-V emission (`emit/spirv.rs`) exists for the gemm/gemv/fft/affine/top-k
kit. **Ternary 1.58-bit FFN PTQ is a dead end** (PPL ≈ 6.5M on a non-ternary-trained model; Q4_0-AWQ
lands +9.4% over the 5% quality gate) — standard **Q4_K_M** is the shippable compression.

## Inference backend selection

`gpu_context::qualia_backend_override()` honors `QUALIA_WGPU_BACKEND`
(`vulkan|dx12|metal|gl|primary|all`) to pin the inference GPU backend; `recommend_inference_backend()`
advises the portable Vulkan path; `shared_gpu()` logs the selected backend. On the A2000 the default
is **Vulkan** (vendor-neutral), and the override is verified to switch the real device (Vulkan↔DX12).

The implementation lives in
`crates/qualia-core-db/src/wgsl_forge/`; the CLI surface is
`crates/qualia-cli/src/shader.rs`.

For the runtime boundary around P64 residency, production decode, governance,
and Q42 provenance, see the
[Q42/P64 Inference Pipeline](p64-q42-inference-pipeline.md).
