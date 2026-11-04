pub use etalon_inference_lab::campaign::{
    default_toggle_space, save_campaign_report, CampaignConfig, CampaignReport,
};
pub fn run_optimization_campaign(cfg: &CampaignConfig) -> CampaignReport {
    etalon_inference_lab::campaign::run_optimization_campaign(
        cfg,
        &mut super::experiment::QualiaExecutor,
    )
}
