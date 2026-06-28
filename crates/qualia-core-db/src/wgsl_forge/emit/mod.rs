pub mod wgsl;
pub mod msl;
pub mod hlsl;
pub mod ptx;

use serde::{Deserialize, Serialize};

use super::{ForgeError, KernelSpec, Schedule};
use wgsl::emit_wgsl;
use msl::emit_msl;
use hlsl::emit_hlsl;
use ptx::emit_ptx;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetBackend {
    Wgsl,
    Msl,
    Hlsl,
    Ptx,
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
    }
}
