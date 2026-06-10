# Mock Pipeline Issue - wgpu/WGSL Fallback Path

## Problem Identified

The wgpu/WGSL fallback path (used when DirectML or Accelerate BLAS are not available) uses `mock_pipeline` instead of the real `fused_transformer.wgsl` shader.

## Platform-Specific Behavior

### Windows (DirectML)
- **Primary path**: DirectML (D3D12) - ✅ Uses real GPU kernels
- **Fallback**: wgpu/Vulkan - ⚠️ Uses mock_pipeline (affected)

### macOS (Apple Silicon)
- **Primary path**: Accelerate BLAS (AMX) - ✅ Uses real Apple framework
- **Fallback**: wgpu/Metal - ⚠️ Uses mock_pipeline (affected when mmap not loaded)

### Linux
- **Primary path**: wgpu/Vulkan - ⚠️ Uses mock_pipeline (affected)
- **No DirectML**: Not available on Linux

## Impact

- **Linux**: Entirely affected - only wgpu/Vulkan path available
- **macOS**: Partially affected - only when mmap not loaded (Accelerate BLAS works)
- **Windows**: Minimally affected - DirectML is primary and works correctly

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