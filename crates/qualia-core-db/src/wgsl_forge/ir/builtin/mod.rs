use crate::wgsl_forge::ForgeError;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

pub(crate) mod iq;
mod specs;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltinKernel {
    AffineF32,
    FusedFfn,
    P64Project,
    TopK,
    RayProbe,
    TernaryGemv,
    Gemm,
    Gemv,
    Fft,
    IqPowerF32,
    IqPowerI16,
}

impl BuiltinKernel {
    pub const ALL: [Self; 11] = [
        Self::AffineF32,
        Self::FusedFfn,
        Self::P64Project,
        Self::TopK,
        Self::RayProbe,
        Self::TernaryGemv,
        Self::Gemm,
        Self::Gemv,
        Self::Fft,
        Self::IqPowerF32,
        Self::IqPowerI16,
    ];

    /// Display name.
    pub const fn name(self) -> &'static str {
        match self {
            Self::AffineF32 => "affine-f32",
            Self::FusedFfn => "fused-ffn",
            Self::P64Project => "p64-project",
            Self::TopK => "topk",
            Self::RayProbe => "ray-probe",
            Self::TernaryGemv => "ternary-gemv",
            Self::Gemm => "gemm",
            Self::Gemv => "gemv",
            Self::Fft => "fft",
            Self::IqPowerF32 => "iq-power-f32",
            Self::IqPowerI16 => "iq-power-i16",
        }
    }

    /// Whether the differential oracle (`evaluate_builtin`) has a CPU reference +
    /// GPU dispatch wired for this kernel, so it can be certified and tuned on
    /// hardware. IQ kernels are representable and emitted by Etalon, but have no
    /// local differential evaluator yet.
    pub const fn has_gpu_oracle(self) -> bool {
        matches!(
            self,
            Self::AffineF32
                | Self::TopK
                | Self::FusedFfn
                | Self::P64Project
                | Self::RayProbe
                | Self::TernaryGemv
                | Self::Gemm
                | Self::Gemv
                | Self::Fft
        )
    }
}

impl FromStr for BuiltinKernel {
    type Err = ForgeError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "affine-f32" | "affine_f32" | "affine" => Ok(Self::AffineF32),
            "fused-ffn" | "fused_ffn" | "ffn" => Ok(Self::FusedFfn),
            "p64-project" | "p64_project" | "p64" => Ok(Self::P64Project),
            "topk" | "top-k" | "top_k" => Ok(Self::TopK),
            "ray-probe" | "ray_probe" | "rayquery" | "ray_query" | "rt" => Ok(Self::RayProbe),
            "ternary-gemv" | "ternary_gemv" | "ternary" | "bitnet" => Ok(Self::TernaryGemv),
            "gemm" | "matmul" | "dense-gemm" | "dense_gemm" => Ok(Self::Gemm),
            "gemv" | "matvec" => Ok(Self::Gemv),
            "fft" | "dft" | "radix2" | "radix-2" => Ok(Self::Fft),
            "iq-power-f32" | "iq_power_f32" => Ok(Self::IqPowerF32),
            "iq-power-i16" | "iq_power_i16" => Ok(Self::IqPowerI16),
            other => Err(ForgeError::UnknownKernel(other.to_string())),
        }
    }
}
