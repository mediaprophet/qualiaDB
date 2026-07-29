use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CudaQ8Profile {
    Incumbent,
    CustomEnvironment,
    A2000SmolLm2Q8V1,
}

impl CudaQ8Profile {
    pub(crate) const fn receipt_id(self) -> &'static str {
        match self {
            Self::Incumbent => "cuda-q8-incumbent-v1",
            Self::CustomEnvironment => "cuda-q8-custom-env-v1",
            Self::A2000SmolLm2Q8V1 => "cuda-q8-a2000-smollm2-q8-v1",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CudaQ8Tuning {
    pub profile: CudaQ8Profile,
    pub stage_debug: bool,
    pub dp4a_swiglu: bool,
    pub dp4a_swiglu_layers: usize,
    pub dp4a_qkv: bool,
    pub dp4a_down_projection: bool,
    pub dp4a_o_projection: bool,
    pub dp4a_lm_head: bool,
}

impl CudaQ8Tuning {
    pub(crate) fn from_env() -> Self {
        let stage_debug = std::env::var_os("QUALIA_CUDA_STAGE_DEBUG").is_some();
        if std::env::var("QUALIA_CUDA_Q8_PROFILE").ok().as_deref() == Some("a2000-smollm2-q8-v1") {
            return Self::a2000_smollm2_q8_v1(stage_debug);
        }
        let dp4a_down_projection = env_enabled("QUALIA_CUDA_Q8_DP4A_RESID");
        let dp4a_swiglu = env_enabled("QUALIA_CUDA_Q8_DP4A_SWIGLU");
        let dp4a_qkv = env_enabled("QUALIA_CUDA_Q8_DP4A_QKV");
        let dp4a_lm_head = env_enabled("QUALIA_CUDA_Q8_DP4A_LM_HEAD");
        let custom = dp4a_swiglu || dp4a_qkv || dp4a_down_projection || dp4a_lm_head;
        Self {
            profile: if custom {
                CudaQ8Profile::CustomEnvironment
            } else {
                CudaQ8Profile::Incumbent
            },
            stage_debug,
            dp4a_swiglu,
            dp4a_swiglu_layers: std::env::var("QUALIA_CUDA_Q8_DP4A_SWIGLU_LAYERS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(usize::MAX),
            dp4a_qkv,
            dp4a_down_projection,
            dp4a_o_projection: dp4a_down_projection && !env_enabled("QUALIA_CUDA_Q8_DP4A_SKIP_O"),
            dp4a_lm_head,
        }
    }

    pub(crate) const fn a2000_smollm2_q8_v1(stage_debug: bool) -> Self {
        Self {
            profile: CudaQ8Profile::A2000SmolLm2Q8V1,
            stage_debug,
            dp4a_swiglu: true,
            dp4a_swiglu_layers: 31,
            dp4a_qkv: true,
            dp4a_down_projection: true,
            dp4a_o_projection: true,
            dp4a_lm_head: true,
        }
    }

    pub(crate) fn for_model(
        self,
        n_embd: usize,
        n_head: usize,
        n_kv: usize,
        head_dim: usize,
        n_layer: u32,
        lm_head_out: usize,
    ) -> Self {
        if self.profile != CudaQ8Profile::A2000SmolLm2Q8V1
            || is_smollm2_360m_shape(n_embd, n_head, n_kv, head_dim, n_layer, lm_head_out)
        {
            return self;
        }
        Self {
            stage_debug: self.stage_debug,
            ..Self::incumbent()
        }
    }

    pub(crate) fn for_model_on_adapter(
        self,
        n_embd: usize,
        n_head: usize,
        n_kv: usize,
        head_dim: usize,
        n_layer: u32,
        lm_head_out: usize,
        adapter_vendor: u32,
        adapter_device: u32,
        adapter_name: &str,
    ) -> Self {
        let model_tuning = self.for_model(n_embd, n_head, n_kv, head_dim, n_layer, lm_head_out);
        if model_tuning.profile != CudaQ8Profile::Incumbent
            || !is_smollm2_360m_shape(n_embd, n_head, n_kv, head_dim, n_layer, lm_head_out)
            || adapter_vendor != 0x10de
            || adapter_device != 0x2571
            || !contains_ascii_case_insensitive(adapter_name, "rtx a2000")
        {
            return model_tuning;
        }
        Self::a2000_smollm2_q8_v1(model_tuning.stage_debug)
    }

    pub(crate) const fn incumbent() -> Self {
        Self {
            profile: CudaQ8Profile::Incumbent,
            stage_debug: false,
            dp4a_swiglu: false,
            dp4a_swiglu_layers: 0,
            dp4a_qkv: false,
            dp4a_down_projection: false,
            dp4a_o_projection: false,
            dp4a_lm_head: false,
        }
    }

    pub(crate) const fn receipt_id(self) -> &'static str {
        self.profile.receipt_id()
    }

    #[inline]
    pub(crate) fn dp4a_swiglu_layer(self, layer: usize) -> bool {
        self.dp4a_swiglu && layer < self.dp4a_swiglu_layers
    }

    pub(crate) fn graph_fingerprint(self) -> u64 {
        let mut fingerprint = 0x5138_5455_4e45_5631u64;
        for value in [
            self.profile as u64,
            self.dp4a_swiglu as u64,
            self.dp4a_swiglu_layers as u64,
            self.dp4a_qkv as u64,
            self.dp4a_down_projection as u64,
            self.dp4a_o_projection as u64,
            self.dp4a_lm_head as u64,
        ] {
            fingerprint ^= value.wrapping_add(0x9e37_79b9_7f4a_7c15);
            fingerprint = fingerprint.rotate_left(13).wrapping_mul(0x100_0000_01b3);
        }
        fingerprint
    }

    #[cfg(test)]
    pub(crate) fn q8_graph_nodes(self, layer_count: u32) -> u64 {
        let layers = layer_count as u64;
        layers * 8
            + if self.dp4a_swiglu {
                self.dp4a_swiglu_layers.min(layer_count as usize) as u64
            } else {
                0
            }
            + layers * u64::from(self.dp4a_qkv)
            + layers * u64::from(self.dp4a_down_projection)
            + layers * u64::from(self.dp4a_o_projection)
            + u64::from(self.dp4a_lm_head)
            + 3
    }
}

fn env_enabled(name: &str) -> bool {
    std::env::var(name)
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "on"))
}

const fn is_smollm2_360m_shape(
    n_embd: usize,
    n_head: usize,
    n_kv: usize,
    head_dim: usize,
    n_layer: u32,
    lm_head_out: usize,
) -> bool {
    n_embd == 960
        && n_head == 15
        && n_kv == 5
        && head_dim == 64
        && n_layer == 32
        && lm_head_out == 49_152
}

fn contains_ascii_case_insensitive(haystack: &str, needle: &str) -> bool {
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|candidate| candidate.eq_ignore_ascii_case(needle.as_bytes()))
}

pub(crate) fn cuda_q8_tuning() -> &'static CudaQ8Tuning {
    static CONFIG: OnceLock<CudaQ8Tuning> = OnceLock::new();
    CONFIG.get_or_init(CudaQ8Tuning::from_env)
}

pub(crate) fn cuda_q8_tuning_for_model(
    n_embd: usize,
    n_head: usize,
    n_kv: usize,
    head_dim: usize,
    n_layer: u32,
    lm_head_out: usize,
) -> CudaQ8Tuning {
    let adapter = &crate::gpu_context::shared_gpu().adapter_caps;
    cuda_q8_tuning().for_model_on_adapter(
        n_embd,
        n_head,
        n_kv,
        head_dim,
        n_layer,
        lm_head_out,
        adapter.vendor,
        adapter.device,
        &adapter.name,
    )
}
