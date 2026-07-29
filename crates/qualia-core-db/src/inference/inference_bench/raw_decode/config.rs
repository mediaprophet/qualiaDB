use crate::inference::runtime::{BackendKind, BenchmarkManifest};

#[derive(Debug, Clone)]
pub struct RawDecodeConfig {
    pub label: String,
    pub model_path: String,
    pub quantization: String,
    pub prompt: String,
    /// Cold benchmark-only prompt shaping: repeat encoded prompt ids to this exact length.
    pub target_prompt_tokens: Option<u32>,
    pub decode_steps: u32,
    pub warmup_runs: u16,
    pub measured_runs: u16,
    /// `Unknown` means "use the selected wgpu adapter and record its concrete backend".
    pub requested_backend: BackendKind,
}

impl RawDecodeConfig {
    pub fn new(
        label: impl Into<String>,
        model_path: impl Into<String>,
        quantization: impl Into<String>,
        prompt: impl Into<String>,
    ) -> Self {
        Self {
            label: label.into(),
            model_path: model_path.into(),
            quantization: quantization.into(),
            prompt: prompt.into(),
            target_prompt_tokens: None,
            decode_steps: 256,
            warmup_runs: 1,
            measured_runs: 5,
            requested_backend: BackendKind::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RawDecodeResult {
    pub manifest: BenchmarkManifest,
    pub run_tok_s: Vec<f64>,
    pub step_latency_ms: Vec<f64>,
    pub generated_token_ids: Vec<u32>,
    /// Exact rendered bytes for each generated token, used for token-boundary parity.
    pub generated_token_bytes: Vec<Vec<u8>>,
    pub generated_text: String,
}
