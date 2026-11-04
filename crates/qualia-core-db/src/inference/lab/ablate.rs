use crate::inference_modes::{set_inference_mode, InferenceMode};
use crate::llm_bench::{set_coop_gemv, set_ffn_fusion, set_kv_int8, set_resident_decode};
pub use etalon_inference_lab::ablate::{format_ablation_report, AblationRow};
use std::path::Path;
struct QualiaAblation;
impl etalon_inference_lab::ablate::AblationExecutor<InferenceMode> for QualiaAblation {
    fn execute(
        &mut self,
        model: &Path,
        tokens: u32,
        resident: bool,
        coop: bool,
        ffn: bool,
        kv: bool,
        mode: InferenceMode,
        forge: Option<&str>,
    ) -> Option<f64> {
        let _state = super::state::TrialState::capture();
        set_resident_decode(resident);
        set_coop_gemv(coop);
        set_ffn_fusion(ffn);
        set_kv_int8(kv);
        set_inference_mode(mode);
        std::env::set_var("QUALIA_INFERENCE_MODE", mode.as_str());
        if let Some(forge) = forge {
            std::env::set_var("QUALIA_FORGE_BACKEND", forge);
        }
        crate::hardware_passport::measure_decode_proxy_tok_s(model, tokens)
    }
    fn passport_key(&self) -> String {
        crate::hardware_passport::read_passport(&crate::hardware_passport::default_cache_path())
            .map(|p| p.key)
            .unwrap_or_default()
    }
    fn backend_label(&self) -> String {
        std::env::var("QUALIA_WGPU_BACKEND").unwrap_or_else(|_| "auto".into())
    }
}
pub fn run_ablation_matrix(model: &Path, tokens: u32, csv_out: Option<&Path>) -> Vec<AblationRow> {
    etalon_inference_lab::ablate::run_ablation_matrix(model, tokens, csv_out, &mut QualiaAblation)
}
