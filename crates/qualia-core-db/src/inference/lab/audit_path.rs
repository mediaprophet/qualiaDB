//! Hot-path wiring auditor — makes unfinished integration visible.

use crate::inference_modes::{
    active_inference_mode, post_turn_verify_enabled, prefer_tensor_core_gemm,
    quant_graph_grounding_enabled, sentinel_mid_decode_enabled,
};
use crate::llm_bench::{
    attention_o_fuse_enabled, attention_preproject_enabled, coop_gemv_enabled, ffn_fusion_enabled,
    ffn_fusion_in_resident, kv_int8_enabled, resident_decode_enabled, resident_prefill_enabled,
    resident_weights_enabled,
};

#[derive(Debug, Clone)]
pub struct HotPathAudit {
    pub resident_decode: bool,
    pub resident_prefill: bool,
    pub resident_weights: bool,
    pub coop_gemv: bool,
    pub ffn_fusion_flag: bool,
    pub ffn_fusion_in_resident_decode: bool,
    pub kv_int8: bool,
    pub attention_preproject: bool,
    pub attention_o_fuse: bool,
    pub prefer_cuda_gemm: bool,
    pub cuda_caps: bool,
    pub mode: String,
    pub post_turn_verify: bool,
    pub sentinel_mid: bool,
    pub quant_graph: bool,
    pub timestamps_supported: bool,
    pub gpu_profile_env: bool,
    pub notes: Vec<String>,
}

/// Static/dynamic audit of claimed features vs what decode is configured to use.
pub fn audit_hot_path() -> HotPathAudit {
    let mut notes = Vec::new();
    let ffn_flag = ffn_fusion_enabled();
    // Set true when a resident plan was built with fused_ffn bind groups (T-A1).
    // False until first successful plan build, or when quant is Q4_K_SOA/F16 (fallback).
    let ffn_in_resident = ffn_fusion_in_resident();
    if ffn_flag && !ffn_in_resident {
        notes.push(
            "ffn_fusion flag ON but last resident plan did not fuse (unsupported quant or plan not built yet) — T-A1 partial"
                .into(),
        );
    }
    if ffn_in_resident {
        notes.push(
            "T-A1/T-A1b: fused_ffn in resident (coop entry when QUALIA_LLM_COOP_GEMV on)".into(),
        );
    }

    let cuda_caps = crate::wgsl_forge::dispatch::caps().cuda;
    if prefer_tensor_core_gemm() && !cuda_caps {
        notes.push("mode prefers CUDA GEMM but cuda caps=false".into());
    }
    if prefer_tensor_core_gemm() && cuda_caps {
        notes.push(
            "CUDA lane available for densify/Q4 GEMV; full hidden-on-device layer stack still open"
                .into(),
        );
    }

    let timestamps_supported = crate::gpu_context::shared_gpu().timestamps_supported;
    let gpu_profile_env = std::env::var("QUALIA_LLM_GPU_PROFILE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !resident_decode_enabled() {
        notes.push("resident_decode OFF — legacy multi-fence path".into());
    }
    if !coop_gemv_enabled() {
        notes.push("coop_gemv OFF — naive GEMV".into());
    }

    HotPathAudit {
        resident_decode: resident_decode_enabled(),
        resident_prefill: resident_prefill_enabled(),
        resident_weights: resident_weights_enabled(),
        coop_gemv: coop_gemv_enabled(),
        ffn_fusion_flag: ffn_flag,
        ffn_fusion_in_resident_decode: ffn_in_resident,
        kv_int8: kv_int8_enabled(),
        attention_preproject: attention_preproject_enabled(),
        attention_o_fuse: attention_o_fuse_enabled(),
        prefer_cuda_gemm: prefer_tensor_core_gemm(),
        cuda_caps,
        mode: active_inference_mode().as_str().to_string(),
        post_turn_verify: post_turn_verify_enabled(),
        sentinel_mid: sentinel_mid_decode_enabled(),
        quant_graph: quant_graph_grounding_enabled(),
        timestamps_supported,
        gpu_profile_env,
        notes,
    }
}

impl HotPathAudit {
    pub fn format_report(&self) -> String {
        let yn = |b: bool| if b { "yes" } else { "NO " };
        let mut s = String::from("Hot-path wiring audit\n");
        s.push_str(&format!("  mode:                    {}\n", self.mode));
        s.push_str(&format!(
            "  resident_decode:         {}\n",
            yn(self.resident_decode)
        ));
        s.push_str(&format!(
            "  resident_prefill:        {}\n",
            yn(self.resident_prefill)
        ));
        s.push_str(&format!(
            "  resident_weights:        {}\n",
            yn(self.resident_weights)
        ));
        s.push_str(&format!(
            "  coop_gemv:               {}\n",
            yn(self.coop_gemv)
        ));
        s.push_str(&format!(
            "  ffn_fusion flag:         {}\n",
            yn(self.ffn_fusion_flag)
        ));
        s.push_str(&format!(
            "  ffn_fusion in resident:  {}  (must be yes for T-A1 done)\n",
            yn(self.ffn_fusion_in_resident_decode)
        ));
        s.push_str(&format!(
            "  kv_int8:                 {}\n",
            yn(self.kv_int8)
        ));
        s.push_str(&format!(
            "  attention_preproject:    {}\n",
            yn(self.attention_preproject)
        ));
        s.push_str(&format!(
            "  attention_o_fuse:        {}\n",
            yn(self.attention_o_fuse)
        ));
        s.push_str(&format!(
            "  prefer_cuda_gemm:        {}\n",
            yn(self.prefer_cuda_gemm)
        ));
        s.push_str(&format!(
            "  cuda_caps:               {}\n",
            yn(self.cuda_caps)
        ));
        s.push_str(&format!(
            "  post_turn_verify:        {}\n",
            yn(self.post_turn_verify)
        ));
        s.push_str(&format!(
            "  sentinel_mid:            {}\n",
            yn(self.sentinel_mid)
        ));
        s.push_str(&format!(
            "  quant_graph:             {}\n",
            yn(self.quant_graph)
        ));
        s.push_str(&format!(
            "  timestamps_supported:    {}\n",
            yn(self.timestamps_supported)
        ));
        s.push_str(&format!(
            "  QUALIA_LLM_GPU_PROFILE:  {}\n",
            yn(self.gpu_profile_env)
        ));
        if !self.notes.is_empty() {
            s.push_str("  notes:\n");
            for n in &self.notes {
                s.push_str(&format!("    - {n}\n"));
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audit_runs() {
        let a = audit_hot_path();
        let r = a.format_report();
        assert!(r.contains("resident_decode"));
        assert!(r.contains("ffn_fusion in resident"));
    }
}
