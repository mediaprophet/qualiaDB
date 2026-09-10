//! Dispatch each scenario identifier to a real library call.

use super::biometrics;
use super::clinical;
use super::custody;
use super::labels;
use super::profiles;
use super::rest;
use super::table::ScenarioId;
use crate::net::qdnf::errors::QdnfError;

/// Run the observational fixture for `id`. Never sets qualification.
pub fn run_fixture(id: ScenarioId) -> Result<(), QdnfError> {
    match id {
        ScenarioId::S01 => labels::s01_missing_label(),
        ScenarioId::S02 => labels::s02_unsigned_mutation(),
        ScenarioId::S03 => labels::s03_summary_inherits(),
        ScenarioId::S04 => labels::s04_disjoint_audience(),
        ScenarioId::S05 => labels::s05_seventeen_compartments(),
        ScenarioId::S06 => labels::s06_stale_consent(),
        ScenarioId::S07 => clinical::s07_revoke_before_deliver(),
        ScenarioId::S08 => clinical::s08_blocked_is_not_grant(),
        ScenarioId::S09 => clinical::s09_ordinary_bytes(),
        ScenarioId::S10 => clinical::s10_key_generation(),
        ScenarioId::S11 => clinical::s11_confidential_help(),
        ScenarioId::S12 => profiles::s12_no_downgrade(),
        ScenarioId::S13 => profiles::s13_mandatory_cover(),
        ScenarioId::S14 => profiles::s14_observer(),
        ScenarioId::S15 => labels::s15_worker_reset(),
        ScenarioId::S16 => labels::s16_restricted_egress(),
        ScenarioId::S17 => labels::s17_embedding_inherits(),
        ScenarioId::S18 => biometrics::s18_match_not_authority(),
        ScenarioId::S19 => biometrics::s19_cross_domain(),
        ScenarioId::S20 => biometrics::s20_non_biometric_recovery(),
        ScenarioId::S21 => profiles::s21_clock_rollback(),
        ScenarioId::S22 => custody::s22_kill_before_receipt(),
        ScenarioId::S23 => custody::s23_overlapping_holds(),
        ScenarioId::S24 => rest::s24_payment_not_consent(),
        ScenarioId::S25 => profiles::s25_gateway_mapping(),
        ScenarioId::S26 => rest::s26_independence(),
        ScenarioId::S27 => rest::s27_public_error(),
        ScenarioId::S28 => clinical::s28_stale_generation(),
        ScenarioId::S29 => custody::s29_restored_backup(),
        ScenarioId::S30 => custody::s30_independent_verifier(),
        ScenarioId::S31 => custody::s31_fhir_export(),
        ScenarioId::S32 => labels::s32_user_edited_marking(),
        ScenarioId::S33 => labels::s33_single_redacted_release(),
        ScenarioId::S34 => biometrics::s34_worker_crash(),
        ScenarioId::S35 => profiles::s35_no_routes(),
        ScenarioId::S36 => labels::s36_combined_authority(),
        ScenarioId::S37 => labels::s37_omitted_dependencies(),
        ScenarioId::S38 => clinical::s38_revoke_before_consume(),
        ScenarioId::S39 => clinical::s39_revoke_after_chunk(),
        ScenarioId::S40 => biometrics::s40_threshold_missing(),
    }
}
