//! Toolbox registration — populates the Registry at startup.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Registers all 15 toolboxes with their tool-chains and tools,
//! plus all manifold seeds, into a shared Registry.
//!
//! Split into per-toolbox modules so MCP-sized commits can land (G-POET-TOOLCHEST).

pub(super) use crate::tool_chest::core::intent_bus::ActionType;
pub(super) use crate::tool_chest::core::registry::Registry;
pub(super) use crate::tool_chest::core::tool::{SimpleTool, ToolKind, ToolMetadata};
pub(super) use crate::tool_chest::core::tool_chain::{ToolChain, ToolChainMetadata};
pub(super) use crate::tool_chest::core::toolbox::{Toolbox, ToolboxMetadata};
pub(super) use crate::tool_chest::manifolds;

mod register_ai_toolbox;
mod register_ai_nlp;
mod register_audio_toolbox;
mod register_code_toolbox;
mod register_communication_toolbox;
mod register_compact_toolbox;
mod register_epistemic_toolbox;
mod register_research_live;
mod register_erp_toolbox;
mod register_econ_toolbox;
mod register_health_toolbox;
mod register_image_toolbox;
mod register_render_live;
mod register_wave33_live;
mod register_gpu_live;
mod register_wave36_live;
mod register_wave37_live;
mod register_wave38_live;
mod register_wave39_live;
mod register_mail_toolbox;
mod register_office_toolbox;
mod register_rights_toolbox;
mod register_scientific_toolbox;
mod register_cg_live;
mod register_sdn_toolbox;
mod register_sheet_toolbox;
mod register_spatial_toolbox;

use register_ai_toolbox::register_ai_toolbox;
use register_audio_toolbox::register_audio_toolbox;
use register_code_toolbox::register_code_toolbox;
use register_communication_toolbox::register_communication_toolbox;
use register_compact_toolbox::register_compact_toolbox;
use register_epistemic_toolbox::register_epistemic_toolbox;
use register_erp_toolbox::register_erp_toolbox;
use register_econ_toolbox::register_econ_toolbox;
use register_health_toolbox::register_health_toolbox;
use register_image_toolbox::register_image_toolbox;
use register_mail_toolbox::register_mail_toolbox;
use register_office_toolbox::register_office_toolbox;
use register_rights_toolbox::register_rights_toolbox;
use register_scientific_toolbox::register_scientific_toolbox;
use register_sdn_toolbox::register_sdn_toolbox;
use register_sheet_toolbox::register_sheet_toolbox;
use register_spatial_toolbox::register_spatial_toolbox;

/// Live ALL_BOUND id for placing a container on the manifold.
pub(super) const SCOPE_PLACE: &str = "Poet.container_place";

/// Shared by compact toolbox helpers (`register_*` siblings).
pub(super) struct CompactTool {
    pub(super) id: &'static str,
    pub(super) label: &'static str,
    pub(super) icon: &'static str,
    pub(super) kind: ToolKind,
    pub(super) action: ActionType,
    pub(super) description: &'static str,
}

/// Build a fully populated Registry with all toolboxes and manifold seeds.
pub fn build_registry() -> Registry {
    let mut reg = Registry::new();

    register_epistemic_toolbox(&mut reg);
    register_office_toolbox(&mut reg);
    register_image_toolbox(&mut reg);
    register_sheet_toolbox(&mut reg);
    register_spatial_toolbox(&mut reg);
    register_audio_toolbox(&mut reg);
    register_communication_toolbox(&mut reg);
    register_erp_toolbox(&mut reg);
    register_econ_toolbox(&mut reg);
    register_mail_toolbox(&mut reg);
    register_scientific_toolbox(&mut reg);
    register_rights_toolbox(&mut reg);
    register_health_toolbox(&mut reg);
    register_code_toolbox(&mut reg);
    register_ai_toolbox(&mut reg);
    register_sdn_toolbox(&mut reg);
    crate::browser::spec_tools::register_all(&mut reg);

    for seed in manifolds::all_seeds() {
        reg.register_manifold(seed);
    }

    for construct in crate::tool_chest::constructs::all_constructs() {
        reg.register_construct(construct);
    }

    reg
}

#[cfg(test)]
mod tests {
    use super::SCOPE_PLACE;
    use crate::tool_chest::core::tool::ToolKind;

    fn is_legacy_scope(scope: &str) -> bool {
        scope.starts_with("graph:")
            || scope.starts_with("capability:")
            || scope.starts_with("vibe:")
            || scope.starts_with("pulse:")
            || scope.starts_with("aura:")
            || scope.starts_with("ui:")
            || scope.starts_with("intent:")
    }

    #[test]
    fn capability_scopes_are_live_family_method_or_local() {
        let registry = super::build_registry();
        for toolbox in registry.toolboxes() {
            for chain in toolbox.chains() {
                for tool in chain.tools() {
                    let meta = tool.metadata();
                    if let Some(scope) = meta.capability_scope.as_deref() {
                        assert!(
                            scope.contains('.'),
                            "{}: capability_scope `{scope}` must be Family.method",
                            meta.id
                        );
                        assert!(
                            !is_legacy_scope(scope),
                            "{}: legacy scope `{scope}` is not a live ALL_BOUND id",
                            meta.id
                        );
                    }
                    if meta.kind == ToolKind::PlaceContainer {
                        assert_eq!(
                            meta.capability_scope.as_deref(),
                            Some(SCOPE_PLACE),
                            "{}: PlaceContainer must cite {SCOPE_PLACE}",
                            meta.id
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn office_shapes_chain_binds_live_n3_and_shacl() {
        let registry = super::build_registry();
        let office = registry.toolbox("office").expect("office toolbox");
        let chain = office
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "office:shapes")
            .expect("office:shapes toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("n3:evaluate", Some("N3Logic.evaluate"))));
        assert!(tools.contains(&("shacl:validate", Some("SHACL.validate"))));
    }

    #[test]
    fn sheet_grid_binds_extended_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("sheet:stats_sum", Some("Statistics.sum"))));
        assert!(tools.contains(&("sheet:stats_skewness", Some("Statistics.skewness"))));
        assert!(tools.contains(&("sheet:stats_kurtosis", Some("Statistics.kurtosis"))));
        assert!(tools.contains(&("sheet:stats_quantile", Some("Statistics.quantile"))));
        assert!(tools.contains(&("sheet:stats_iqr", Some("Statistics.iqr"))));
        assert!(tools.contains(&("sheet:stats_mode", Some("Statistics.mode"))));
        assert!(tools.contains(&(
            "sheet:stats_trimmed_mean",
            Some("Statistics.trimmed_mean")
        )));
        assert!(tools.contains(&(
            "sheet:stats_mad",
            Some("Statistics.median_abs_deviation")
        )));
        assert!(tools.contains(&("sheet:stats_pearson", Some("Statistics.pearson"))));
        assert!(tools.contains(&("sheet:stats_covariance", Some("Statistics.covariance"))));
        assert!(tools.contains(&(
            "sheet:stats_z_score_outliers",
            Some("Statistics.z_score_outliers")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave2_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("sheet:stats_argmax", Some("Statistics.argmax"))));
        assert!(tools.contains(&(
            "sheet:stats_binomial_pmf",
            Some("Statistics.binomial_pmf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_binomial_cdf",
            Some("Statistics.binomial_cdf")
        )));
        assert!(tools.contains(&("sheet:stats_beta_pdf", Some("Statistics.beta_pdf"))));
        assert!(tools.contains(&(
            "sheet:stats_chi_squared_pdf",
            Some("Statistics.chi_squared_pdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_chi_squared_cdf",
            Some("Statistics.chi_squared_cdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_chi_squared_quantile",
            Some("Statistics.chi_squared_quantile")
        )));
        assert!(tools.contains(&(
            "sheet:stats_autocorrelation",
            Some("Statistics.autocorrelation")
        )));
        assert!(tools.contains(&(
            "sheet:stats_bootstrap_means",
            Some("Statistics.bootstrap_means")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave3_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "sheet:stats_normal_pdf",
            Some("Statistics.normal_pdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_normal_cdf",
            Some("Statistics.normal_cdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_normal_quantile",
            Some("Statistics.normal_quantile")
        )));
        assert!(tools.contains(&(
            "sheet:stats_standard_normal_cdf",
            Some("Statistics.standard_normal_cdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_poisson_pmf",
            Some("Statistics.poisson_pmf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_poisson_cdf",
            Some("Statistics.poisson_cdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_exponential_pdf",
            Some("Statistics.exponential_pdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_exponential_cdf",
            Some("Statistics.exponential_cdf")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave4_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("sheet:stats_spearman", Some("Statistics.spearman"))));
        assert!(tools.contains(&("sheet:stats_kendall", Some("Statistics.kendall"))));
        assert!(tools.contains(&(
            "sheet:stats_winsorized_mean",
            Some("Statistics.winsorized_mean")
        )));
        assert!(tools.contains(&("sheet:stats_erf", Some("Statistics.erf"))));
        assert!(tools.contains(&("sheet:stats_erfc", Some("Statistics.erfc"))));
        assert!(tools.contains(&(
            "sheet:stats_uniform_pdf",
            Some("Statistics.uniform_pdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_laplace_pdf",
            Some("Statistics.laplace_pdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_standard_pdf",
            Some("Statistics.standard_pdf")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave5_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "sheet:stats_uniform_cdf",
            Some("Statistics.uniform_cdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_laplace_cdf",
            Some("Statistics.laplace_cdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_lognormal_pdf",
            Some("Statistics.lognormal_pdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_lognormal_cdf",
            Some("Statistics.lognormal_cdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_standard_quantile",
            Some("Statistics.standard_quantile")
        )));
        assert!(tools.contains(&(
            "sheet:stats_ln_gamma",
            Some("Statistics.ln_gamma")
        )));
        assert!(tools.contains(&(
            "sheet:stats_gamma_fn",
            Some("Statistics.gamma_fn")
        )));
        assert!(tools.contains(&(
            "sheet:stats_weibull_pdf",
            Some("Statistics.weibull_pdf")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave6_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "sheet:stats_gamma_pdf",
            Some("Statistics.gamma_pdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_two_sided_p",
            Some("Statistics.two_sided_p")
        )));
        assert!(tools.contains(&(
            "sheet:stats_chi_squared_upper_p",
            Some("Statistics.chi_squared_upper_p")
        )));
        assert!(tools.contains(&(
            "sheet:stats_students_t_pdf",
            Some("Statistics.students_t_pdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_fisher_f_pdf",
            Some("Statistics.fisher_f_pdf")
        )));
        assert!(tools.contains(&("sheet:stats_gammp", Some("Statistics.gammp"))));
        assert!(tools.contains(&("sheet:stats_gammq", Some("Statistics.gammq"))));
        assert!(tools.contains(&("sheet:stats_betai", Some("Statistics.betai"))));
    }

    #[test]
    fn sheet_grid_binds_wave7_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "sheet:stats_students_t_cdf",
            Some("Statistics.students_t_cdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_students_t_two_sided_p",
            Some("Statistics.students_t_two_sided_p")
        )));
        assert!(tools.contains(&(
            "sheet:stats_students_t_upper_p",
            Some("Statistics.students_t_upper_p")
        )));
        assert!(tools.contains(&(
            "sheet:stats_students_t_quantile",
            Some("Statistics.students_t_quantile")
        )));
        assert!(tools.contains(&(
            "sheet:stats_fisher_f_cdf",
            Some("Statistics.fisher_f_cdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_fisher_f_upper_p",
            Some("Statistics.fisher_f_upper_p")
        )));
        assert!(tools.contains(&(
            "sheet:stats_fisher_f_quantile",
            Some("Statistics.fisher_f_quantile")
        )));
        assert!(tools.contains(&(
            "sheet:stats_tukey_fences",
            Some("Statistics.tukey_fences")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave8_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "sheet:stats_empirical_cdf",
            Some("Statistics.empirical_cdf")
        )));
        assert!(tools.contains(&("sheet:stats_entropy", Some("Statistics.entropy"))));
        assert!(tools.contains(&(
            "sheet:stats_kl_divergence",
            Some("Statistics.kl_divergence")
        )));
        assert!(tools.contains(&(
            "sheet:stats_cross_entropy",
            Some("Statistics.cross_entropy")
        )));
        assert!(tools.contains(&(
            "sheet:stats_entropy_from_counts",
            Some("Statistics.entropy_from_counts")
        )));
        assert!(tools.contains(&(
            "sheet:stats_moving_average",
            Some("Statistics.moving_average")
        )));
        assert!(tools.contains(&(
            "sheet:stats_modified_z_score_outliers",
            Some("Statistics.modified_z_score_outliers")
        )));
        assert!(tools.contains(&(
            "sheet:stats_iqr_outliers",
            Some("Statistics.iqr_outliers")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave9_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "sheet:stats_exponential_smoothing",
            Some("Statistics.exponential_smoothing")
        )));
        assert!(tools.contains(&(
            "sheet:stats_adf_proxy",
            Some("Statistics.adf_proxy")
        )));
        assert!(tools.contains(&(
            "sheet:stats_histogram",
            Some("Statistics.histogram")
        )));
        assert!(tools.contains(&(
            "sheet:stats_ks_1sample",
            Some("Statistics.ks_1sample")
        )));
        assert!(tools.contains(&(
            "sheet:stats_grubbs_test",
            Some("Statistics.grubbs_test")
        )));
        assert!(tools.contains(&(
            "sheet:stats_one_sample_t",
            Some("Statistics.one_sample_t")
        )));
        assert!(tools.contains(&(
            "sheet:stats_two_sample_t",
            Some("Statistics.two_sample_t")
        )));
        assert!(tools.contains(&(
            "sheet:stats_paired_t",
            Some("Statistics.paired_t")
        )));
        assert!(tools.contains(&(
            "sheet:stats_linear_regression",
            Some("Statistics.linear_regression")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave10_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "sheet:stats_chi_square_gof",
            Some("Statistics.chi_square_gof")
        )));
        assert!(tools.contains(&(
            "sheet:stats_chi_square_independence",
            Some("Statistics.chi_square_independence")
        )));
        assert!(tools.contains(&(
            "sheet:stats_correlation_p_value",
            Some("Statistics.correlation_p_value")
        )));
        assert!(tools.contains(&(
            "sheet:stats_friedman",
            Some("Statistics.friedman")
        )));
        assert!(tools.contains(&(
            "sheet:stats_ljung_box",
            Some("Statistics.ljung_box")
        )));
        assert!(tools.contains(&(
            "sheet:stats_mann_whitney_u",
            Some("Statistics.mann_whitney_u")
        )));
        assert!(tools.contains(&(
            "sheet:stats_mcnemar",
            Some("Statistics.mcnemar")
        )));
        assert!(tools.contains(&(
            "sheet:stats_mutual_information",
            Some("Statistics.mutual_information")
        )));
        assert!(tools.contains(&(
            "sheet:stats_one_way_anova",
            Some("Statistics.one_way_anova")
        )));
        assert!(tools.contains(&(
            "sheet:stats_mahalanobis_sq",
            Some("Statistics.mahalanobis_sq")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave11_statistics_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "sheet:stats_mvn_log_pdf",
            Some("Statistics.mvn_log_pdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_mvn_pdf",
            Some("Statistics.mvn_pdf")
        )));
        assert!(tools.contains(&(
            "sheet:stats_mvn_sample",
            Some("Statistics.mvn_sample")
        )));
        assert!(tools.contains(&(
            "sheet:stats_mvn_mle",
            Some("Statistics.mvn_mle")
        )));
        assert!(tools.contains(&(
            "sheet:stats_validate_probability",
            Some("Statistics.validate_probability")
        )));
        assert!(tools.contains(&(
            "sheet:stats_simplex_project",
            Some("Statistics.simplex_project")
        )));
        assert!(tools.contains(&(
            "sheet:stats_fisher_distance",
            Some("Statistics.fisher_distance")
        )));
        assert!(tools.contains(&(
            "sheet:stats_neg_entropy",
            Some("Statistics.neg_entropy")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave12_poly_algebra_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "sheet:poly_eval",
            Some("PolynomialAlgebra.eval")
        )));
        assert!(tools.contains(&(
            "sheet:poly_add",
            Some("PolynomialAlgebra.add")
        )));
        assert!(tools.contains(&(
            "sheet:poly_sub",
            Some("PolynomialAlgebra.sub")
        )));
        assert!(tools.contains(&(
            "sheet:poly_mul",
            Some("PolynomialAlgebra.mul")
        )));
        assert!(tools.contains(&(
            "sheet:poly_gcd",
            Some("PolynomialAlgebra.gcd")
        )));
        assert!(tools.contains(&(
            "sheet:poly_degree",
            Some("PolynomialAlgebra.degree")
        )));
        assert!(tools.contains(&(
            "sheet:poly_leading",
            Some("PolynomialAlgebra.leading")
        )));
        assert!(tools.contains(&(
            "sheet:poly_is_zero",
            Some("PolynomialAlgebra.is_zero")
        )));
        assert!(tools.contains(&(
            "sheet:poly_scale",
            Some("PolynomialAlgebra.scale")
        )));
        assert!(tools.contains(&(
            "sheet:poly_zero",
            Some("PolynomialAlgebra.zero")
        )));
        assert!(tools.contains(&(
            "sheet:poly_constant",
            Some("PolynomialAlgebra.constant")
        )));
        assert!(tools.contains(&(
            "sheet:poly_derivative",
            Some("PolynomialAlgebra.derivative")
        )));
    }

    #[test]
    fn sheet_grid_binds_wave13_poly_and_manifold_caps() {
        let registry = super::build_registry();
        let sheet = registry.toolbox("sheet").expect("sheet toolbox");
        let chain = sheet
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "sheet:grid")
            .expect("sheet:grid toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "sheet:poly_monic",
            Some("PolynomialAlgebra.monic")
        )));
        assert!(tools.contains(&(
            "sheet:poly_div_rem",
            Some("PolynomialAlgebra.div_rem")
        )));
        assert!(tools.contains(&(
            "sheet:poly_resultant",
            Some("PolynomialAlgebra.resultant")
        )));
        assert!(tools.contains(&(
            "sheet:stats_simplex_project_idempotent",
            Some("Statistics.simplex_project_idempotent")
        )));
        assert!(tools.contains(&(
            "sheet:stats_fisher_inner_product",
            Some("Statistics.fisher_inner_product")
        )));
        assert!(tools.contains(&(
            "sheet:stats_neg_entropy_grad",
            Some("Statistics.neg_entropy_grad")
        )));
        assert!(tools.contains(&(
            "sheet:stats_kl_bregman_form",
            Some("Statistics.kl_bregman_form")
        )));
        assert!(tools.contains(&(
            "sheet:stats_bregman_pythagorean_test",
            Some("Statistics.bregman_pythagorean_test")
        )));
        assert!(tools.contains(&(
            "sheet:stats_probability_hash",
            Some("Statistics.probability_hash")
        )));
    }

    #[test]
    fn ai_ml_chain_binds_curated_machine_learning_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:ml")
            .expect("ai:ml toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("ai:ml_mse", Some("MachineLearning.mse"))));
        assert!(tools.contains(&("ai:ml_rmse", Some("MachineLearning.rmse"))));
        assert!(tools.contains(&("ai:ml_mae", Some("MachineLearning.mae"))));
        assert!(tools.contains(&("ai:ml_r2", Some("MachineLearning.r2_score"))));
        assert!(tools.contains(&("ai:ml_accuracy", Some("MachineLearning.accuracy"))));
        assert!(tools.contains(&("ai:ml_ols", Some("MachineLearning.ols"))));
        assert!(tools.contains(&(
            "ai:ml_train_test_split",
            Some("MachineLearning.train_test_split")
        )));
        assert!(tools.contains(&("ai:ml_kmeans", Some("MachineLearning.kmeans"))));
        assert!(tools.contains(&("ai:ml_log_loss", Some("MachineLearning.log_loss"))));
        assert!(tools.contains(&("ai:ml_bonferroni", Some("MachineLearning.bonferroni"))));
        assert!(tools.contains(&(
            "ai:ml_confusion_binary",
            Some("MachineLearning.confusion_binary")
        )));
    }

    #[test]
    fn ai_ml_chain_binds_wave2_multiple_testing_and_resampling_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:ml")
            .expect("ai:ml toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("ai:ml_holm", Some("MachineLearning.holm"))));
        assert!(tools.contains(&(
            "ai:ml_benjamini_hochberg",
            Some("MachineLearning.benjamini_hochberg")
        )));
        assert!(tools.contains(&("ai:ml_ab_test", Some("MachineLearning.ab_test"))));
        assert!(tools.contains(&(
            "ai:ml_bootstrap_estimate",
            Some("MachineLearning.bootstrap_estimate")
        )));
        assert!(tools.contains(&(
            "ai:ml_bootstrap_ci",
            Some("MachineLearning.bootstrap_ci")
        )));
        assert!(tools.contains(&(
            "ai:ml_permutation_test",
            Some("MachineLearning.permutation_test")
        )));
        assert!(tools.contains(&(
            "ai:ml_n_rejected",
            Some("MachineLearning.n_rejected")
        )));
        assert!(tools.contains(&(
            "ai:ml_power_two_sample",
            Some("MachineLearning.power_two_sample")
        )));
    }

    #[test]
    fn ai_ml_chain_binds_wave3_metrics_resampling_and_design_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:ml")
            .expect("ai:ml toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("ai:ml_roc_auc", Some("MachineLearning.roc_auc"))));
        assert!(tools.contains(&("ai:ml_k_fold", Some("MachineLearning.k_fold"))));
        assert!(tools.contains(&(
            "ai:ml_bootstrap_indices",
            Some("MachineLearning.bootstrap_indices")
        )));
        assert!(tools.contains(&("ai:ml_pca", Some("MachineLearning.pca"))));
        assert!(tools.contains(&(
            "ai:ml_required_sample_size",
            Some("MachineLearning.required_sample_size")
        )));
        assert!(tools.contains(&(
            "ai:ml_polynomial_regression",
            Some("MachineLearning.polynomial_regression")
        )));
        assert!(tools.contains(&(
            "ai:ml_required_n_two_prop",
            Some("MachineLearning.required_sample_size_two_proportion")
        )));
        assert!(tools.contains(&("ai:ml_loocv", Some("MachineLearning.loocv"))));
    }

    #[test]
    fn ai_ml_chain_binds_wave4_kg_scores_and_fitters_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:ml")
            .expect("ai:ml toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "ai:ml_transe_score",
            Some("MachineLearning.transe_score")
        )));
        assert!(tools.contains(&(
            "ai:ml_distmult_score",
            Some("MachineLearning.distmult_score")
        )));
        assert!(tools.contains(&(
            "ai:ml_complex_score",
            Some("MachineLearning.complex_score")
        )));
        assert!(tools.contains(&(
            "ai:ml_rotate_score",
            Some("MachineLearning.rotate_score")
        )));
        assert!(tools.contains(&("ai:ml_ridge_fit", Some("MachineLearning.ridge_fit"))));
        assert!(tools.contains(&("ai:ml_lasso_fit", Some("MachineLearning.lasso_fit"))));
        assert!(tools.contains(&("ai:ml_pls_fit", Some("MachineLearning.pls_fit"))));
        assert!(tools.contains(&(
            "ai:ml_standard_scaler",
            Some("MachineLearning.standard_scaler_fit_transform")
        )));
    }

    #[test]
    fn ai_ml_chain_binds_wave5_clustering_glm_and_classifiers_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:ml")
            .expect("ai:ml toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "ai:ml_kmeans_fit",
            Some("MachineLearning.kmeans_fit")
        )));
        assert!(tools.contains(&("ai:ml_gmm_fit", Some("MachineLearning.gmm_fit"))));
        assert!(tools.contains(&(
            "ai:ml_logistic_fit",
            Some("MachineLearning.logistic_fit")
        )));
        assert!(tools.contains(&(
            "ai:ml_poisson_fit",
            Some("MachineLearning.poisson_fit")
        )));
        assert!(tools.contains(&(
            "ai:ml_naive_bayes_fit",
            Some("MachineLearning.naive_bayes_fit")
        )));
        assert!(tools.contains(&("ai:ml_knn_fit", Some("MachineLearning.knn_fit"))));
        assert!(tools.contains(&("ai:ml_lda_fit", Some("MachineLearning.lda_fit"))));
        assert!(tools.contains(&("ai:ml_pcr_fit", Some("MachineLearning.pcr_fit"))));
    }

    #[test]
    fn ai_ml_chain_binds_wave6_discriminant_trees_and_survival_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:ml")
            .expect("ai:ml toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("ai:ml_qda_fit", Some("MachineLearning.qda_fit"))));
        assert!(tools.contains(&(
            "ai:ml_multinomial_logistic_fit",
            Some("MachineLearning.multinomial_logistic_fit")
        )));
        assert!(tools.contains(&(
            "ai:ml_hierarchical_fit",
            Some("MachineLearning.hierarchical_fit")
        )));
        assert!(tools.contains(&(
            "ai:ml_hierarchical_labels",
            Some("MachineLearning.hierarchical_labels")
        )));
        assert!(tools.contains(&(
            "ai:ml_bayesian_linear_fit",
            Some("MachineLearning.bayesian_linear_fit")
        )));
        assert!(tools.contains(&(
            "ai:ml_decision_tree_regressor",
            Some("MachineLearning.decision_tree_fit_regressor")
        )));
        assert!(tools.contains(&(
            "ai:ml_decision_tree_classifier",
            Some("MachineLearning.decision_tree_fit_classifier")
        )));
        assert!(tools.contains(&("ai:ml_gp_fit", Some("MachineLearning.gp_fit"))));
        assert!(tools.contains(&("ai:ml_svm_fit", Some("MachineLearning.svm_fit"))));
        assert!(tools.contains(&(
            "ai:ml_kaplan_meier_fit",
            Some("MachineLearning.kaplan_meier_fit")
        )));
    }

    #[test]
    fn ai_ml_chain_binds_wave7_forests_boosting_and_sequential_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:ml")
            .expect("ai:ml toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("ai:ml_cox_fit", Some("MachineLearning.cox_fit"))));
        assert!(tools.contains(&(
            "ai:ml_hmm_baum_welch",
            Some("MachineLearning.hmm_baum_welch")
        )));
        assert!(tools.contains(&(
            "ai:ml_variational_gaussian_fit",
            Some("MachineLearning.variational_gaussian_fit")
        )));
        assert!(tools.contains(&(
            "ai:ml_mcmc_metropolis",
            Some("MachineLearning.mcmc_metropolis")
        )));
        assert!(tools.contains(&(
            "ai:ml_svm_multiclass_fit",
            Some("MachineLearning.svm_multiclass_fit")
        )));
        assert!(tools.contains(&("ai:ml_som_train", Some("MachineLearning.som_train"))));
        assert!(tools.contains(&(
            "ai:ml_random_forest_regressor",
            Some("MachineLearning.random_forest_fit_regressor")
        )));
        assert!(tools.contains(&(
            "ai:ml_random_forest_classifier",
            Some("MachineLearning.random_forest_fit_classifier")
        )));
        assert!(tools.contains(&(
            "ai:ml_gradient_boosting_regressor",
            Some("MachineLearning.gradient_boosting_fit_regressor")
        )));
        assert!(tools.contains(&("ai:ml_bart_fit", Some("MachineLearning.bart_fit"))));
    }

    #[test]
    fn ai_ml_chain_binds_wave8_kg_kalman_factor_and_al_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:ml")
            .expect("ai:ml toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "ai:ml_kg_mean_rank",
            Some("MachineLearning.kg_mean_rank")
        )));
        assert!(tools.contains(&(
            "ai:ml_kg_mrr",
            Some("MachineLearning.kg_mean_reciprocal_rank")
        )));
        assert!(tools.contains(&(
            "ai:ml_kg_hits_at_k",
            Some("MachineLearning.kg_hits_at_k")
        )));
        assert!(tools.contains(&(
            "ai:ml_kalman_new",
            Some("MachineLearning.kalman_new")
        )));
        assert!(tools.contains(&(
            "ai:ml_factor_graph_marginals",
            Some("MachineLearning.factor_graph_marginals")
        )));
        assert!(tools.contains(&(
            "ai:ml_al_row_score",
            Some("MachineLearning.al_row_score")
        )));
        assert!(tools.contains(&("ai:ml_al_score", Some("MachineLearning.al_score"))));
        assert!(tools.contains(&(
            "ai:ml_al_cosine",
            Some("MachineLearning.al_cosine_similarity")
        )));
    }

    #[test]
    fn ai_ml_chain_binds_wave9_al_rank_density_and_committee_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:ml")
            .expect("ai:ml toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "ai:ml_al_rank_informative",
            Some("MachineLearning.al_rank_informative")
        )));
        assert!(tools.contains(&(
            "ai:ml_al_most_informative",
            Some("MachineLearning.al_most_informative")
        )));
        assert!(tools.contains(&(
            "ai:ml_al_representativeness",
            Some("MachineLearning.al_representativeness")
        )));
        assert!(tools.contains(&(
            "ai:ml_al_information_density",
            Some("MachineLearning.al_information_density")
        )));
        assert!(tools.contains(&(
            "ai:ml_al_rank_by_density",
            Some("MachineLearning.al_rank_by_density")
        )));
        assert!(tools.contains(&(
            "ai:ml_al_vote_entropy",
            Some("MachineLearning.al_vote_entropy")
        )));
        assert!(tools.contains(&(
            "ai:ml_al_consensus",
            Some("MachineLearning.al_consensus")
        )));
        assert!(tools.contains(&(
            "ai:ml_al_consensus_entropy",
            Some("MachineLearning.al_consensus_entropy")
        )));
        assert!(tools.contains(&(
            "ai:ml_al_kl_disagreement",
            Some("MachineLearning.al_average_kl_disagreement")
        )));
        assert!(tools.contains(&(
            "ai:ml_al_rank_by_disagreement",
            Some("MachineLearning.al_rank_by_disagreement")
        )));
    }

    #[test]
    fn econ_live_chain_binds_curated_econ_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let chain = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:live")
            .expect("econ:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("econ:capm", Some("Econ.capm_expected_return"))));
        assert!(tools.contains(&("econ:gini", Some("Econ.gini"))));
        assert!(tools.contains(&("econ:mixed_nash", Some("Econ.mixed_nash_2x2"))));
        assert!(tools.contains(&("econ:black_scholes", Some("Econ.black_scholes"))));
        assert!(tools.contains(&("econ:solow", Some("Econ.solow_steady_state"))));
        assert!(tools.contains(&("econ:cournot", Some("Econ.cournot_duopoly"))));
        assert!(tools.contains(&("econ:bertrand", Some("Econ.bertrand_duopoly"))));
        assert!(tools.contains(&("econ:historical_var", Some("Econ.historical_var"))));
        assert!(tools.contains(&("econ:atkinson", Some("Econ.atkinson"))));
        assert!(tools.contains(&("econ:gordon_growth", Some("Econ.gordon_growth"))));
        assert!(tools.contains(&("econ:binomial_option", Some("Econ.binomial_option"))));
        assert!(tools.contains(&("econ:forward_rate", Some("Econ.forward_rate"))));
        assert!(tools.contains(&("econ:gbm_simulate", Some("Econ.gbm_simulate"))));
        assert!(tools.contains(&("econ:headcount_poverty", Some("Econ.headcount_poverty"))));
        assert!(tools.contains(&("econ:hyperbolic_discount", Some("Econ.hyperbolic_discount"))));
        assert!(tools.contains(&("econ:fiscal_multiplier", Some("Econ.fiscal_multiplier"))));
        assert!(tools.contains(&("econ:drawdown", Some("Econ.drawdown"))));
        assert!(tools.contains(&("econ:covariance_matrix", Some("Econ.covariance_matrix"))));
    }

    #[test]
    fn econ_live_wave2_binds_additional_econ_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let chain = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:live")
            .expect("econ:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("econ:capm_beta", Some("Econ.capm_beta"))));
        assert!(tools.contains(&("econ:autocorrelation", Some("Econ.autocorrelation"))));
        assert!(tools.contains(&("econ:cross_correlation", Some("Econ.cross_correlation"))));
        assert!(tools.contains(&(
            "econ:bertrand_with_demand",
            Some("Econ.bertrand_with_demand")
        )));
        assert!(tools.contains(&(
            "econ:check_budget_balance",
            Some("Econ.check_budget_balance")
        )));
        assert!(tools.contains(&(
            "econ:ccapm_equity_premium",
            Some("Econ.ccapm_equity_premium")
        )));
        assert!(tools.contains(&("econ:mean_return", Some("Econ.mean_return"))));
        assert!(tools.contains(&("econ:poverty_gap", Some("Econ.poverty_gap"))));
    }

    #[test]
    fn econ_live_wave3_binds_additional_econ_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let chain = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:live")
            .expect("econ:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("econ:sample_variance", Some("Econ.sample_variance"))));
        assert!(tools.contains(&(
            "econ:utilitarian_welfare",
            Some("Econ.utilitarian_welfare")
        )));
        assert!(tools.contains(&("econ:rawlsian_welfare", Some("Econ.rawlsian_welfare"))));
        assert!(tools.contains(&("econ:nash_welfare", Some("Econ.nash_welfare"))));
        assert!(tools.contains(&("econ:stackelberg", Some("Econ.stackelberg_duopoly"))));
        assert!(tools.contains(&("econ:put_call_parity", Some("Econ.put_call_parity"))));
        assert!(tools.contains(&("econ:parametric_var", Some("Econ.parametric_var"))));
        assert!(tools.contains(&("econ:laffer_curve", Some("Econ.laffer_curve"))));
        assert!(tools.contains(&("econ:historical_cvar", Some("Econ.historical_cvar"))));
        assert!(tools.contains(&("econ:endowment_effect", Some("Econ.endowment_effect"))));
    }

    #[test]
    fn econ_live_wave4_binds_additional_econ_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let chain = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:live")
            .expect("econ:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("econ:prospect_value", Some("Econ.prospect_value"))));
        assert!(tools.contains(&(
            "econ:probability_weight",
            Some("Econ.probability_weight")
        )));
        assert!(tools.contains(&("econ:ccapm_sdf", Some("Econ.ccapm_sdf"))));
        assert!(tools.contains(&("econ:gravity_flow", Some("Econ.gravity_flow"))));
        assert!(tools.contains(&("econ:transfer_payment", Some("Econ.transfer_payment"))));
        assert!(tools.contains(&("econ:efficiency_units", Some("Econ.efficiency_units"))));
        assert!(tools.contains(&(
            "econ:social_cost_of_carbon",
            Some("Econ.social_cost_of_carbon")
        )));
        assert!(tools.contains(&("econ:pollution_damage", Some("Econ.pollution_damage"))));
        assert!(tools.contains(&("econ:marginal_damage", Some("Econ.marginal_damage"))));
        assert!(tools.contains(&(
            "econ:ramsey_steady_state",
            Some("Econ.ramsey_steady_state")
        )));
    }

    #[test]
    fn econ_live_wave5_binds_additional_econ_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let chain = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:live")
            .expect("econ:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("econ:simple_returns", Some("Econ.simple_returns"))));
        assert!(tools.contains(&("econ:log_returns", Some("Econ.log_returns"))));
        assert!(tools.contains(&("econ:rolling_mean", Some("Econ.rolling_mean"))));
        assert!(tools.contains(&("econ:rolling_variance", Some("Econ.rolling_variance"))));
        assert!(tools.contains(&("econ:labor_supply", Some("Econ.labor_supply"))));
        assert!(tools.contains(&("econ:optimal_abatement", Some("Econ.optimal_abatement"))));
        assert!(tools.contains(&("econ:optimal_pollution", Some("Econ.optimal_pollution"))));
        assert!(tools.contains(&("econ:olg_steady_state", Some("Econ.olg_steady_state"))));
        assert!(tools.contains(&(
            "econ:ramsey_euler_residual",
            Some("Econ.ramsey_euler_residual")
        )));
        assert!(tools.contains(&(
            "econ:present_biased_utility",
            Some("Econ.present_biased_utility")
        )));
        assert!(tools.contains(&(
            "econ:reference_dependent_utility",
            Some("Econ.reference_dependent_utility")
        )));
    }

    #[test]
    fn econ_live_wave6_binds_additional_econ_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let chain = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:live")
            .expect("econ:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("econ:npv", Some("Econ.npv"))));
        assert!(tools.contains(&("econ:multi_period_ddm", Some("Econ.multi_period_ddm"))));
        assert!(tools.contains(&(
            "econ:portfolio_max_drawdown",
            Some("Econ.portfolio_max_drawdown")
        )));
        assert!(tools.contains(&(
            "econ:interpolate_zero_rate",
            Some("Econ.interpolate_zero_rate")
        )));
        assert!(tools.contains(&("econ:discount_factor", Some("Econ.discount_factor"))));
        assert!(tools.contains(&("econ:par_yield", Some("Econ.par_yield"))));
        assert!(tools.contains(&("econ:progressive_tax", Some("Econ.progressive_tax"))));
        assert!(tools.contains(&(
            "econ:abatement_net_benefit",
            Some("Econ.abatement_net_benefit")
        )));
        assert!(tools.contains(&(
            "econ:household_production_ces",
            Some("Econ.household_production_ces")
        )));
        assert!(tools.contains(&("econ:malfeasance_delta", Some("Econ.malfeasance_delta"))));
    }

    #[test]
    fn econ_live_wave7_binds_additional_econ_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let chain = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:live")
            .expect("econ:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "econ:portfolio_variance",
            Some("Econ.portfolio_variance")
        )));
        assert!(tools.contains(&(
            "econ:portfolio_returns",
            Some("Econ.portfolio_returns")
        )));
        assert!(tools.contains(&(
            "econ:distributional_npv",
            Some("Econ.distributional_npv")
        )));
        assert!(tools.contains(&("econ:stress_scenario", Some("Econ.stress_scenario"))));
        assert!(tools.contains(&(
            "econ:repeated_game_payoff",
            Some("Econ.repeated_game_payoff")
        )));
        assert!(tools.contains(&(
            "econ:total_transport_cost",
            Some("Econ.total_transport_cost")
        )));
        assert!(tools.contains(&(
            "econ:transition_probability",
            Some("Econ.transition_probability")
        )));
        assert!(tools.contains(&(
            "econ:expected_holding_time",
            Some("Econ.expected_holding_time")
        )));
        assert!(tools.contains(&("econ:check_ir", Some("Econ.check_ir"))));
        assert!(tools.contains(&("econ:vcg_payment", Some("Econ.vcg_payment"))));
    }

    #[test]
    fn econ_live_wave8_binds_additional_econ_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let chain = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:live")
            .expect("econ:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "econ:validate_transition_matrix",
            Some("Econ.validate_transition_matrix")
        )));
        assert!(tools.contains(&(
            "econ:stationary_distribution",
            Some("Econ.stationary_distribution")
        )));
        assert!(tools.contains(&(
            "econ:mean_first_passage",
            Some("Econ.mean_first_passage")
        )));
        assert!(tools.contains(&(
            "econ:degree_centrality",
            Some("Econ.degree_centrality")
        )));
        assert!(tools.contains(&(
            "econ:eigenvector_centrality",
            Some("Econ.eigenvector_centrality")
        )));
        assert!(tools.contains(&(
            "econ:new_keynesian_solve",
            Some("Econ.new_keynesian_solve")
        )));
        assert!(tools.contains(&(
            "econ:nearest_facility",
            Some("Econ.nearest_facility")
        )));
        assert!(tools.contains(&(
            "econ:pure_nash_equilibria",
            Some("Econ.pure_nash_equilibria")
        )));
        assert!(tools.contains(&("econ:morans_i", Some("Econ.morans_i"))));
        assert!(tools.contains(&(
            "econ:strategy_proofness",
            Some("Econ.strategy_proofness")
        )));
    }

    #[test]
    fn econ_live_wave9_binds_additional_econ_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let chain = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:live")
            .expect("econ:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("econ:lorenz_curve", Some("Econ.lorenz_curve"))));
        assert!(tools.contains(&("econ:ols", Some("Econ.ols"))));
        assert!(tools.contains(&("econ:wls", Some("Econ.wls"))));
        assert!(tools.contains(&(
            "econ:lucas_asset_price",
            Some("Econ.lucas_asset_price")
        )));
        assert!(tools.contains(&("econ:bellman_update", Some("Econ.bellman_update"))));
        assert!(tools.contains(&(
            "econ:block_bootstrap",
            Some("Econ.block_bootstrap")
        )));
        assert!(tools.contains(&("econ:simulate_chain", Some("Econ.simulate_chain"))));
        assert!(tools.contains(&(
            "econ:value_iteration",
            Some("Econ.value_iteration")
        )));
        assert!(tools.contains(&("econ:iv_2sls", Some("Econ.iv_2sls"))));
        assert!(tools.contains(&("econ:logistic_mle", Some("Econ.logistic_mle"))));
    }

    #[test]
    fn econ_live_wave10_binds_remaining_econ_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let chain = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:live")
            .expect("econ:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "econ:interbank_clearing",
            Some("Econ.interbank_clearing")
        )));
        assert!(tools.contains(&(
            "econ:leontief_inverse",
            Some("Econ.leontief_inverse")
        )));
        assert!(tools.contains(&(
            "econ:output_multipliers",
            Some("Econ.output_multipliers")
        )));
        assert!(tools.contains(&(
            "econ:agent_based_aggregate_wealth",
            Some("Econ.agent_based_aggregate_wealth")
        )));
        assert!(tools.contains(&(
            "econ:validate_scalar_constraint",
            Some("Econ.validate_scalar_constraint")
        )));
        assert!(tools.contains(&(
            "econ:aggregate_paper_fills",
            Some("Econ.aggregate_paper_fills")
        )));
    }

    #[test]
    fn code_cas_binds_wave10_symbolic_algebra_caps() {
        let registry = super::build_registry();
        let code = registry.toolbox("code").expect("code toolbox");
        let chain = code
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "code:logic")
            .expect("code:logic toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "code:symbolic_differentiate",
            Some("SymbolicAlgebra.differentiate")
        )));
        assert!(tools.contains(&("code:symbolic_simplify", Some("SymbolicAlgebra.simplify"))));
        assert!(tools.contains(&("code:symbolic_expand", Some("SymbolicAlgebra.expand"))));
        assert!(tools.contains(&("code:symbolic_factor", Some("SymbolicAlgebra.factor"))));
        assert!(tools.contains(&("code:symbolic_integrate", Some("SymbolicAlgebra.integrate"))));
        assert!(tools.contains(&(
            "code:symbolic_simplify_trig",
            Some("SymbolicAlgebra.simplify_trig")
        )));
        assert!(tools.contains(&("code:symbolic_partial", Some("SymbolicAlgebra.partial"))));
        assert!(tools.contains(&("code:symbolic_limit", Some("SymbolicAlgebra.limit"))));
    }

    #[test]
    fn code_cas_binds_wave11_symbolic_algebra_constructors() {
        let registry = super::build_registry();
        let code = registry.toolbox("code").expect("code toolbox");
        let chain = code
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "code:logic")
            .expect("code:logic toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("code:symbolic_add", Some("SymbolicAlgebra.add"))));
        assert!(tools.contains(&("code:symbolic_sub", Some("SymbolicAlgebra.sub"))));
        assert!(tools.contains(&("code:symbolic_mul", Some("SymbolicAlgebra.mul"))));
        assert!(tools.contains(&("code:symbolic_div", Some("SymbolicAlgebra.div"))));
        assert!(tools.contains(&("code:symbolic_pow", Some("SymbolicAlgebra.pow"))));
        assert!(tools.contains(&("code:symbolic_neg", Some("SymbolicAlgebra.neg"))));
        assert!(tools.contains(&("code:symbolic_sqrt", Some("SymbolicAlgebra.sqrt"))));
        assert!(tools.contains(&("code:symbolic_exp", Some("SymbolicAlgebra.exp"))));
    }

    #[test]
    fn code_cas_binds_wave12_symbolic_algebra_trig_log_parse() {
        let registry = super::build_registry();
        let code = registry.toolbox("code").expect("code toolbox");
        let chain = code
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "code:logic")
            .expect("code:logic toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("code:symbolic_ln", Some("SymbolicAlgebra.ln"))));
        assert!(tools.contains(&("code:symbolic_sin", Some("SymbolicAlgebra.sin"))));
        assert!(tools.contains(&("code:symbolic_cos", Some("SymbolicAlgebra.cos"))));
        assert!(tools.contains(&("code:symbolic_tan", Some("SymbolicAlgebra.tan"))));
        assert!(tools.contains(&("code:symbolic_parse", Some("SymbolicAlgebra.parse"))));
        assert!(tools.contains(&("code:symbolic_c", Some("SymbolicAlgebra.c"))));
        assert!(tools.contains(&("code:symbolic_var", Some("SymbolicAlgebra.var"))));
        assert!(tools.contains(&("code:symbolic_hessian", Some("SymbolicAlgebra.hessian"))));
    }

    #[test]
    fn code_cas_binds_wave13_symbolic_algebra_series_roots() {
        let registry = super::build_registry();
        let code = registry.toolbox("code").expect("code toolbox");
        let chain = code
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "code:logic")
            .expect("code:logic toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "code:symbolic_integrate_definite",
            Some("SymbolicAlgebra.integrate_definite")
        )));
        assert!(tools.contains(&(
            "code:symbolic_limit_at_infinity",
            Some("SymbolicAlgebra.limit_at_infinity")
        )));
        assert!(tools.contains(&(
            "code:symbolic_real_roots",
            Some("SymbolicAlgebra.real_roots")
        )));
        assert!(tools.contains(&("code:symbolic_roots", Some("SymbolicAlgebra.roots"))));
        assert!(tools.contains(&(
            "code:symbolic_taylor_coefficients",
            Some("SymbolicAlgebra.taylor_coefficients")
        )));
        assert!(tools.contains(&(
            "code:symbolic_taylor_eval",
            Some("SymbolicAlgebra.taylor_eval")
        )));
        assert!(tools.contains(&("code:symbolic_jacobian", Some("SymbolicAlgebra.jacobian"))));
        assert!(tools.contains(&(
            "code:symbolic_gradient_at",
            Some("SymbolicAlgebra.gradient_at")
        )));
    }

    #[test]
    fn code_cas_binds_wave14_symbolic_algebra_remainder() {
        let registry = super::build_registry();
        let code = registry.toolbox("code").expect("code toolbox");
        let chain = code
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "code:logic")
            .expect("code:logic toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "code:symbolic_hessian_at",
            Some("SymbolicAlgebra.hessian_at")
        )));
        assert!(tools.contains(&(
            "code:symbolic_solve_quadratic",
            Some("SymbolicAlgebra.solve_quadratic")
        )));
        assert!(tools.contains(&(
            "code:symbolic_solve_quadratic_symbolic",
            Some("SymbolicAlgebra.solve_quadratic_symbolic")
        )));
        assert!(tools.contains(&(
            "code:symbolic_factor_quadratic",
            Some("SymbolicAlgebra.factor_quadratic")
        )));
        assert!(tools.contains(&(
            "code:symbolic_solve_polynomial_expr",
            Some("SymbolicAlgebra.solve_polynomial_expr")
        )));
        assert!(tools.contains(&(
            "code:symbolic_simplify_with_assumptions",
            Some("SymbolicAlgebra.simplify_with_assumptions")
        )));
        assert!(tools.contains(&(
            "code:symbolic_expr_citation_hash",
            Some("SymbolicAlgebra.expr_citation_hash")
        )));
        assert!(tools.contains(&(
            "code:symbolic_to_quins",
            Some("SymbolicAlgebra.to_quins")
        )));
        assert!(tools.contains(&(
            "code:symbolic_from_quins",
            Some("SymbolicAlgebra.from_quins")
        )));
    }

    #[test]
    fn image_vision_chain_binds_remaining_computer_vision_caps() {
        let registry = super::build_registry();
        let image = registry.toolbox("image").expect("image toolbox");
        let chain = image
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "image:vision")
            .expect("image:vision toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "image:equalize_hist",
            Some("ComputerVision.equalize_hist")
        )));
        assert!(tools.contains(&("image:rgb_to_gray", Some("ComputerVision.rgb_to_gray"))));
        assert!(tools.contains(&("image:dhash", Some("ComputerVision.dhash"))));
        assert!(tools.contains(&(
            "image:hamming_distance",
            Some("ComputerVision.hamming_distance")
        )));
        assert!(tools.contains(&(
            "image:cosine_similarity",
            Some("ComputerVision.cosine_similarity")
        )));
    }

    #[test]
    fn every_chain_has_at_least_one_tool() {
        let registry = super::build_registry();
        for toolbox in registry.toolboxes() {
            for chain in toolbox.chains() {
                assert!(
                    !chain.tools().is_empty(),
                    "empty chain {} in toolbox {}",
                    chain.metadata().id,
                    toolbox.metadata().id
                );
            }
        }
    }

    #[test]
    fn sci_linalg_binds_wave11_linear_algebra_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:linalg")
            .expect("scientific:linalg toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:la_matmul",
            Some("LinearAlgebra.matmul")
        )));
        assert!(tools.contains(&(
            "scientific:la_matvec",
            Some("LinearAlgebra.matvec")
        )));
        assert!(tools.contains(&(
            "scientific:la_transpose",
            Some("LinearAlgebra.transpose")
        )));
        assert!(tools.contains(&(
            "scientific:la_determinant",
            Some("LinearAlgebra.determinant")
        )));
        assert!(tools.contains(&("scientific:la_solve", Some("LinearAlgebra.solve"))));
        assert!(tools.contains(&("scientific:la_scale", Some("LinearAlgebra.scale"))));
        assert!(tools.contains(&(
            "scientific:la_add_into",
            Some("LinearAlgebra.add_into")
        )));
        assert!(tools.contains(&("scientific:la_axpy", Some("LinearAlgebra.axpy"))));
        assert!(tools.contains(&(
            "scientific:la_hadamard_into",
            Some("LinearAlgebra.hadamard_into")
        )));
        assert!(tools.contains(&(
            "scientific:la_qr_factor",
            Some("LinearAlgebra.qr_factor")
        )));
    }

    #[test]
    fn sci_linalg_binds_wave12_decompositions_and_spectrum() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:linalg")
            .expect("scientific:linalg toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:la_cholesky_factor",
            Some("LinearAlgebra.cholesky_factor")
        )));
        assert!(tools.contains(&(
            "scientific:la_cholesky_solve",
            Some("LinearAlgebra.cholesky_solve")
        )));
        assert!(tools.contains(&(
            "scientific:la_lu_decompose",
            Some("LinearAlgebra.lu_decompose")
        )));
        assert!(tools.contains(&(
            "scientific:la_lu_solve",
            Some("LinearAlgebra.lu_solve")
        )));
        assert!(tools.contains(&(
            "scientific:la_qr_form_q",
            Some("LinearAlgebra.qr_form_q")
        )));
        assert!(tools.contains(&(
            "scientific:la_qr_solve_ls",
            Some("LinearAlgebra.qr_solve_least_squares")
        )));
        assert!(tools.contains(&("scientific:la_svd", Some("LinearAlgebra.svd"))));
        assert!(tools.contains(&(
            "scientific:la_eigenvalues",
            Some("LinearAlgebra.eigenvalues")
        )));
    }

    #[test]
    fn sci_linalg_binds_wave13_remaining_linear_algebra_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:linalg")
            .expect("scientific:linalg toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:la_add_assign",
            Some("LinearAlgebra.add_assign")
        )));
        assert!(tools.contains(&(
            "scientific:la_hadamard_assign",
            Some("LinearAlgebra.hadamard_assign")
        )));
        assert!(tools.contains(&(
            "scientific:la_cholesky_det",
            Some("LinearAlgebra.cholesky_determinant")
        )));
        assert!(tools.contains(&(
            "scientific:la_charpoly",
            Some("LinearAlgebra.characteristic_polynomial")
        )));
        assert!(tools.contains(&(
            "scientific:la_eigenvalues_general",
            Some("LinearAlgebra.eigenvalues_general")
        )));
        assert!(tools.contains(&(
            "scientific:la_eigen_symmetric",
            Some("LinearAlgebra.eigen_symmetric")
        )));
        assert!(tools.contains(&(
            "scientific:la_polynomial_roots",
            Some("LinearAlgebra.polynomial_roots")
        )));
        assert!(tools.contains(&(
            "scientific:la_solve_linear_system",
            Some("LinearAlgebra.solve_linear_system")
        )));
        assert!(tools.contains(&(
            "scientific:la_symmetric_eigen_3x3",
            Some("LinearAlgebra.symmetric_eigen_3x3")
        )));
        assert!(tools.contains(&("scientific:la_dot", Some("LinearAlgebra.dot"))));
        assert!(tools.contains(&("scientific:la_norm", Some("LinearAlgebra.norm"))));
        assert!(tools.contains(&("scientific:la_trace", Some("LinearAlgebra.trace"))));
        assert!(tools.contains(&(
            "scientific:la_identity",
            Some("LinearAlgebra.identity")
        )));
        assert!(tools.contains(&("scientific:la_inverse", Some("LinearAlgebra.inverse"))));
    }

    #[test]
    fn wave40_binds_linear_algebra_app_primitives() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:linalg")
            .expect("scientific:linalg toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("scientific:la_dot", Some("LinearAlgebra.dot"))));
        assert!(tools.contains(&("scientific:la_inverse", Some("LinearAlgebra.inverse"))));
        let crypto = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:crypto_priv")
            .expect("scientific:crypto_priv toolchain");
        assert!(crypto.tools().iter().any(|t| {
            t.metadata().id == "scientific:gemm_live"
                && t.metadata().capability_scope.as_deref() == Some("LinearAlgebra.gemm")
        }));
    }

    #[test]
    fn sci_chem_binds_wave14_integrals_and_angular_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:chem")
            .expect("scientific:chem toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:chem_boys",
            Some("Chemistry.boys_function")
        )));
        assert!(tools.contains(&(
            "scientific:chem_overlap_s",
            Some("Chemistry.overlap_s")
        )));
        assert!(tools.contains(&(
            "scientific:chem_kinetic_s",
            Some("Chemistry.kinetic_s")
        )));
        assert!(tools.contains(&(
            "scientific:chem_nuclear_s",
            Some("Chemistry.nuclear_s")
        )));
        assert!(tools.contains(&(
            "scientific:chem_dipole_s",
            Some("Chemistry.dipole_s")
        )));
        assert!(tools.contains(&(
            "scientific:chem_evaluate_eri",
            Some("Chemistry.evaluate_eri")
        )));
        assert!(tools.contains(&(
            "scientific:chem_total_angular_momentum",
            Some("Chemistry.total_angular_momentum")
        )));
        assert!(tools.contains(&("scientific:chem_letter", Some("Chemistry.letter"))));
        assert!(tools.contains(&(
            "scientific:chem_n_cartesian",
            Some("Chemistry.n_cartesian")
        )));
        assert!(tools.contains(&(
            "scientific:chem_n_spherical",
            Some("Chemistry.n_spherical")
        )));
        assert!(tools.contains(&(
            "scientific:chem_from_letter",
            Some("Chemistry.from_letter")
        )));
    }

    #[test]
    fn sci_physics_binds_wave14_physics_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:physics")
            .expect("scientific:physics toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:phys_doppler",
            Some("Physics.doppler_shift")
        )));
        assert!(tools.contains(&(
            "scientific:phys_emf_attenuation",
            Some("Physics.emf_attenuation")
        )));
        assert!(tools.contains(&(
            "scientific:phys_harmonic",
            Some("Physics.harmonic_oscillator")
        )));
        assert!(tools.contains(&("scientific:phys_pendulum", Some("Physics.pendulum"))));
        assert!(tools.contains(&(
            "scientific:phys_logistic",
            Some("Physics.logistic_growth")
        )));
        assert!(tools.contains(&("scientific:phys_cfd_step", Some("Physics.cfd_step"))));
        assert!(tools.contains(&(
            "scientific:phys_heat_1d",
            Some("Physics.heat_diffusion_1d")
        )));
        assert!(tools.contains(&("scientific:phys_wave_1d", Some("Physics.wave_1d"))));
        assert!(tools.contains(&(
            "scientific:phys_advection_1d",
            Some("Physics.advection_diffusion_1d")
        )));
        assert!(tools.contains(&(
            "scientific:phys_quantum_1d",
            Some("Physics.quantum_states_1d")
        )));
        assert!(tools.len() >= 10);
    }

    #[test]
    fn sci_wave15_physics_remainder_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:physics")
            .expect("scientific:physics toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("scientific:phys_n_body", Some("Physics.n_body"))));
        assert!(tools.contains(&(
            "scientific:phys_molecular_dynamics",
            Some("Physics.molecular_dynamics")
        )));
        assert!(tools.contains(&(
            "scientific:phys_emf_interference",
            Some("Physics.emf_interference")
        )));
        assert!(tools.contains(&(
            "scientific:phys_emf_field_grid",
            Some("Physics.emf_field_grid_3d")
        )));
        assert!(tools.contains(&(
            "scientific:phys_emf_sample_depth",
            Some("Physics.emf_sample_at_depth")
        )));
        assert!(tools.contains(&(
            "scientific:phys_field_sample",
            Some("Physics.field_sample")
        )));
        assert!(tools.contains(&(
            "scientific:phys_material_query",
            Some("Physics.material_query")
        )));
        assert!(tools.contains(&(
            "scientific:phys_evaluate_interaction",
            Some("Physics.evaluate_interaction")
        )));
        assert_eq!(tools.len(), 18);
    }

    #[test]
    fn sci_wave15_special_functions_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:sf")
            .expect("scientific:sf toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:sf_airy_ai",
            Some("SpecialFunctions.airy_ai")
        )));
        assert!(tools.contains(&(
            "scientific:sf_airy_bi",
            Some("SpecialFunctions.airy_bi")
        )));
        assert!(tools.contains(&("scientific:sf_zeta", Some("SpecialFunctions.zeta"))));
        assert!(tools.contains(&(
            "scientific:sf_legendre",
            Some("SpecialFunctions.legendre")
        )));
        assert!(tools.contains(&(
            "scientific:sf_chebyshev_t",
            Some("SpecialFunctions.chebyshev_t")
        )));
        assert!(tools.contains(&(
            "scientific:sf_chebyshev_u",
            Some("SpecialFunctions.chebyshev_u")
        )));
        assert!(tools.contains(&(
            "scientific:sf_hermite",
            Some("SpecialFunctions.hermite")
        )));
        assert!(tools.contains(&(
            "scientific:sf_laguerre",
            Some("SpecialFunctions.laguerre")
        )));
        assert!(tools.contains(&(
            "scientific:sf_bessel_j",
            Some("SpecialFunctions.bessel_j")
        )));
        assert!(tools.contains(&(
            "scientific:sf_bessel_i",
            Some("SpecialFunctions.bessel_i")
        )));
        assert!(tools.contains(&(
            "scientific:sf_bessel_y",
            Some("SpecialFunctions.bessel_y")
        )));
        assert!(tools.contains(&(
            "scientific:sf_bessel_k",
            Some("SpecialFunctions.bessel_k")
        )));
        assert_eq!(tools.len(), 12);
    }

    #[test]
    fn sci_wave16_calculus_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:calc")
            .expect("scientific:calc toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:calc_hermite_dense",
            Some("Calculus.hermite_dense_output")
        )));
        assert!(tools.contains(&("scientific:calc_bdf1", Some("Calculus.bdf1_step"))));
        assert!(tools.contains(&("scientific:calc_bdf2", Some("Calculus.bdf2_step"))));
        assert!(tools.contains(&("scientific:calc_verlet", Some("Calculus.verlet_step"))));
        assert!(tools.contains(&("scientific:calc_ruth3", Some("Calculus.ruth3_step"))));
        assert!(tools.contains(&(
            "scientific:calc_yoshida4",
            Some("Calculus.yoshida4_step")
        )));
        assert!(tools.contains(&(
            "scientific:calc_integrate_bdf",
            Some("Calculus.integrate_bdf")
        )));
        assert!(tools.contains(&(
            "scientific:calc_integrate_sens",
            Some("Calculus.integrate_with_sensitivity")
        )));
        assert!(tools.contains(&(
            "scientific:calc_invariant_drift",
            Some("Calculus.invariant_drift")
        )));
        assert!(tools.contains(&(
            "scientific:calc_perm_parity",
            Some("Calculus.permutation_parity")
        )));
        assert!(tools.contains(&(
            "scientific:calc_pack_f32",
            Some("Calculus.pack_f32_pair")
        )));
        assert!(tools.contains(&(
            "scientific:calc_unpack_f32",
            Some("Calculus.unpack_f32_pair")
        )));
        assert_eq!(tools.len(), 22);
    }

    #[test]
    fn sci_calc_binds_wave19_calculus_remainder_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:calc")
            .expect("scientific:calc toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:calc_poisson_bracket",
            Some("Calculus.canonical_poisson_bracket")
        )));
        assert!(tools.contains(&(
            "scientific:calc_stormer_verlet",
            Some("Calculus.stormer_verlet_step")
        )));
        assert!(tools.contains(&(
            "scientific:calc_gauss_kronrod",
            Some("Calculus.adaptive_gauss_kronrod_15")
        )));
        assert!(tools.contains(&("scientific:calc_jvp", Some("Calculus.jvp"))));
        assert!(tools.contains(&("scientific:calc_vjp", Some("Calculus.vjp"))));
        assert!(tools.contains(&(
            "scientific:calc_adaptive_simpson",
            Some("Calculus.adaptive_simpson")
        )));
        assert!(tools.contains(&(
            "scientific:calc_adaptive_deriv",
            Some("Calculus.adaptive_derivative")
        )));
        assert!(tools.contains(&(
            "scientific:calc_newton_solve",
            Some("Calculus.newton_solve")
        )));
        assert!(tools.contains(&(
            "scientific:calc_num_jacobian",
            Some("Calculus.numerical_jacobian")
        )));
        assert!(tools.contains(&(
            "scientific:calc_num_hessian",
            Some("Calculus.numerical_hessian")
        )));
        assert_eq!(tools.len(), 22);
    }

    #[test]
    fn code_constr_binds_wave15_constructibility_caps() {
        let registry = super::build_registry();
        let code = registry.toolbox("code").expect("code toolbox");
        let chain = code
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "code:constr")
            .expect("code:constr toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "code:constr_regular_polygon",
            Some("Constructibility.is_regular_polygon_constructible")
        )));
        assert!(tools.contains(&(
            "code:constr_fermat_prime",
            Some("Constructibility.is_fermat_prime")
        )));
        assert!(tools.contains(&(
            "code:constr_min_poly_degree",
            Some("Constructibility.constructible_from_min_poly_degree")
        )));
        assert!(tools.contains(&(
            "code:constr_power_of_two",
            Some("Constructibility.is_power_of_two")
        )));
        assert!(tools.contains(&(
            "code:constr_central_angle",
            Some("Constructibility.is_central_angle_constructible")
        )));
        assert!(tools.contains(&(
            "code:constr_doubling_cube",
            Some("Constructibility.doubling_the_cube_constructible")
        )));
        assert!(tools.contains(&(
            "code:constr_trisect_angle",
            Some("Constructibility.trisecting_general_angle_constructible")
        )));
        assert!(tools.contains(&(
            "code:constr_square_circle",
            Some("Constructibility.squaring_the_circle_constructible")
        )));
        assert!(tools.contains(&(
            "code:constr_number",
            Some("Constructibility.is_constructible_number")
        )));
        assert_eq!(tools.len(), 9);
    }

    #[test]
    fn sci_wave16_computational_geometry_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:cg")
            .expect("scientific:cg toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:cg_distance_2d",
            Some("ComputationalGeometry.distance_2d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_distance_3d",
            Some("ComputationalGeometry.distance_3d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_point_segment_2d",
            Some("ComputationalGeometry.point_segment_distance_2d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_orientation_2",
            Some("ComputationalGeometry.orientation_2")
        )));
        assert!(tools.contains(&(
            "scientific:cg_orient_3d",
            Some("ComputationalGeometry.orient_3d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_morton_encode_2d",
            Some("ComputationalGeometry.morton_encode_2d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_morton_decode_2d",
            Some("ComputationalGeometry.morton_decode_2d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_morton_encode_3d",
            Some("ComputationalGeometry.morton_encode_3d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_hilbert_encode_2d",
            Some("ComputationalGeometry.hilbert_encode_2d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_circumcenter",
            Some("ComputationalGeometry.circumcenter")
        )));
        assert!(tools.len() >= 10);
    }

    #[test]
    fn sci_wave17_computational_geometry_remainder_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:cg")
            .expect("scientific:cg toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:cg_point_segment_3d",
            Some("ComputationalGeometry.point_segment_distance_3d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_point_triangle_3d",
            Some("ComputationalGeometry.point_triangle_distance_3d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_convex_hull_2",
            Some("ComputationalGeometry.convex_hull_2")
        )));
        assert!(tools.contains(&(
            "scientific:cg_triangulate",
            Some("ComputationalGeometry.triangulate_polygon")
        )));
        assert!(tools.contains(&(
            "scientific:cg_surface_area",
            Some("ComputationalGeometry.surface_area")
        )));
        assert!(tools.contains(&(
            "scientific:cg_signed_volume",
            Some("ComputationalGeometry.signed_volume")
        )));
        assert!(tools.contains(&(
            "scientific:cg_segment_intersect_2",
            Some("ComputationalGeometry.line_segment_intersection_2")
        )));
        assert!(tools.contains(&(
            "scientific:cg_bezier_eval",
            Some("ComputationalGeometry.bezier_eval")
        )));
        assert!(tools.contains(&(
            "scientific:cg_nearest_site",
            Some("ComputationalGeometry.nearest_site_brute_force")
        )));
        assert_eq!(tools.len(), 19);
    }

    #[test]
    fn code_nt_binds_wave16_number_theory_caps() {
        let registry = super::build_registry();
        let code = registry.toolbox("code").expect("code toolbox");
        let chain = code
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "code:nt")
            .expect("code:nt toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("code:nt_gcd", Some("NumberTheory.gcd"))));
        assert!(tools.contains(&("code:nt_lcm", Some("NumberTheory.lcm"))));
        assert!(tools.contains(&("code:nt_is_prime", Some("NumberTheory.is_prime"))));
        assert!(tools.contains(&("code:nt_factorial", Some("NumberTheory.factorial"))));
        assert!(tools.contains(&("code:nt_binomial", Some("NumberTheory.binomial"))));
        assert!(tools.contains(&(
            "code:nt_euler_totient",
            Some("NumberTheory.euler_totient")
        )));
        assert!(tools.contains(&("code:nt_mod_pow", Some("NumberTheory.mod_pow"))));
        assert!(tools.contains(&("code:nt_next_prime", Some("NumberTheory.next_prime"))));
        assert!(tools.contains(&(
            "code:nt_mod_inverse",
            Some("NumberTheory.mod_inverse")
        )));
        assert!(tools.contains(&(
            "code:nt_divisor_count",
            Some("NumberTheory.divisor_count")
        )));
        assert!(tools.len() >= 10);
    }

    #[test]
    fn code_nt_binds_wave18_number_theory_remainder_caps() {
        let registry = super::build_registry();
        let code = registry.toolbox("code").expect("code toolbox");
        let chain = code
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "code:nt")
            .expect("code:nt toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "code:nt_prime_factors",
            Some("NumberTheory.prime_factors")
        )));
        assert!(tools.contains(&("code:nt_divisors", Some("NumberTheory.divisors"))));
        assert!(tools.contains(&("code:nt_mobius", Some("NumberTheory.mobius"))));
        assert!(tools.contains(&(
            "code:nt_divisor_sum",
            Some("NumberTheory.divisor_sum")
        )));
        assert!(tools.contains(&(
            "code:nt_partitions",
            Some("NumberTheory.partitions")
        )));
        assert!(tools.contains(&("code:nt_catalan", Some("NumberTheory.catalan"))));
        assert!(tools.contains(&(
            "code:nt_stirling_second",
            Some("NumberTheory.stirling_second")
        )));
        assert!(tools.contains(&(
            "code:nt_stirling_first",
            Some("NumberTheory.stirling_first")
        )));
        assert!(tools.contains(&(
            "code:nt_extended_gcd",
            Some("NumberTheory.extended_gcd")
        )));
        assert!(tools.contains(&("code:nt_crt", Some("NumberTheory.crt"))));
        assert_eq!(tools.len(), 20);
    }

    #[test]
    fn sci_wave17_engineering_analysis_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:eng")
            .expect("scientific:eng toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:eng_natural_freq",
            Some("EngineeringAnalysis.natural_frequency_sdof")
        )));
        assert!(tools.contains(&(
            "scientific:eng_harmonic_sdof",
            Some("EngineeringAnalysis.analyze_harmonic_sdof")
        )));
        assert!(tools.contains(&(
            "scientific:eng_euler",
            Some("EngineeringAnalysis.analyze_euler")
        )));
        assert!(tools.contains(&(
            "scientific:eng_reliability",
            Some("EngineeringAnalysis.compute_reliability_index")
        )));
        assert!(tools.contains(&(
            "scientific:eng_kinematics",
            Some("EngineeringAnalysis.kinematics")
        )));
        assert!(tools.contains(&(
            "scientific:eng_cauchy",
            Some("EngineeringAnalysis.cauchy_stress")
        )));
        assert!(tools.contains(&(
            "scientific:eng_drag",
            Some("EngineeringAnalysis.drag_force")
        )));
        assert!(tools.contains(&(
            "scientific:eng_reynolds",
            Some("EngineeringAnalysis.reynolds_number")
        )));
        assert!(tools.contains(&(
            "scientific:eng_fatigue",
            Some("EngineeringAnalysis.fatigue_cycles")
        )));
        assert!(tools.contains(&(
            "scientific:eng_miner",
            Some("EngineeringAnalysis.miner_damage")
        )));
        assert_eq!(tools.len(), 10);
    }

    #[test]
    fn code_fuzzy_binds_wave18_fuzzy_query_caps() {
        let registry = super::build_registry();
        let code = registry.toolbox("code").expect("code toolbox");
        let chain = code
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "code:fuzzy")
            .expect("code:fuzzy toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "code:fuzzy_triangular",
            Some("FuzzyQuery.triangular")
        )));
        assert!(tools.contains(&(
            "code:fuzzy_trapezoidal",
            Some("FuzzyQuery.trapezoidal")
        )));
        assert!(tools.contains(&(
            "code:fuzzy_approximately",
            Some("FuzzyQuery.approximately")
        )));
        assert!(tools.contains(&("code:fuzzy_ramp_up", Some("FuzzyQuery.ramp_up"))));
        assert!(tools.contains(&("code:fuzzy_ramp_down", Some("FuzzyQuery.ramp_down"))));
        assert!(tools.contains(&(
            "code:fuzzy_much_greater_than",
            Some("FuzzyQuery.much_greater_than")
        )));
        assert!(tools.contains(&(
            "code:fuzzy_much_less_than",
            Some("FuzzyQuery.much_less_than")
        )));
        assert!(tools.contains(&("code:fuzzy_threshold", Some("FuzzyQuery.threshold"))));
        assert!(tools.contains(&("code:fuzzy_top_k", Some("FuzzyQuery.top_k"))));
        assert!(tools.contains(&("code:fuzzy_negate", Some("FuzzyQuery.negate"))));
        assert!(tools.contains(&("code:fuzzy_and", Some("FuzzyQuery.and"))));
        assert!(tools.contains(&("code:fuzzy_or", Some("FuzzyQuery.or"))));
        assert_eq!(tools.len(), 12);
    }


    #[test]
    fn sci_wave17_geometric_algebra_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:ga")
            .expect("scientific:ga toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("scientific:ga_dot", Some("GeometricAlgebra.dot"))));
        assert!(tools.contains(&(
            "scientific:ga_cross",
            Some("GeometricAlgebra.cross_product")
        )));
        assert!(tools.contains(&(
            "scientific:ga_normalize",
            Some("GeometricAlgebra.normalize_vector")
        )));
        assert!(tools.contains(&(
            "scientific:ga_angle",
            Some("GeometricAlgebra.angle_between_vectors")
        )));
        assert!(tools.contains(&(
            "scientific:ga_geometric_product",
            Some("GeometricAlgebra.geometric_product")
        )));
        assert!(tools.contains(&(
            "scientific:ga_outer_product",
            Some("GeometricAlgebra.outer_product")
        )));
        assert!(tools.contains(&(
            "scientific:ga_rotor",
            Some("GeometricAlgebra.rotor_from_angle_axis")
        )));
        assert!(tools.contains(&(
            "scientific:ga_apply_rotor",
            Some("GeometricAlgebra.apply_rotor")
        )));
        assert!(tools.contains(&(
            "scientific:ga_translator",
            Some("GeometricAlgebra.translator_from_displacement")
        )));
        assert!(tools.contains(&(
            "scientific:ga_apply_translator",
            Some("GeometricAlgebra.apply_translator")
        )));
        assert!(tools.contains(&(
            "scientific:ga_is_simd",
            Some("GeometricAlgebra.is_simd_available")
        )));
        assert_eq!(tools.len(), 11);
    }

    #[test]
    fn sci_chem_binds_wave18_scf_element_lda_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:chem")
            .expect("scientific:chem toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:chem_gaussian_elim",
            Some("Chemistry.gaussian_elimination")
        )));
        assert!(tools.contains(&(
            "scientific:chem_jacobi",
            Some("Chemistry.jacobi_diagonalization")
        )));
        assert!(tools.contains(&(
            "scientific:chem_transpose",
            Some("Chemistry.transpose")
        )));
        assert!(tools.contains(&(
            "scientific:chem_orthogonalize",
            Some("Chemistry.orthogonalization_matrix")
        )));
        assert!(tools.contains(&(
            "scientific:chem_element_symbol",
            Some("Chemistry.element_symbol")
        )));
        assert!(tools.contains(&(
            "scientific:chem_atomic_number",
            Some("Chemistry.atomic_number")
        )));
        assert!(tools.contains(&(
            "scientific:chem_atomic_weight",
            Some("Chemistry.standard_atomic_weight")
        )));
        assert!(tools.contains(&(
            "scientific:chem_lda_exchange",
            Some("Chemistry.lda_exchange")
        )));
        assert!(tools.contains(&(
            "scientific:chem_lda_vwn",
            Some("Chemistry.lda_correlation_vwn")
        )));
        assert!(tools.contains(&(
            "scientific:chem_sto3g_h2",
            Some("Chemistry.sto3g_h2")
        )));
        assert_eq!(tools.len(), 21);
    }


    #[test]
    fn sci_cosmic_binds_wave19_cosmic_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:cosmic")
            .expect("scientific:cosmic toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:cosmic_geodetic_distance",
            Some("Cosmic.geodetic_distance")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_surface_gravity",
            Some("Cosmic.surface_gravity")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_flrw_distance",
            Some("Cosmic.flrw_distance")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_flrw_redshift",
            Some("Cosmic.flrw_redshift")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_flrw_hubble",
            Some("Cosmic.flrw_hubble_velocity")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_warp_velocity",
            Some("Cosmic.warp_velocity")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_warp_factor_c",
            Some("Cosmic.warp_factor_c")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_typical_length",
            Some("Cosmic.typical_length")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_observe_redshift",
            Some("Cosmic.observe_redshift")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_compton",
            Some("Cosmic.compton_wavelength")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_de_broglie",
            Some("Cosmic.de_broglie_wavelength")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_atm_pressure",
            Some("Cosmic.atmosphere_pressure")
        )));
        assert_eq!(tools.len(), 23);
    }

    #[test]
    fn sci_cosmic_binds_wave20_cosmic_remainder_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:cosmic")
            .expect("scientific:cosmic toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:cosmic_geodetic_to_ecef",
            Some("Cosmic.geodetic_to_ecef")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_ecef_to_geodetic",
            Some("Cosmic.ecef_to_geodetic")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_ecef_to_enu",
            Some("Cosmic.ecef_to_enu")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_enu_to_ecef",
            Some("Cosmic.enu_to_ecef")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_body_profile",
            Some("Cosmic.body_profile")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_stardate",
            Some("Cosmic.stardate_to_gregorian")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_cochrane",
            Some("Cosmic.cochrane_units")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_atm_temperature",
            Some("Cosmic.atmosphere_temperature")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_magnetosphere",
            Some("Cosmic.magnetosphere_field")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_scale_factor",
            Some("Cosmic.scale_factor")
        )));
        assert!(tools.contains(&(
            "scientific:cosmic_usri_parse",
            Some("Cosmic.usri_parse")
        )));
        assert_eq!(tools.len(), 23);
    }

    #[test]
    fn sci_xform_binds_wave19_integral_transforms_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:xform")
            .expect("scientific:xform toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("scientific:xform_dft", Some("IntegralTransforms.dft"))));
        assert!(tools.contains(&(
            "scientific:xform_dft_complex",
            Some("IntegralTransforms.dft_complex")
        )));
        assert!(tools.contains(&("scientific:xform_idft", Some("IntegralTransforms.idft"))));
        assert!(tools.contains(&(
            "scientific:xform_z_transform_finite",
            Some("IntegralTransforms.z_transform_finite")
        )));
        assert!(tools.contains(&(
            "scientific:xform_unit_step_z",
            Some("IntegralTransforms.unit_step_z")
        )));
        assert!(tools.contains(&(
            "scientific:xform_geometric_z",
            Some("IntegralTransforms.geometric_z")
        )));
        assert!(tools.contains(&(
            "scientific:xform_laplace_numeric",
            Some("IntegralTransforms.laplace_numeric")
        )));
        assert!(tools.contains(&(
            "scientific:xform_laplace_symbolic",
            Some("IntegralTransforms.laplace_symbolic")
        )));
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn ai_inf_binds_wave27_inference_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:inf")
            .expect("ai:inf toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("ai:inf_relu", Some("Inference.relu"))));
        assert!(tools.contains(&("ai:inf_sigmoid", Some("Inference.sigmoid"))));
        assert!(tools.contains(&("ai:inf_gelu", Some("Inference.gelu"))));
        assert!(tools.contains(&("ai:inf_softmax", Some("Inference.softmax"))));
        assert!(tools.contains(&("ai:inf_rms_norm", Some("Inference.rms_norm"))));
        assert!(tools.contains(&("ai:inf_embed", Some("Inference.embed"))));
        assert!(tools.contains(&(
            "ai:inf_run_classifier",
            Some("Inference.run_classifier")
        )));
        assert!(tools.contains(&(
            "ai:inf_vector_search",
            Some("Inference.vector_search")
        )));
        assert!(tools.contains(&("ai:inf_load_model", Some("Inference.load_model"))));
        assert!(tools.contains(&("ai:inf_unload_model", Some("Inference.unload_model"))));
        assert!(tools.contains(&(
            "ai:inf_run_transformer",
            Some("Inference.run_transformer")
        )));
        assert!(tools.contains(&("ai:inf_run_reranker", Some("Inference.run_reranker"))));
        assert!(tools.contains(&(
            "ai:inf_constrained_decode",
            Some("Inference.constrained_decode")
        )));
        assert_eq!(tools.len(), 13);
    }

    #[test]
    fn ai_orch_binds_wave20_orchestration_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:orch")
            .expect("ai:orch toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "ai:orch_session_create",
            Some("Orchestration.session_create")
        )));
        assert!(tools.contains(&(
            "ai:orch_session_plan",
            Some("Orchestration.session_plan")
        )));
        assert!(tools.contains(&(
            "ai:orch_session_execute",
            Some("Orchestration.session_execute")
        )));
        assert!(tools.contains(&(
            "ai:orch_session_status",
            Some("Orchestration.session_status")
        )));
        assert!(tools.contains(&(
            "ai:orch_roster_register",
            Some("Orchestration.roster_register")
        )));
        assert!(tools.contains(&(
            "ai:orch_roster_list",
            Some("Orchestration.roster_list")
        )));
        assert!(tools.contains(&(
            "ai:orch_roster_capabilities",
            Some("Orchestration.roster_capabilities")
        )));
        assert!(tools.contains(&(
            "ai:orch_assign_agents",
            Some("Orchestration.assign_agents")
        )));
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn spatial_threed_binds_wave20_threed_caps() {
        let registry = super::build_registry();
        let spatial = registry.toolbox("spatial").expect("spatial toolbox");
        let chain = spatial
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "spatial:threed")
            .expect("spatial:threed toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "spatial:threed_add_object",
            Some("ThreeD.add_object")
        )));
        assert!(tools.contains(&(
            "spatial:threed_set_transform",
            Some("ThreeD.set_transform")
        )));
        assert!(tools.contains(&(
            "spatial:threed_set_material",
            Some("ThreeD.set_material")
        )));
        assert!(tools.contains(&(
            "spatial:threed_add_camera",
            Some("ThreeD.add_camera")
        )));
        assert!(tools.contains(&("spatial:threed_add_light", Some("ThreeD.add_light"))));
        assert!(tools.contains(&("spatial:threed_add_rig", Some("ThreeD.add_rig"))));
        assert!(tools.contains(&(
            "spatial:threed_add_animation",
            Some("ThreeD.add_animation")
        )));
        assert!(tools.contains(&("spatial:threed_set_mesh", Some("ThreeD.set_mesh"))));
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn spatial_scene_binds_wave21_scene_caps() {
        let registry = super::build_registry();
        let spatial = registry.toolbox("spatial").expect("spatial toolbox");
        let chain = spatial
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "spatial:scene")
            .expect("spatial:scene toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "spatial:scene_lerp_camera",
            Some("Scene.lerp_camera")
        )));
        assert!(tools.contains(&(
            "spatial:scene_camera_frame_node",
            Some("Scene.camera_frame_node")
        )));
        assert!(tools.contains(&(
            "spatial:scene_smooth_damp",
            Some("Scene.smooth_damp")
        )));
        assert!(tools.contains(&(
            "spatial:scene_smooth_damp_vec3",
            Some("Scene.smooth_damp_vec3")
        )));
        assert!(tools.contains(&("spatial:scene_ik_look_at", Some("Scene.ik_look_at"))));
        assert!(tools.contains(&("spatial:scene_ik_ccd", Some("Scene.ik_ccd"))));
        assert!(tools.contains(&(
            "spatial:scene_set_render_budget",
            Some("Scene.set_render_budget")
        )));
        assert!(tools.contains(&(
            "spatial:scene_set_clear_colour",
            Some("Scene.set_clear_colour")
        )));
        // Wave-21 plus wave-22 graph/build Host binds.
        assert!(tools.len() >= 19);
    }

    #[test]
    fn spatial_scene_binds_wave22_scene_graph_caps() {
        let registry = super::build_registry();
        let spatial = registry.toolbox("spatial").expect("spatial toolbox");
        let chain = spatial
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "spatial:scene")
            .expect("spatial:scene toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("spatial:scene_create", Some("Scene.create"))));
        assert!(tools.contains(&("spatial:scene_add_node", Some("Scene.add_node"))));
        assert!(tools.contains(&(
            "spatial:scene_set_transform",
            Some("Scene.set_transform")
        )));
        assert!(tools.contains(&("spatial:scene_set_mesh", Some("Scene.set_mesh"))));
        assert!(tools.contains(&("spatial:scene_add_camera", Some("Scene.add_camera"))));
        assert!(tools.contains(&("spatial:scene_render", Some("Scene.render"))));
        assert!(tools.contains(&(
            "spatial:scene_set_viewport",
            Some("Scene.set_viewport")
        )));
        assert!(tools.contains(&(
            "spatial:scene_capture_frame",
            Some("Scene.capture_frame")
        )));
        assert!(tools.contains(&("spatial:scene_add_light", Some("Scene.add_light"))));
        assert!(tools.contains(&(
            "spatial:scene_link_semantic",
            Some("Scene.link_semantic")
        )));
        assert!(tools.contains(&(
            "spatial:scene_duplicate_node",
            Some("Scene.duplicate_node")
        )));
    }

    #[test]
    fn audio_fx_binds_wave22_audio_caps() {
        let registry = super::build_registry();
        let audio = registry.toolbox("audio").expect("audio toolbox");
        let chain = audio
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "audio:fx")
            .expect("audio:fx toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("audio:fx_oscillator", Some("Audio.oscillator"))));
        assert!(tools.contains(&("audio:fx_envelope", Some("Audio.envelope"))));
        assert!(tools.contains(&("audio:fx_filter", Some("Audio.filter"))));
        assert!(tools.contains(&("audio:fx_lfo", Some("Audio.lfo"))));
        assert!(tools.contains(&("audio:fx_delay", Some("Audio.delay"))));
        assert!(tools.contains(&("audio:fx_reverb", Some("Audio.reverb"))));
        assert!(tools.contains(&("audio:fx_compressor", Some("Audio.compressor"))));
        assert!(tools.contains(&("audio:fx_eq", Some("Audio.eq"))));
        assert!(tools.contains(&("audio:fx_transport", Some("Audio.transport"))));
        assert!(tools.contains(&(
            "audio:fx_waveform_meter",
            Some("Audio.waveform_meter")
        )));
        assert!(tools.contains(&("audio:fx_phase_meter", Some("Audio.phase_meter"))));
        assert!(tools.contains(&(
            "audio:fx_loudness_meter",
            Some("Audio.loudness_meter")
        )));
        assert!(tools.contains(&("audio:fx_spectrum", Some("Audio.spectrum"))));
        assert_eq!(tools.len(), 13);
    }

    #[test]
    fn image_edit_binds_wave22_image_caps() {
        let registry = super::build_registry();
        let image = registry.toolbox("image").expect("image toolbox");
        let chain = image
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "image:edit")
            .expect("image:edit toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("image:edit_new", Some("Image.new"))));
        assert!(tools.contains(&("image:edit_add_layer", Some("Image.add_layer"))));
        assert!(tools.contains(&("image:edit_remove_layer", Some("Image.remove_layer"))));
        assert!(tools.contains(&("image:edit_set_pixel", Some("Image.set_pixel"))));
        assert!(tools.contains(&("image:edit_fill", Some("Image.fill"))));
        assert!(tools.contains(&("image:edit_brush", Some("Image.brush"))));
        assert!(tools.contains(&(
            "image:edit_apply_filter",
            Some("Image.apply_filter")
        )));
        assert!(tools.contains(&("image:edit_set_opacity", Some("Image.set_opacity"))));
        assert!(tools.contains(&(
            "image:edit_set_blend_mode",
            Some("Image.set_blend_mode")
        )));
        assert!(tools.contains(&("image:edit_set_visible", Some("Image.set_visible"))));
        assert!(tools.contains(&("image:edit_set_mask", Some("Image.set_mask"))));
        assert!(tools.contains(&("image:edit_clear_mask", Some("Image.clear_mask"))));
        assert!(tools.contains(&("image:edit_composite", Some("Image.composite"))));
        assert!(tools.contains(&(
            "image:edit_add_selection",
            Some("Image.add_selection")
        )));
        assert!(tools.contains(&(
            "image:edit_clear_selections",
            Some("Image.clear_selections")
        )));
        assert_eq!(tools.len(), 15);
    }

    #[test]
    fn ai_nlp_binds_wave21_nlp_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:nlp")
            .expect("ai:nlp toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("ai:nlp_tokenize", Some("NLP.tokenize"))));
        assert!(tools.contains(&("ai:nlp_split_sentences", Some("NLP.split_sentences"))));
        assert!(tools.contains(&("ai:nlp_coref_resolve", Some("NLP.coref_resolve"))));
        assert!(tools.contains(&("ai:nlp_frame_extract", Some("NLP.frame_extract"))));
        assert!(tools.contains(&("ai:nlp_fst_lookup", Some("NLP.fst_lookup"))));
        assert!(tools.contains(&("ai:nlp_gazetteer_build", Some("NLP.gazetteer_build"))));
        assert!(tools.contains(&("ai:nlp_graphrag_query", Some("NLP.graphrag_query"))));
        assert!(tools.contains(&("ai:nlp_relation_extract", Some("NLP.relation_extract"))));
        assert!(tools.contains(&("ai:nlp_substrate_extract", Some("NLP.substrate_extract"))));
        assert_eq!(tools.len(), 9);
    }

    #[test]
    fn dmx_live_binds_wave23_dmx_caps() {
        let registry = super::build_registry();
        let spatial = registry.toolbox("spatial").expect("spatial toolbox");
        let chain = spatial
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "dmx:live")
            .expect("dmx:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("dmx:live_new_universe", Some("Dmx.new_universe"))));
        assert!(tools.contains(&("dmx:live_set_channel", Some("Dmx.set_channel"))));
        assert!(tools.contains(&("dmx:live_add_fixture", Some("Dmx.add_fixture"))));
        assert!(tools.contains(&(
            "dmx:live_fixture_set_colour",
            Some("Dmx.fixture_set_colour")
        )));
        assert!(tools.contains(&(
            "dmx:live_fixture_set_intensity",
            Some("Dmx.fixture_set_intensity")
        )));
        assert!(tools.contains(&(
            "dmx:live_fixture_set_pan_tilt",
            Some("Dmx.fixture_set_pan_tilt")
        )));
        assert!(tools.contains(&("dmx:live_new_cue", Some("Dmx.new_cue"))));
        assert!(tools.contains(&("dmx:live_cue_set_channel", Some("Dmx.cue_set_channel"))));
        assert!(tools.contains(&("dmx:live_cue_set_fade", Some("Dmx.cue_set_fade"))));
        assert!(tools.contains(&("dmx:live_new_cue_stack", Some("Dmx.new_cue_stack"))));
        assert!(tools.contains(&("dmx:live_cue_stack_add", Some("Dmx.cue_stack_add"))));
        assert!(tools.contains(&("dmx:live_cue_stack_go", Some("Dmx.cue_stack_go"))));
        assert!(tools.contains(&(
            "dmx:live_cue_stack_go_back",
            Some("Dmx.cue_stack_go_back")
        )));
        assert!(tools.contains(&("dmx:live_cue_stack_reset", Some("Dmx.cue_stack_reset"))));
        assert_eq!(tools.len(), 14);
    }

    #[test]
    fn video_live_binds_wave23_video_caps() {
        let registry = super::build_registry();
        let image = registry.toolbox("image").expect("image toolbox");
        let chain = image
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "video:live")
            .expect("video:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("video:live_new_project", Some("Video.new_project"))));
        assert!(tools.contains(&("video:live_add_track", Some("Video.add_track"))));
        assert!(tools.contains(&("video:live_add_clip", Some("Video.add_clip"))));
        assert!(tools.contains(&("video:live_trim_clip", Some("Video.trim_clip"))));
        assert!(tools.contains(&("video:live_set_speed", Some("Video.set_speed"))));
        assert!(tools.contains(&("video:live_colour_grade", Some("Video.colour_grade"))));
        assert!(tools.contains(&(
            "video:live_add_transition",
            Some("Video.add_transition")
        )));
        assert!(tools.contains(&(
            "video:live_set_render_format",
            Some("Video.set_render_format")
        )));
        assert!(tools.contains(&(
            "video:live_set_render_bitrate",
            Some("Video.set_render_bitrate")
        )));
        assert!(tools.contains(&("video:live_remove_clip", Some("Video.remove_clip"))));
        assert_eq!(tools.len(), 10);
    }

    #[test]
    fn hid_live_binds_wave23_hid_caps() {
        let registry = super::build_registry();
        let comm = registry.toolbox("communication").expect("communication toolbox");
        let chain = comm
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "hid:live")
            .expect("hid:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("hid:live_poll", Some("HID.poll"))));
        assert!(tools.contains(&("hid:live_wait", Some("HID.wait"))));
        assert!(tools.contains(&("hid:live_clear", Some("HID.clear"))));
        assert!(tools.contains(&("hid:live_pointer_capture", Some("HID.pointer_capture"))));
        assert!(tools.contains(&("hid:live_pointer_release", Some("HID.pointer_release"))));
        assert!(tools.contains(&("hid:live_set_cursor", Some("HID.set_cursor"))));
        assert!(tools.contains(&("hid:live_gamepad_poll", Some("HID.gamepad_poll"))));
        assert!(tools.contains(&("hid:live_gamepad_vibrate", Some("HID.gamepad_vibrate"))));
        assert!(tools.contains(&("hid:live_midi_send", Some("HID.midi_send"))));
        assert!(tools.contains(&("hid:live_midi_poll", Some("HID.midi_poll"))));
        assert!(tools.contains(&("hid:live_haptic_pulse", Some("HID.haptic_pulse"))));
        assert!(tools.contains(&("hid:live_haptic_pattern", Some("HID.haptic_pattern"))));
        assert!(tools.contains(&(
            "hid:live_spatial_head_pose",
            Some("HID.spatial_head_pose")
        )));
        assert!(tools.contains(&(
            "hid:live_spatial_hand_skeleton",
            Some("HID.spatial_hand_skeleton")
        )));
        assert!(tools.contains(&("hid:live_spatial_gaze_ray", Some("HID.spatial_gaze_ray"))));
        assert!(tools.contains(&("hid:live_biosignal_poll", Some("HID.biosignal_poll"))));
        assert_eq!(tools.len(), 16);
    }

    #[test]
    fn vc_binds_wave24_vector_calculus_caps() {
        let registry = super::build_registry();
        let sci = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = sci
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:vc")
            .expect("scientific:vc toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("scientific:vc_gradient", Some("VectorCalculus.gradient"))));
        assert!(tools.contains(&(
            "scientific:vc_divergence",
            Some("VectorCalculus.divergence")
        )));
        assert!(tools.contains(&("scientific:vc_curl", Some("VectorCalculus.curl"))));
        assert!(tools.contains(&("scientific:vc_laplacian", Some("VectorCalculus.laplacian"))));
        assert!(tools.contains(&(
            "scientific:vc_line_integral_scalar",
            Some("VectorCalculus.line_integral_scalar")
        )));
        assert!(tools.contains(&(
            "scientific:vc_line_integral_work",
            Some("VectorCalculus.line_integral_work")
        )));
        assert!(tools.contains(&(
            "scientific:vc_surface_flux",
            Some("VectorCalculus.surface_flux")
        )));
        assert_eq!(tools.len(), 7);
    }

    #[test]
    fn interp_binds_wave24_interpolation_caps() {
        let registry = super::build_registry();
        let sci = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = sci
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:interp")
            .expect("scientific:interp toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:interp_linear",
            Some("Interpolation.linear_interp")
        )));
        assert!(tools.contains(&(
            "scientific:interp_lagrange",
            Some("Interpolation.lagrange_eval")
        )));
        assert!(tools.contains(&(
            "scientific:interp_newton_coef",
            Some("Interpolation.newton_coefficients")
        )));
        assert!(tools.contains(&(
            "scientific:interp_newton_eval",
            Some("Interpolation.newton_eval")
        )));
        assert!(tools.contains(&("scientific:interp_poly_fit", Some("Interpolation.poly_fit"))));
        assert!(tools.contains(&(
            "scientific:interp_poly_eval",
            Some("Interpolation.poly_eval")
        )));
        assert_eq!(tools.len(), 6);
    }

    #[test]
    fn spectral_binds_wave24_spectral_caps() {
        let registry = super::build_registry();
        let sci = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = sci
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:spectral")
            .expect("scientific:spectral toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:spectral_emf_to_spd",
            Some("Spectral.emf_to_spd")
        )));
        assert!(tools.contains(&(
            "scientific:spectral_spd_to_xyz",
            Some("Spectral.spd_to_xyz")
        )));
        assert!(tools.contains(&(
            "scientific:spectral_emf_to_rgb",
            Some("Spectral.emf_to_rgb")
        )));
        assert!(tools.contains(&("scientific:spectral_blend", Some("Spectral.blend"))));
        assert!(tools.contains(&(
            "scientific:spectral_gamut_map",
            Some("Spectral.gamut_map")
        )));
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn world_binds_wave24_world_caps() {
        let registry = super::build_registry();
        let spatial = registry.toolbox("spatial").expect("spatial toolbox");
        let chain = spatial
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "spatial:world")
            .expect("spatial:world toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("spatial:world_new", Some("World.new"))));
        assert!(tools.contains(&("spatial:world_add_object", Some("World.add_object"))));
        assert!(tools.contains(&("spatial:world_add_portal", Some("World.add_portal"))));
        assert!(tools.contains(&("spatial:world_add_avatar", Some("World.add_avatar"))));
        assert!(tools.contains(&("spatial:world_set_gravity", Some("World.set_gravity"))));
        assert!(tools.contains(&(
            "spatial:world_object_apply_force",
            Some("World.object_apply_force")
        )));
        assert!(tools.contains(&(
            "spatial:world_object_step_physics",
            Some("World.object_step_physics")
        )));
        assert_eq!(tools.len(), 7);
    }

    #[test]
    fn asset_binds_wave25_asset_caps() {
        let registry = super::build_registry();
        let office = registry.toolbox("office").expect("office toolbox");
        let chain = office
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "office:asset")
            .expect("office:asset toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("office:asset_create", Some("Asset.create"))));
        assert!(tools.contains(&("office:asset_add_temporal", Some("Asset.add_temporal"))));
        assert!(tools.contains(&("office:asset_add_topic", Some("Asset.add_topic"))));
        assert!(tools.contains(&("office:asset_set_spatial", Some("Asset.set_spatial"))));
        assert!(tools.contains(&("office:asset_compile", Some("Asset.compile"))));
        assert!(tools.contains(&("office:asset_temporal_span", Some("Asset.temporal_span"))));
        assert!(tools.contains(&("office:asset_query_aspects", Some("Asset.query_aspects"))));
        assert!(tools.contains(&("office:asset_persist", Some("Asset.persist"))));
        assert!(tools.contains(&("office:asset_resolve", Some("Asset.resolve"))));
        assert!(tools.contains(&(
            "office:asset_resolve_by_spatial",
            Some("Asset.resolve_by_spatial")
        )));
        assert!(tools.contains(&("office:asset_resolve_by_topic", Some("Asset.resolve_by_topic"))));
        assert!(tools.contains(&(
            "office:asset_resolve_by_temporal",
            Some("Asset.resolve_by_temporal")
        )));
        assert!(tools.contains(&("office:asset_list", Some("Asset.list"))));
        assert!(tools.contains(&("office:asset_count", Some("Asset.count"))));
        assert_eq!(tools.len(), 21);
    }

    #[test]
    fn asset_binds_wave26_persist_caps() {
        let registry = super::build_registry();
        let office = registry.toolbox("office").expect("office toolbox");
        let chain = office
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "office:asset")
            .expect("office:asset toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "office:asset_persist_create",
            Some("Asset.persist_create")
        )));
        assert!(tools.contains(&(
            "office:asset_persist_add_temporal",
            Some("Asset.persist_add_temporal")
        )));
        assert!(tools.contains(&(
            "office:asset_persist_add_topic",
            Some("Asset.persist_add_topic")
        )));
        assert!(tools.contains(&(
            "office:asset_persist_set_spatial",
            Some("Asset.persist_set_spatial")
        )));
        assert!(tools.contains(&(
            "office:asset_persist_compile",
            Some("Asset.persist_compile")
        )));
        assert!(tools.contains(&(
            "office:asset_persist_temporal_span",
            Some("Asset.persist_temporal_span")
        )));
        assert!(tools.contains(&(
            "office:asset_persist_query_aspects",
            Some("Asset.persist_query_aspects")
        )));
        assert_eq!(tools.len(), 21);
    }

    #[test]
    fn ode_binds_wave25_symbolic_ode_caps() {
        let registry = super::build_registry();
        let sci = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = sci
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:ode")
            .expect("scientific:ode toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:ode_lin1",
            Some("SymbolicODE.solve_linear_first_order")
        )));
        assert!(tools.contains(&(
            "scientific:ode_lin2",
            Some("SymbolicODE.solve_linear_second_order")
        )));
        assert!(tools.contains(&(
            "scientific:ode_classify_pde",
            Some("SymbolicODE.classify_second_order_pde")
        )));
        assert!(tools.contains(&("scientific:ode_separable", Some("SymbolicODE.solve_separable"))));
        assert!(tools.contains(&(
            "scientific:ode_pde1",
            Some("SymbolicODE.solve_first_order_linear_pde")
        )));
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn agent_binds_wave25_agent_caps() {
        let registry = super::build_registry();
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let chain = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:agent_live")
            .expect("ai:agent_live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("ai:agent_trace", Some("Agent.trace"))));
        assert!(tools.contains(&("ai:agent_verify", Some("Agent.verify"))));
        assert!(tools.contains(&("ai:agent_plan", Some("Agent.plan"))));
        assert!(tools.contains(&("ai:agent_execute", Some("Agent.execute"))));
        assert!(tools.contains(&("ai:agent_evaluate", Some("Agent.evaluate"))));
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn pulse_live_binds_wave26_pulse_caps() {
        let registry = super::build_registry();
        let comm = registry
            .toolbox("communication")
            .expect("communication toolbox");
        let chain = comm
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "comm:pulse_live")
            .expect("comm:pulse_live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("comm:pulse_live_publish", Some("Pulse.publish"))));
        assert!(tools.contains(&(
            "comm:pulse_live_graph_mutation",
            Some("Pulse.publish_graph_mutation")
        )));
        assert!(tools.contains(&(
            "comm:pulse_live_notification",
            Some("Pulse.publish_notification")
        )));
        assert!(tools.contains(&(
            "comm:pulse_live_telemetry",
            Some("Pulse.publish_telemetry")
        )));
        assert!(tools.contains(&(
            "comm:pulse_live_agent_message",
            Some("Pulse.publish_agent_message")
        )));
        assert!(tools.contains(&("comm:pulse_live_sync", Some("Pulse.publish_sync"))));
        assert!(tools.contains(&("comm:pulse_live_open_channel", Some("Pulse.open_channel"))));
        assert!(tools.contains(&("comm:pulse_live_close_channel", Some("Pulse.close_channel"))));
        assert!(tools.contains(&(
            "comm:pulse_live_set_transport",
            Some("Pulse.set_transport")
        )));
        assert_eq!(tools.len(), 9);
    }

    #[test]
    fn portal_binds_wave26_portal_avatar_caps() {
        let registry = super::build_registry();
        let spatial = registry.toolbox("spatial").expect("spatial toolbox");
        let chain = spatial
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "spatial:portal")
            .expect("spatial:portal toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("spatial:portal_set_target", Some("Portal.set_target"))));
        assert!(tools.contains(&("spatial:portal_activate", Some("Portal.activate"))));
        assert!(tools.contains(&(
            "spatial:portal_deactivate",
            Some("Portal.deactivate")
        )));
        assert!(tools.contains(&("spatial:avatar_move", Some("Avatar.move"))));
        assert!(tools.contains(&(
            "spatial:avatar_set_appearance",
            Some("Avatar.set_appearance")
        )));
        assert_eq!(tools.len(), 5);
    }

    #[test]
    fn research_live_binds_wave27_research_caps() {
        let registry = super::build_registry();
        let epi = registry.toolbox("epistemic").expect("epistemic toolbox");
        let chain = epi
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "research:live")
            .expect("research:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("research:live_new", Some("Research.new"))));
        assert!(tools.contains(&(
            "research:live_set_purpose",
            Some("Research.set_purpose")
        )));
        assert!(tools.contains(&(
            "research:live_define_scope",
            Some("Research.define_scope")
        )));
        assert!(tools.contains(&(
            "research:live_add_constraint",
            Some("Research.add_constraint")
        )));
        assert!(tools.contains(&(
            "research:live_add_question",
            Some("Research.add_question")
        )));
        assert!(tools.contains(&(
            "research:live_link_questions",
            Some("Research.link_questions")
        )));
        assert!(tools.contains(&(
            "research:live_add_corpus_item",
            Some("Research.add_corpus_item")
        )));
        assert!(tools.contains(&(
            "research:live_import_literature",
            Some("Research.import_literature")
        )));
        assert!(tools.contains(&(
            "research:live_import_dataset",
            Some("Research.import_dataset")
        )));
        assert!(tools.contains(&(
            "research:live_set_corpus_confidence",
            Some("Research.set_corpus_confidence")
        )));
        assert!(tools.contains(&(
            "research:live_extract_from_corpus",
            Some("Research.extract_from_corpus")
        )));
        assert!(tools.contains(&(
            "research:live_infer_dark_link",
            Some("Research.infer_dark_link")
        )));
        assert!(tools.contains(&(
            "research:live_detect_provenance_gaps",
            Some("Research.detect_provenance_gaps")
        )));
        assert!(tools.contains(&(
            "research:live_detect_concealment",
            Some("Research.detect_concealment")
        )));
        assert!(tools.contains(&(
            "research:live_confirm_dark_link",
            Some("Research.confirm_dark_link")
        )));
        assert!(tools.contains(&(
            "research:live_refute_dark_link",
            Some("Research.refute_dark_link")
        )));
        assert!(tools.contains(&(
            "research:live_make_inference",
            Some("Research.make_inference")
        )));
        assert!(tools.contains(&(
            "research:live_chain_inference",
            Some("Research.chain_inference")
        )));
        assert!(tools.contains(&(
            "research:live_set_inference_confidence",
            Some("Research.set_inference_confidence")
        )));
        assert!(tools.contains(&(
            "research:live_validate_inference",
            Some("Research.validate_inference")
        )));
        assert_eq!(tools.len(), 73);
    }

    #[test]
    fn research_live_binds_wave28_investigation_caps() {
        let registry = super::build_registry();
        let epi = registry.toolbox("epistemic").expect("epistemic toolbox");
        let chain = epi
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "research:live")
            .expect("research:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "research:live_new_investigation",
            Some("Research.new_investigation")
        )));
        assert!(tools.contains(&(
            "research:live_collect_evidence",
            Some("Research.collect_evidence")
        )));
        assert!(tools.contains(&(
            "research:live_set_reliability",
            Some("Research.set_reliability")
        )));
        assert!(tools.contains(&(
            "research:live_propose_hypothesis",
            Some("Research.propose_hypothesis")
        )));
        assert!(tools.contains(&(
            "research:live_evaluate_evidence",
            Some("Research.evaluate_evidence")
        )));
        assert!(tools.contains(&(
            "research:live_create_timeline",
            Some("Research.create_timeline")
        )));
        assert!(tools.contains(&("research:live_add_link", Some("Research.add_link"))));
        assert!(tools.contains(&("research:live_find_path", Some("Research.find_path"))));
        assert!(tools.contains(&(
            "research:live_create_hypothesis_graph",
            Some("Research.create_hypothesis_graph")
        )));
        assert!(tools.contains(&(
            "research:live_contribute_evaluation",
            Some("Research.contribute_evaluation")
        )));
        assert!(tools.contains(&(
            "research:live_bridge_dark_link",
            Some("Research.bridge_dark_link")
        )));
        assert!(tools.contains(&(
            "research:live_reframe_hypothesis",
            Some("Research.reframe_hypothesis")
        )));
        assert!(tools.contains(&(
            "research:live_merge_hypotheses",
            Some("Research.merge_hypotheses")
        )));
        assert!(tools.contains(&("research:live_flag_gap", Some("Research.flag_gap"))));
        assert!(tools.contains(&("research:live_close_gap", Some("Research.close_gap"))));
        assert!(tools.contains(&(
            "research:live_create_revision",
            Some("Research.create_revision")
        )));
        assert!(tools.contains(&(
            "research:live_diff_revisions",
            Some("Research.diff_revisions")
        )));
        assert!(tools.contains(&(
            "research:live_subscribe_updates",
            Some("Research.subscribe_updates")
        )));
        assert!(tools.contains(&(
            "research:live_create_assessment",
            Some("Research.create_assessment")
        )));
        assert_eq!(tools.len(), 73);
    }

    #[test]
    fn research_live_binds_wave29_remainder_caps() {
        let registry = super::build_registry();
        let epi = registry.toolbox("epistemic").expect("epistemic toolbox");
        let chain = epi
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "research:live")
            .expect("research:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "research:live_set_reality_category",
            Some("Research.set_reality_category")
        )));
        assert!(tools.contains(&(
            "research:live_classify_reality",
            Some("Research.classify_reality")
        )));
        assert!(tools.contains(&(
            "research:live_assess_sentiment",
            Some("Research.assess_sentiment")
        )));
        assert!(tools.contains(&(
            "research:live_register_perspective",
            Some("Research.register_perspective")
        )));
        assert!(tools.contains(&(
            "research:live_analyse_inequality",
            Some("Research.analyse_inequality")
        )));
        assert!(tools.contains(&(
            "research:live_detect_ug_patterns",
            Some("Research.detect_ug_patterns")
        )));
        assert_eq!(tools.len(), 73);
    }

    #[test]
    fn render_live_binds_wave30_render_caps() {
        let registry = super::build_registry();
        let image = registry.toolbox("image").expect("image toolbox");
        let chain = image
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "render:live")
            .expect("render:live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("render:live_scene", Some("Render.scene"))));
        assert!(tools.contains(&(
            "render:live_css_animation",
            Some("Render.css_animation")
        )));
        assert!(tools.contains(&("render:live_css_color", Some("Render.css_color"))));
        assert!(tools.contains(&(
            "render:live_css_transform",
            Some("Render.css_transform")
        )));
        assert!(tools.contains(&(
            "render:live_animation_eval_curve",
            Some("Render.animation_eval_curve")
        )));
        assert!(tools.contains(&(
            "render:live_animation_spring_step",
            Some("Render.animation_spring_step")
        )));
        assert!(tools.contains(&(
            "render:live_animation_sclerp",
            Some("Render.animation_sclerp")
        )));
        assert!(tools.contains(&(
            "render:live_animation_eval_preset",
            Some("Render.animation_eval_preset")
        )));
        assert!(tools.contains(&(
            "render:live_animation_squad_step",
            Some("Render.animation_squad_step")
        )));
        assert!(tools.contains(&(
            "render:live_animation_list_presets",
            Some("Render.animation_list_presets")
        )));
        assert!(tools.contains(&(
            "render:live_animation_compute_pass",
            Some("Render.animation_compute_pass")
        )));
        assert!(tools.contains(&("render:live_svg_path", Some("Render.svg_path"))));
        assert!(tools.contains(&("render:live_svg_circle", Some("Render.svg_circle"))));
        assert!(tools.contains(&("render:live_svg_rect", Some("Render.svg_rect"))));
        assert!(tools.contains(&("render:live_svg_line", Some("Render.svg_line"))));
        assert!(tools.contains(&("render:live_svg_bezier", Some("Render.svg_bezier"))));
        assert!(tools.contains(&("render:live_svg_field", Some("Render.svg_field"))));
        assert_eq!(tools.len(), 17);
    }

    #[test]
    fn cg_live_binds_wave31_cg_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:cg_live")
            .expect("scientific:cg_live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:cg_live_average_spacing_3d",
            Some("ComputationalGeometry.average_spacing_3d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_live_incircle",
            Some("ComputationalGeometry.incircle")
        )));
        assert!(tools.contains(&(
            "scientific:cg_live_insphere",
            Some("ComputationalGeometry.insphere")
        )));
        assert!(tools.contains(&(
            "scientific:cg_live_nearest_segment_site",
            Some("ComputationalGeometry.nearest_segment_site")
        )));
        assert!(tools.len() >= 25);
    }

    #[test]
    fn cg_live_binds_wave32_cg_remainder_caps() {
        let registry = super::build_registry();
        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let chain = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:cg_live")
            .expect("scientific:cg_live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "scientific:cg_live_width_coreset",
            Some("ComputationalGeometry.width_coreset")
        )));
        assert!(tools.contains(&(
            "scientific:cg_live_cross_ratio_1d",
            Some("ComputationalGeometry.cross_ratio_1d")
        )));
        assert!(tools.contains(&(
            "scientific:cg_live_separating_plane_aabb",
            Some("ComputationalGeometry.separating_plane_aabb")
        )));
        assert_eq!(tools.len(), 49);
    }

    #[test]
    fn wave33_binds_anim_ode_hbbtv_caps() {
        let registry = super::build_registry();
        let spatial = registry.toolbox("spatial").expect("spatial toolbox");
        let anim = spatial
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "spatial:anim_live")
            .expect("spatial:anim_live toolchain");
        let anim_tools: Vec<_> = anim
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools_contains(
            &anim_tools,
            "animation:live_spring_step",
            "Animation.spring_step"
        ));
        assert_eq!(anim_tools.len(), 4);

        let scientific = registry.toolbox("scientific").expect("scientific toolbox");
        let ode = scientific
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:num_ode")
            .expect("scientific:num_ode toolchain");
        assert_eq!(ode.tools().len(), 4);
        assert!(ode.tools().iter().any(|t| {
            t.metadata().id == "scientific:num_ode_rk4"
                && t.metadata().capability_scope.as_deref() == Some("Ode.rk4_integrate")
        }));

        let comm = registry.toolbox("communication").expect("communication toolbox");
        let hbbtv = comm
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "comm:hbbtv")
            .expect("comm:hbbtv toolchain");
        assert_eq!(hbbtv.tools().len(), 4);
        assert!(hbbtv.tools().iter().any(|t| {
            t.metadata().id == "comm:hbbtv_new_app"
                && t.metadata().capability_scope.as_deref() == Some("HbbTV.new_app")
        }));
    }

    fn tools_contains(tools: &[(&str, Option<&str>)], id: &str, scope: &str) -> bool {
        tools.contains(&(id, Some(scope)))
    }

    #[test]
    fn gpu_live_binds_wave34_gpu_caps() {
        let registry = super::build_registry();
        let image = registry.toolbox("image").expect("image toolbox");
        let chain = image
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "render:gpu_live")
            .expect("render:gpu_live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&("render:gpu_live_init", Some("Render.gpu_init"))));
        assert!(tools.contains(&(
            "render:gpu_live_backend_info",
            Some("Render.gpu_backend_info")
        )));
        assert!(tools.len() >= 17);
    }

    #[test]
    fn gpu_live_binds_wave35_gpu_emf_caps() {
        let registry = super::build_registry();
        let image = registry.toolbox("image").expect("image toolbox");
        let chain = image
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "render:gpu_live")
            .expect("render:gpu_live toolchain");
        let tools: Vec<_> = chain
            .tools()
            .iter()
            .map(|tool| {
                (
                    tool.metadata().id.as_str(),
                    tool.metadata().capability_scope.as_deref(),
                )
            })
            .collect();
        assert!(tools.contains(&(
            "render:gpu_live_upload_mesh_colored",
            Some("Render.gpu_upload_mesh_colored")
        )));
        assert!(tools.contains(&(
            "render:gpu_live_emf_field_info",
            Some("Render.emf_field_info")
        )));
        assert_eq!(tools.len(), 34);
    }

    #[test]
    fn wave36_binds_social_finance_graph_caps() {
        let registry = super::build_registry();
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let social = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "social:live")
            .expect("social:live toolchain");
        assert_eq!(social.tools().len(), 6);
        assert!(social.tools().iter().any(|t| {
            t.metadata().id == "social:live_gini"
                && t.metadata().capability_scope.as_deref() == Some("Social.gini")
        }));
        let finance = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "finance:live")
            .expect("finance:live toolchain");
        assert_eq!(finance.tools().len(), 3);
        let comm = registry.toolbox("communication").expect("communication toolbox");
        let graph = comm
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "comm:graph_live")
            .expect("comm:graph_live toolchain");
        assert_eq!(graph.tools().len(), 7);
        assert!(graph.tools().iter().any(|t| {
            t.metadata().id == "comm:graph_live_validate_fragment"
                && t.metadata().capability_scope.as_deref() == Some("ChatGraph.validate_fragment")
        }));
    }

    #[test]
    fn wave37_binds_graph_opt_sampler_caps() {
        let registry = super::build_registry();
        let sci = registry.toolbox("scientific").expect("scientific toolbox");
        let graph = sci
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:graph_reason")
            .expect("scientific:graph_reason toolchain");
        assert_eq!(graph.tools().len(), 9);
        assert!(graph.tools().iter().any(|t| {
            t.metadata().id == "scientific:graph_live_fuzzy_jaccard"
                && t.metadata().capability_scope.as_deref() == Some("GraphMatch.fuzzy_jaccard")
        }));
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let sampler = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:sampler_live")
            .expect("ai:sampler_live toolchain");
        assert_eq!(sampler.tools().len(), 10);
        assert!(sampler.tools().iter().any(|t| {
            t.metadata().id == "ai:sampler_live_configure"
                && t.metadata().capability_scope.as_deref() == Some("sampler.configure")
        }));
    }

    #[test]
    fn wave38_binds_med_manifold_crypto_dag_fm_caps() {
        let registry = super::build_registry();
        let health = registry.toolbox("health").expect("health toolbox");
        let med = health
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "health:med_live")
            .expect("health:med_live toolchain");
        assert_eq!(med.tools().len(), 5);
        assert!(med.tools().iter().any(|t| {
            t.metadata().id == "health:med_live_tanimoto"
                && t.metadata().capability_scope.as_deref() == Some("Medical.tanimoto")
        }));
        let spatial = registry.toolbox("spatial").expect("spatial toolbox");
        let manifold = spatial
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "spatial:manifold_live")
            .expect("spatial:manifold_live toolchain");
        assert_eq!(manifold.tools().len(), 3);
        let sci = registry.toolbox("scientific").expect("scientific toolbox");
        let crypto = sci
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:crypto_priv")
            .expect("scientific:crypto_priv toolchain");
        assert_eq!(crypto.tools().len(), 6);
        assert!(crypto.tools().iter().any(|t| {
            t.metadata().id == "scientific:gemm_live"
                && t.metadata().capability_scope.as_deref() == Some("LinearAlgebra.gemm")
        }));
        let ai = registry.toolbox("ai").expect("ai toolbox");
        let disc = ai
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "ai:disc_dag")
            .expect("ai:disc_dag toolchain");
        assert_eq!(disc.tools().len(), 5);
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let fm = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:fm_live")
            .expect("econ:fm_live toolchain");
        assert_eq!(fm.tools().len(), 2);
    }

    #[test]
    fn wave39_binds_remaining_curated_q2_singles() {
        let registry = super::build_registry();
        let sci = registry.toolbox("scientific").expect("scientific toolbox");
        let longtail = sci
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "scientific:longtail")
            .expect("scientific:longtail toolchain");
        assert_eq!(longtail.tools().len(), 16);
        assert!(longtail.tools().iter().any(|t| {
            t.metadata().id == "scientific:longtail_ltl_finally"
                && t.metadata().capability_scope.as_deref()
                    == Some("TemporalAndDescriptionLogic.ltl.finally")
        }));
        let econ = registry.toolbox("econ").expect("econ toolbox");
        let wealth = econ
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "econ:wealth_live")
            .expect("econ:wealth_live toolchain");
        assert_eq!(wealth.tools().len(), 3);
        let comm = registry.toolbox("communication").expect("communication toolbox");
        let net = comm
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "comm:net_live")
            .expect("comm:net_live toolchain");
        assert_eq!(net.tools().len(), 2);
        let rights = registry.toolbox("rights").expect("rights toolbox");
        let id_live = rights
            .chains()
            .iter()
            .find(|chain| chain.metadata().id == "rights:id_live")
            .expect("rights:id_live toolchain");
        assert_eq!(id_live.tools().len(), 3);
        assert!(id_live.tools().iter().any(|t| {
            t.metadata().id == "rights:id_live_agency"
                && t.metadata().capability_scope.as_deref() == Some("Agency.evaluate")
        }));
    }
}
