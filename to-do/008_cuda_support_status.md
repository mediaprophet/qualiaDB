# Task 008: CUDA Support Status

## Current Status

### CUDA GPUDirect Storage (Linux Only - Opt-In)

**File**: `crates/qualia-core-db/src/modalities/calculus/cuda_bridge.rs`  
**Feature Flag**: `cuda_gds` (opt-in)  
**Platform**: Linux only  
**Status**: ✅ Implemented (for calculus operations only)

### What CUDA Support Exists

The project has **CUDA GPUDirect Storage (GDS)** support, which is different from CUDA compute:

1. **Purpose**: DMA transfers from NVMe to GPU VRAM, bypassing CPU RAM
2. **Use Case**: Calculus operations (numerical integration, differential equations)
3. **Platform**: Linux only
4. **Feature Flag**: `--features cuda_gds` (opt-in)
5. **Requirements**: NVIDIA drivers + cuFile library

### What CUDA Support Does NOT Include

- ❌ CUDA compute kernels for LLM inference
- ❌ CUDA backend for wgpu (uses Vulkan instead)
- ❌ Direct CUDA API calls in inference path
- ❌ CUDA tensor operations (uses wgpu compute shaders instead)

### LLM Inference GPU Backends

| Platform | Primary Backend | Secondary Backend | CUDA Used? |
|----------|----------------|-------------------|------------|
| **Windows** | DirectML (D3D12) | wgpu/Vulkan fallback | No |
| **Linux** | wgpu/Vulkan | None (mock pipeline currently) | No* |
| **macOS** | Accelerate BLAS (AMX) | wgpu/Metal fallback | No |

*Note: wgpu on Linux can use Vulkan with CUDA backend, but this is transparent to the application. The code doesn't use CUDA directly.

### CUDA GPUDirect Architecture

```rust
// cuda_bridge.rs - Only compiled with feature flag
#![cfg(all(target_os = "linux", feature = "cuda_gds"))]

pub struct CudaIntegrator {
    device_ptr: CUdeviceptr,
    file_handle: CUfileHandle,
    buffer_size: usize,
}

// Provides DMA: NVMe → GPU VRAM
pub fn async_read_to_gpu(&mut self, file_offset: u64, size: usize)
```

### Feature Flag Configuration

```toml
# Cargo.toml
[features]
cuda_gds = []  # Opt-in feature for Linux with NVIDIA hardware
```

### Build Configuration

```rust
// build.rs
#[cfg(all(target_os = "linux", feature = "cuda_gds"))]
// Link with libcufile and libcuda
```

## Should CUDA Compute Be Added?

### Pros
- Direct CUDA kernels could be faster than wgpu/Vulkan
- Better integration with NVIDIA ecosystem (TensorRT, cuDNN)
- Access to NVIDIA-specific optimizations

### Cons
- Breaks cross-platform wgpu abstraction
- Requires CUDA toolkit and NVIDIA hardware
- Increases maintenance burden
- wgpu/Vulkan already works (just needs mock pipeline fix)
- DirectML on Windows is already performant

### Recommendation

**Do NOT add CUDA compute for LLM inference**. Reasons:

1. **wgpu is sufficient**: Vulkan backend on Linux can use CUDA transparently
2. **Cross-platform**: wgpu works on all platforms without platform-specific code
3. **Mock pipeline is the real issue**: Fixing Task 006 will enable real inference on Linux
4. **CUDA GDS already exists**: For calculus operations where direct I/O matters
5. **Maintenance burden**: Adding CUDA compute increases complexity

### Alternative: Use CUDA Backend for wgpu

If NVIDIA-specific performance is needed:
- Configure wgpu to use Vulkan with CUDA backend on Linux
- This is transparent to the application code
- No need for direct CUDA FFI
- Maintains cross-platform abstraction

## Related Files
- `crates/qualia-core-db/src/modalities/calculus/cuda_bridge.rs` (CUDA GDS)
- `crates/qualia-core-db/src/modalities/calculus/gpu.rs` (GPU integration)
- `crates/qualia-core-db/src/gguf_bridge.rs` (LLM inference - uses wgpu, not CUDA)
- `crates/qualia-core-db/Cargo.toml` (feature flags)

## Conclusion

**CUDA support exists but is for GPUDirect Storage (I/O), not compute.**

The LLM inference uses wgpu (Vulkan/Metal/WebGPU) and DirectML, not CUDA directly. The mock pipeline issue (Task 006) is the real blocker for Linux inference, not lack of CUDA support.

If needed, wgpu can be configured to use Vulkan with CUDA backend on Linux for NVIDIA-specific performance without adding direct CUDA FFI to the codebase.

## Status
- ✅ CUDA GPUDirect Storage: Implemented (calculus only, opt-in)
- ❌ CUDA compute kernels: Not implemented (not recommended)
- ⚠️ Linux LLM inference: Blocked by mock pipeline (Task 006), not CUDA