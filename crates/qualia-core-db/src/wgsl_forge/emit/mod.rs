pub mod coopmat;
pub mod cuda_c;
pub mod cuda_c_fused;
pub mod cuda_graph;
pub mod df64;
pub mod dxc;
pub mod dxc_cache;
pub mod graph_hlsl;
pub mod graph_msl;
pub mod hlsl;
pub mod hlsl_wave;
pub mod msl;
pub mod ptx;
pub mod spirv;
pub mod wgsl;

use serde::{Deserialize, Serialize};

use super::{ForgeError, KernelSpec, Schedule};
pub use coopmat::{matmul_tc_wgsl, matmul_tc_wgsl_tiled, MATMUL_TC_TILED_ENTRY};
pub use cuda_c::emit_cuda_c;
pub use cuda_graph::{emit_graph_cuda_c, graph_cuda_entry, CudaCLowerer};
pub use df64::{GEMM_DF64_ENTRY, GEMM_DF64_WGSL};
pub use dxc::compile_hlsl_to_spirv;
pub use dxc_cache::{clear_dxc_cache, compile_hlsl_to_spirv_cached, dxc_cache_len};
pub use graph_hlsl::{conv2d_hlsl, emit_graph_hlsl, HlslLowerer};
pub use graph_msl::{conv2d_msl, emit_graph_msl, MslLowerer};
pub use hlsl::emit_hlsl;
pub use msl::emit_msl;
pub use ptx::emit_ptx;
pub use spirv::{decode_spirv_words, emit_spirv, emit_spirv_patched, patch_spirv_workgroup_size};
pub use wgsl::{emit_graph_wgsl, emit_wgsl};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetBackend {
    Wgsl,
    Msl,
    Hlsl,
    Ptx,
    /// CUDA-C compiled to PTX by NVRTC at runtime (mirrors HLSL -> DXC).
    CudaC,
    /// Binary SPIR-V, produced from the generated WGSL via naga's `spv-out`
    /// backend. The words are stored in `GeneratedShader::source` as a
    /// `;`-joined decimal string (see [`spirv`]).
    Spirv,
}

impl Default for TargetBackend {
    fn default() -> Self {
        Self::Wgsl
    }
}

impl std::str::FromStr for TargetBackend {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "wgsl" => Ok(Self::Wgsl),
            "msl" => Ok(Self::Msl),
            "hlsl" => Ok(Self::Hlsl),
            "ptx" => Ok(Self::Ptx),
            "cuda" | "cuda-c" | "cuda_c" => Ok(Self::CudaC),
            "spirv" | "spir-v" | "spv" => Ok(Self::Spirv),
            _ => Err(format!("unknown target backend: {}", s)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeneratedShader {
    pub kernel_id: String,
    pub semantic_hash: String,
    pub source_hash: String,
    pub schedule: Schedule,
    pub source: String,
}

pub fn emit_shader(
    kernel: &KernelSpec,
    schedule: Schedule,
    target: TargetBackend,
) -> Result<GeneratedShader, ForgeError> {
    match target {
        TargetBackend::Wgsl => emit_wgsl(kernel, schedule),
        TargetBackend::Msl => emit_msl(kernel, schedule),
        TargetBackend::Hlsl => emit_hlsl(kernel, schedule),
        TargetBackend::Ptx => emit_ptx(kernel, schedule),
        TargetBackend::CudaC => emit_cuda_c(kernel, schedule),
        TargetBackend::Spirv => emit_spirv(kernel, schedule),
    }
}
