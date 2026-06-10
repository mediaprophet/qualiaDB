# Mock Pipeline Issue - wgpu/Vulkan Path

## Problem Identified

The wgpu/Vulkan path (advertised in README as the Linux inference route) uses `mock_pipeline` instead of the real `fused_transformer.wgsl` shader.

## Location

File: `crates/qualia-core-db/src/gguf_bridge.rs`
Function: `dispatch_fused_transformer_block`
Lines: 984-1049 (wgpu/WGSL fallback section)

## Current Issues

1. **Line 992**: Uses `self.mock_pipeline.get_bind_group_layout(0)` instead of `self.pipeline`
2. **Line 1020**: Uses `self.mock_pipeline` instead of `self.pipeline`
3. **Line 1022**: Hardcoded dispatch `dispatch_workgroups(4096 / 64, 1, 1)` instead of actual dimensions
4. **Line 984**: Hardcoded output size `(4096 * 4)` instead of actual dimensions
5. **Line 1048**: Hardcoded telemetry `4096 * 4096` instead of actual operation count

## What Needs to Be Fixed

The mock pipeline section needs to be replaced with logic that:

1. Uses `self.pipeline` (the real fused_transformer shader) instead of `self.mock_pipeline`
2. Calculates actual output size based on `rows` and `cols` parameters
3. Dispatches workgroups based on actual dimensions: `(n_out as u32 + 63) / 64`
4. Reports actual telemetry based on real operation count

## Reference Implementation

See lines 1106-1148 in the same file for how the real pipeline is used correctly:

```rust
// Line 1106-1148 shows correct usage:
let n_out = rows * (input_activations.len() / cols.max(1));
let output_size = (n_out * 4) as wgpu::BufferAddress;
// ...
cpass.set_pipeline(&self.pipeline);
cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
```

## Impact

- The wgpu/Vulkan/Linux path currently runs placeholder computations
- Real inference requires this fix to use actual GPU compute
- DirectML path (Windows) and Accelerate BLAS (macOS) are not affected

## Status

- Tokio runtime fixes: ✅ Committed (cff37440)
- Mock pipeline fix: ⚠️ Requires careful implementation
- Orchestrator integration: ⏳ Pending