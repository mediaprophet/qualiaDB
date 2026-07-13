//! Bit-packed uniform parameter blocks (POD) for the forge oracle kernels, plus the
//! numeric comparison tolerance. Pure data types — no GPU or reference logic here.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct AffineParams {
    pub length: u32,
    pub scale: f32,
    pub bias: f32,
    pub _pad: u32,
}

/// 16-byte uniform block for the top-k kernel (`block_size` == workgroup size).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct TopKParams {
    pub length: u32,
    pub k: u32,
    pub block_size: u32,
    pub _pad: u32,
}

/// 16-byte uniform block for the fused-FFN kernel.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct FfnParams {
    pub input_size: u32,
    pub hidden_size: u32,
    pub output_size: u32,
    pub _pad: u32,
}

/// 16-byte uniform block for the ternary-GEMV kernel (`k_words` == ceil(k/16)).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct TernaryGemvParams {
    pub m: u32,
    pub k: u32,
    pub k_words: u32,
    pub _pad: u32,
}

/// 16-byte uniform block for the dense GEMM kernel: row-major `C[M×N] = A[M×K]·B[K×N]`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct GemmParams {
    pub m: u32,
    pub n: u32,
    pub k: u32,
    pub _pad: u32,
}

/// 16-byte uniform block for the dense GEMV kernel: row-major `y[M] = A[M×N]·x[N]`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct GemvParams {
    pub m: u32,
    pub n: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

/// 16-byte uniform block for the radix-2 FFT kernel: `n` complex elements,
/// `log2n = log2(n)`. The kernel runs one workgroup of `n` threads.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct FftParams {
    pub n: u32,
    pub log2n: u32,
    pub _pad0: u32,
    pub _pad1: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OracleTolerance {
    pub absolute: f32,
    pub relative: f32,
}

impl Default for OracleTolerance {
    fn default() -> Self {
        Self {
            absolute: 1.0e-6,
            relative: 1.0e-5,
        }
    }
}
