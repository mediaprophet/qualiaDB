//! Backend capability declarations.
//!
//! A capability record declares the exact model architectures, quantization formats,
//! KV-cache encodings, maximum batch sizes, and graph acceleration modes supported by
//! a given prepared backend.

use super::decode_plan::PreparedBackend;

/// Supported neural model architecture families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SupportedArchitecture {
    Llama,
    Mistral,
    Qwen2,
    Qwen3,
    Qwen3_5MoE,
    DeepSeek,
    Gemma,
    Phi,
    DenseTransformer,
}

/// Supported KV cache encoding formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KvEncoding {
    F32Dense,
    F16Dense,
    Q8_0Dense,
    PagedF32,
    PagedF16,
    PagedQ8_0,
}

/// Explicit capability declaration for a prepared backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendCapabilities {
    pub backend: PreparedBackend,
    pub max_batch_size: u32,
    pub supports_paged_attention: bool,
    pub supports_cuda_graphs: bool,
    pub supports_directml_graphs: bool,
    pub supported_architectures: &'static [SupportedArchitecture],
    pub supported_quants: &'static [&'static str],
    pub supported_kv_encodings: &'static [KvEncoding],
}

impl BackendCapabilities {
    pub const fn cpu_reference() -> Self {
        Self {
            backend: PreparedBackend::Cpu,
            max_batch_size: 1,
            supports_paged_attention: true,
            supports_cuda_graphs: false,
            supports_directml_graphs: false,
            supported_architectures: &[
                SupportedArchitecture::Llama,
                SupportedArchitecture::Qwen2,
                SupportedArchitecture::Qwen3,
                SupportedArchitecture::DenseTransformer,
            ],
            supported_quants: &["F32", "F16", "Q8_0", "Q4_K_M", "Q4_0"],
            supported_kv_encodings: &[
                KvEncoding::F32Dense,
                KvEncoding::PagedF32,
                KvEncoding::PagedF16,
            ],
        }
    }

    pub const fn cuda_reference() -> Self {
        Self {
            backend: PreparedBackend::Cuda,
            max_batch_size: 16,
            supports_paged_attention: true,
            supports_cuda_graphs: true,
            supports_directml_graphs: false,
            supported_architectures: &[
                SupportedArchitecture::Llama,
                SupportedArchitecture::Qwen2,
                SupportedArchitecture::Qwen3,
                SupportedArchitecture::DenseTransformer,
            ],
            supported_quants: &["F16", "Q8_0", "Q4_K_M"],
            supported_kv_encodings: &[
                KvEncoding::F16Dense,
                KvEncoding::PagedF16,
                KvEncoding::PagedQ8_0,
            ],
        }
    }

    pub fn is_architecture_supported(&self, arch: SupportedArchitecture) -> bool {
        self.supported_architectures.contains(&arch)
    }

    pub fn is_quant_supported(&self, quant: &str) -> bool {
        self.supported_quants
            .iter()
            .any(|&q| q.eq_ignore_ascii_case(quant))
    }

    pub fn is_kv_encoding_supported(&self, enc: KvEncoding) -> bool {
        self.supported_kv_encodings.contains(&enc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_capabilities() {
        let caps = BackendCapabilities::cpu_reference();
        assert!(caps.is_architecture_supported(SupportedArchitecture::Llama));
        assert!(caps.is_quant_supported("q4_k_m"));
        assert!(caps.is_kv_encoding_supported(KvEncoding::PagedF32));
        assert!(!caps.supports_cuda_graphs);
        assert!(!caps.is_quant_supported("NVFP4"));
    }

    #[test]
    fn test_cuda_capabilities() {
        let caps = BackendCapabilities::cuda_reference();
        assert!(caps.is_architecture_supported(SupportedArchitecture::Llama));
        assert!(caps.is_quant_supported("q4_k_m"));
        assert!(caps.is_kv_encoding_supported(KvEncoding::PagedQ8_0));
        assert!(caps.supports_cuda_graphs);
        assert_eq!(caps.max_batch_size, 16);
        assert!(!caps.is_quant_supported("NVFP4"));
        assert!(!caps.is_quant_supported("FP8"));
        assert!(!caps.is_architecture_supported(SupportedArchitecture::Qwen3_5MoE));
    }
}
