# Task 006: Fix wgpu/Vulkan Mock Pipeline - Use Real Fused Transformer Shader

## Problem
The wgpu/Vulkan fallback (Linux inference route per README) dispatches `mock_pipeline`, which uses `fused_tensor_contraction.wgsl` - a placeholder shader with hardcoded 4096 dimensions and no dequantization. The real `fused_transformer.wgsl` shader exists but is unused in the wgpu path.

**File**: `crates/qualia-core-db/src/gguf_bridge.rs`  
**Lines**: 984-1049 (mock section)  
**Severity**: 🔴 HIGH

## Current State

### Real Pipeline (Lines 395-405)
```rust
let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("Fused Transformer Shader"),
    source: wgpu::ShaderSource::Wgsl(
        include_str!("shaders/fused_transformer.wgsl").into()
    ),
});

let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: Some("Fused Transformer Pipeline"),
    layout: None,
    module: &shader,
    entry_point: "main",
});
```

### Mock Pipeline (Lines 407-416)
```rust
let mock_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
    label: Some("Mock Fused Contraction Shader"),
    source: wgpu::ShaderSource::Wgsl(
        include_str!("shaders/fused_tensor_contraction.wgsl").into()
    ),
});

let mock_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
    label: Some("Mock Fused Contraction Pipeline"),
    layout: None,
    module: &mock_shader,
    entry_point: "main",
});
```

### Mock Usage (Line 1020)
```rust
cpass.set_pipeline(&self.mock_pipeline); // Should be self.pipeline
cpass.set_bind_group(0, &bind_group, &[]);
cpass.dispatch_workgroups(4096 / 64, 1, 1); // Hardcoded dimensions
```

### Shader Difference

**fused_transformer.wgsl (Real)**:
- Dynamic dimensions via GemmParams struct
- Q4_K/Q6_K dequantization
- Proper weight byte reading
- Actual quantized GEMM

**fused_tensor_contraction.wgsl (Mock)**:
- Hardcoded 4096 dimensions
- Comment: "Very simplified placeholder for a fused attention + FFN block"
- Comment: "ReLU mock"
- No dequantization

## Implementation Plan

Replace mock_pipeline usage with real pipeline:

1. **Change pipeline selection** (Line 992):
   ```rust
   let bind_group_layout = self.pipeline.get_bind_group_layout(0);
   ```

2. **Change pipeline usage** (Line 1020):
   ```rust
   cpass.set_pipeline(&self.pipeline);
   ```

3. **Fix dispatch dimensions** (Line 1022):
   ```rust
   // Calculate actual output dimensions
   let n_out = rows * (input_activations.len() / cols.max(1));
   cpass.dispatch_workgroups((n_out as u32 + 63) / 64, 1, 1);
   ```

4. **Fix output buffer size** (Line 984):
   ```rust
   let n_out = rows * (input_activations.len() / cols.max(1));
   let output_size = (n_out * 4) as wgpu::BufferAddress;
   ```

5. **Fix telemetry** (Line 1048):
   ```rust
   crate::telemetry::SIEVE_OPS_COUNT
       .fetch_add(rows * (input_activations.len() / cols.max(1)),
                  std::sync::atomic::Ordering::Relaxed);
   ```

6. **Update bind group layout** to match real pipeline requirements:
   ```rust
   // Real pipeline uses GemmParams uniform buffer
   let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
       label: Some("GemmParams"),
       size: std::mem::size_of::<GemmParams>() as u64,
       usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
       mapped_at_creation: false,
   });

   // Upload GemmParams
   let params = GemmParams {
       n_in: cols as u32,
       n_out: rows as u32,
       weight_ggml_type: 12, // GGML_TYPE_Q4_K
       weight_row_elems: cols as u32,
       weight_byte_len: (rows * cols) as u32,
   };
   queue.write_buffer(&params_buffer, 0, bytemuck::cast_slice(&[params]));
   ```

## Implementation Steps

1. Replace `self.mock_pipeline` with `self.pipeline` in line 992
2. Replace `self.mock_pipeline` with `self.pipeline` in line 1020
3. Calculate actual output dimensions instead of hardcoded 4096
4. Update dispatch_workgroups to use actual dimensions
5. Add GemmParams uniform buffer setup
6. Update bind group to include GemmParams
7. Update telemetry to report actual operation count
8. Test with real GGUF model
9. Benchmark performance vs mock pipeline
10. Update documentation to reflect real capabilities

## Success Criteria
- ✅ wgpu/Vulkan path uses real fused_transformer.wgsl
- ✅ Dimensions are calculated dynamically, not hardcoded
- ✅ Q4_K/Q6_K dequantization is performed
- ✅ Real weights are used, not placeholder
- ✅ Inference produces correct outputs
- ✅ Performance is acceptable
- ✅ Linux inference works correctly
- ✅ Documentation updated

## Related Files
- `crates/qualia-core-db/src/gguf_bridge.rs` (main)
- `crates/qualia-core-db/src/shaders/fused_transformer.wgsl` (real shader)
- `crates/qualia-core-db/src/shaders/fused_tensor_contraction.wgsl` (mock shader)
- `crates/qualia-core-db/src/llm_agent.rs` (inference orchestration)
- `README.md` (Linux inference claims)
- `MOCK_PIPELINE_FIX.md` (documentation)

## Estimated Complexity
- Core fix: 1-2 days
- Testing and benchmarking: 1-2 days
- **Total**: 2-4 days

## Dependencies
- Can be done independently
- May coordinate with tokio runtime fixes (already completed)
- Requires test GGUF model

## Platform Impact

| Platform | Before Fix | After Fix |
|----------|-----------|-----------|
| **Linux** | Mock pipeline (placeholder) | Real GPU inference ✅ |
| **macOS** | Mock fallback (when mmap not loaded) | Real fallback ✅ |
| **Windows** | Mock fallback (when DirectML unavailable) | Real fallback ✅ |

## Notes
- This is critical for Linux inference
- The real shader already exists and looks correct
- DirectML and Accelerate BLAS paths are not affected
- This fix enables real GPU compute on all platforms
- Benchmark to ensure performance is acceptable
- Update MOCK_PIPELINE_FIX.md when complete
- This was identified in Claude review 2026-06-10