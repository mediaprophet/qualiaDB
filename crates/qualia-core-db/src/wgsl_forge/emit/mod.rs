pub mod wgsl;
pub mod hlsl;
pub mod msl;
pub mod ptx;
pub mod cuda_c;
pub mod dxc;

use serde::{Deserialize, Serialize};

use super::{ForgeError, KernelSpec, Schedule};
pub use wgsl::emit_wgsl;
pub use msl::emit_msl;
pub use hlsl::emit_hlsl;
pub use ptx::emit_ptx;
pub use cuda_c::emit_cuda_c;
pub use dxc::compile_hlsl_to_spirv;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetBackend {
    Wgsl,
    Msl,
    Hlsl,
    Ptx,
    /// CUDA-C compiled to PTX by NVRTC at runtime (mirrors HLSL -> DXC).
    CudaC,
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
    }
}
