//! Share stateless engine bindings with offline production without a Poet session.

use super::{dispatch_math_stats, econ, ids, stats};
use vibe::{DiagCode, Diagnostic, Span, Value};

pub fn evaluate_statistics(task: &str, args: &Value) -> Result<Value, Diagnostic> {
    let span = Span { start: 0, end: 0 };
    let stats_id = match task {
        "one_sample_t" => ids::STAT_ONE_SAMPLE_T,
        "two_sample_t" => ids::STAT_TWO_SAMPLE_T,
        "paired_t" => ids::STAT_PAIRED_T,
        "mann_whitney_u" => ids::STAT_MANN_WHITNEY_U,
        "chi_square_gof" => ids::STAT_CHI_SQUARE_GOF,
        "chi_square_independence" => ids::STAT_CHI_SQUARE_INDEPENDENCE,
        "one_way_anova" => ids::STAT_ONE_WAY_ANOVA,
        "friedman" => ids::STAT_FRIEDMAN,
        "fisher_distance" => return stats::fisher_distance(args, span),
        "kl_bregman_form" => return stats::kl_bregman_form(args, span),
        "ols" => return econ::econ::econ_ols(args, span),
        "wls" => return econ::econ::econ_wls(args, span),
        "iv_2sls" => return econ::econ::econ_iv_2sls(args, span),
        "leontief_inverse" => return econ::econ::econ_leontief_inverse(args, span),
        "output_multipliers" => return econ::econ::econ_output_multipliers(args, span),
        "gini" => return econ::econ::econ_gini(args, span),
        "atkinson" => return econ::econ::econ_atkinson(args, span),
        "rawlsian_welfare" => return econ::econ::econ_rawlsian_welfare(args, span),
        "vcg_payment" => return econ::econ::econ_vcg_payment(args, span),
        "mixed_nash_2x2" => return econ::econ::econ_mixed_nash_2x2(args, span),
        "bellman_update" => return econ::econ::econ_bellman_update(args, span),
        "social_cost_of_carbon" => return econ::econ::econ_social_cost_of_carbon(args, span),
        "optimal_abatement" => return econ::econ::econ_optimal_abatement(args, span),
        "abatement_net_benefit" => return econ::econ::econ_abatement_net_benefit(args, span),
        _ => return Err(Diagnostic::new(DiagCode::E300, span, "Unknown production statistics task")),
    };
    dispatch_math_stats::try_dispatch(stats_id, args, span).unwrap_or_else(|| {
        Err(Diagnostic::new(DiagCode::E300, span, "Production statistics task is unbound"))
    })
}
