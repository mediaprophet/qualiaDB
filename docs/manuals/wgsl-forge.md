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

The implementation lives in
`crates/qualia-core-db/src/wgsl_forge/`; the CLI surface is
`crates/qualia-cli/src/shader.rs`.
