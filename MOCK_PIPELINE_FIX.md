# Mock Pipeline Issue - wgpu/WGSL Fallback Path

## Problem Identified

The wgpu/WGSL fallback path uses `mock_pipeline` with a placeholder shader instead of the real quantized GEMM shader.

## Critical Finding: Two Different Shaders

### Real Pipeline (lines 395-405)
- **Shader**: `fused_transformer.wgsl`
- **Features**: Real quantized GEMM with Q4_K/Q6_K dequantization
- **Dimensions**: Dynamic via GemmParams struct
- **Status**: ✅ Real implementation

### Mock Pipeline (lines 407-416)
- **Shader**: `fused_tensor_contraction.wgsl`
- **Features**: Hardcoded 4096 dimensions, simple matmul, ReLU
- **Comment in shader**: "Very simplified placeholder for a fused attention + FFN block"
- **Status**: ⚠️ Placeholder/mock implementation

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
- Mock pipeline fix: ✅ Implemented (2026-06-11)
- Orchestrator integration: ⏳ Pending

## Fix Applied (2026-06-11)

`dispatch_fused_transformer_block` in `gguf_bridge.rs` was updated:

1. `self.mock_pipeline` → `self.pipeline` (real `fused_transformer.wgsl`)
2. Hardcoded `(4096 * 4)` output size → dynamic `(rows * 4).max(4)`
3. `GemmGpuParams` uniform buffer created and uploaded (binding 2); output moved to binding 3
4. `dispatch_workgroups(4096 / 64, 1, 1)` → `dispatch_workgroups((rows as u32 + 63) / 64, 1, 1)`
5. Telemetry: `4096 * 4096` → `rows * cols`

The wgpu/Vulkan path on Linux now uses the same quantized GEMM shader (`fused_transformer.wgsl`)
as the real `dispatch_gemm_raw_into()` path.

## Other Mock Implementations Found

During codebase search, identified other mock implementations:

### pinn_extension.rs (Physics-Informed Neural Networks)
- `mock_neural_forward()` - Placeholder neural network computation
- Fallback when native backend unavailable
- Used in test with "mock_fluid_model"

### quantum_dft.rs (Quantum DFT)
- `calculate_ground_state_energy()` - Mock DFT convergence
- Returns mock Hydrogen ground state (-13.6 eV)
- Comment: "In a real implementation, this would iteratively solve Kohn-Sham equations"

### webizen.rs (Webizen VM)
- Line 521: Mock tax schema evaluation
- Lines 1236, 1252: Mock continuous grid for calculus operations
- Lines 1518-1521: Test mock VM for CRDT queue

### gguf_sharder.rs (GGUF Parser)
- Lines 568-572: Mock tensor byte offsets for tests
- Lines 911, 932: Mock model names in tests

### Shader Files
- `fused_tensor_contraction.wgsl`: Contains "ReLU mock" comment, hardcoded 4096 dims
- `fluid_dynamics.wgsl`: "Placeholder Navier-Stokes mock update"
- `kinematics.wgsl`: "Gravitational/Electrostatic Force mock", "mock Euler step"

## Priority

1. **HIGH**: gguf_bridge.rs mock_pipeline - Blocks real LLM inference on Linux/wgpu path
2. **MEDIUM**: pinn_extension.rs mock - Affects physics simulations
3. **LOW**: quantum_dft.rs mock - Affects quantum calculations (experimental feature)
4. **LOW**: webizen.rs mocks - Mostly for tests/demonstrations