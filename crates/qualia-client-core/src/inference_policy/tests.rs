use super::store;
use qualia_core_db::llm_agent::governed::{MAX_POLICY_JSON_BYTES, POLICY_COLD_BUDGET_BYTES};

#[test]
fn input_budget_refusals_do_not_initialize_global_host_or_touch_vault() {
    assert!(
        store::install_policy("{}", 1, 2, 1, 0, POLICY_COLD_BUDGET_BYTES - 1)
            .unwrap_err()
            .contains("Budget")
    );
    let oversized = " ".repeat(MAX_POLICY_JSON_BYTES + 1);
    assert!(
        store::install_policy(&oversized, 1, 2, 1, 0, POLICY_COLD_BUDGET_BYTES)
            .unwrap_err()
            .contains("Capacity")
    );
}

#[test]
fn explicit_configuration_requires_the_separate_installer_key_and_bounded_names() {
    use super::configuration::InferenceHostConfiguration;
    let key = ed25519_dalek::SigningKey::from_bytes(&[91; 32]);
    let mut config = InferenceHostConfiguration {
        installer_identifier: "urn:operator".into(),
        installer_public_key: key.verifying_key().to_bytes(),
        project_id: "project".into(),
        project_issuer: "issuer".into(),
        project_public_key: [7; 32],
    };
    assert!(config.validate(&key).is_ok());
    config.installer_public_key = [4; 32];
    assert!(config.validate(&key).is_err());
    config.installer_public_key = key.verifying_key().to_bytes();
    config.installer_identifier = " ".into();
    assert!(config.validate(&key).is_err());
    config.installer_identifier = "x".repeat(4097);
    assert!(config.validate(&key).is_err());
}
