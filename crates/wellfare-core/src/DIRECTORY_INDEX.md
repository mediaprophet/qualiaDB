---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# src Index

## Functionality Overview
Comprehensive index of functionality for `src`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `lib.rs`
  - `fn test_parse_time_offset_minutes`
  - `fn test_parse_samsung_datetime`
  - `fn test_parse_weight_csv`
  - `fn test_parse_sleep_csv`
  - `fn test_parse_heart_rate_csv`
  - `fn test_parse_steps_csv`
- 📄 `models.rs`
  - `struct WeightRecord`
  - `struct SleepRecord`
  - `struct HeartRateRecord`
  - `struct StepRecord`
  - `enum PrivacyMode`
  - `enum DiagnosticReportStatus`
  - `struct PathologyObservation`
  - `struct DiagnosticReport`
- 📄 `n3_rules.rs`
  - `struct N3RuleMatch`
  - `impl N3RuleMatch`
  - `fn to_json`
  - `fn query_avg`
  - `fn evaluate_n3_rules`
  - `fn evaluate_n3_rules_turtle`
  - `fn store_with`
  - `fn test_tachycardia_fires`
  - `fn test_sleep_debt_fires`
  - `fn test_normal_data_no_flags`
  - `fn test_adrenal_fatigue_fires`
- 📄 `parser.rs`
  - `fn clean_csv_content`
  - `fn parse_time_offset_minutes`
  - `fn parse_samsung_datetime`
  - `struct WeightCsvRow`
  - `fn parse_weight_csv`
  - `struct SleepCsvRow`
  - `fn parse_sleep_csv`
  - `struct HeartRateCsvRow`
  - `fn parse_heart_rate_csv`
  - `struct StepCsvRow`
  - `fn parse_steps_csv`
- 📄 `qualia_bindings.rs`
  - `struct QualiaStore`
  - `impl QualiaStore`
  - `fn new`
  - `fn insert_quin`
  - `fn query_subject`
  - `fn query_predicate`
  - `fn query_context`
  - `fn len`
  - `fn clear`
  - `fn insert_from_cbor_ld`
  - `fn parse_cbor_quin`
- 📄 `rdf.rs`
  - `fn generate_rdf_prefixes`
  - `fn vault_meds_to_turtle`
  - `fn vault_diet_to_turtle`
  - `fn vault_biometrics_to_turtle`
  - `fn weight_to_turtle`
  - `fn sleep_to_turtle`
  - `fn heart_rate_to_turtle`
  - `fn steps_to_turtle`
- 📄 `shapes.rs`
  - `struct ShapeViolation`
  - `struct ValidationReport`
  - `impl ValidationReport`
  - `fn is_valid`
  - `fn to_json`
  - `fn validate_turtle`
  - `fn test_valid_sleep_passes`
  - `fn test_invalid_efficiency_caught`
- 📄 `store.rs`
  - `struct HealthStore`
  - `impl HealthStore`
  - `fn new`
  - `fn load_turtle`
  - `fn query`
  - `fn check_shape`
  - `impl Default`
  - `fn default`
  - `fn test_load_and_query`
  - `fn test_ask_shape_violation`
- 📄 `wasm.rs`
  - `fn parse_weight_csv_json`
  - `fn parse_sleep_csv_json`
  - `fn parse_heart_rate_csv_json`
  - `fn parse_steps_csv_json`
  - `fn weight_turtle_from_csv`
  - `fn sleep_turtle_from_csv`
  - `fn heart_rate_turtle_from_csv`
  - `fn steps_turtle_from_csv`
  - `struct WasmHealthStore`
  - `impl WasmHealthStore`
  - `fn new`
  - `fn load_turtle`
  - `fn query`
  - `fn validate_health_turtle`
  - `fn vault_meds_to_turtle`
  - *(...and 4 more)*
- 📄 `webizen.rs`
  - `enum WebizenOpcode`
  - `struct WebizenRule`
  - `struct WebizenVM`
  - `impl WebizenVM`
  - `fn new`
  - `fn load_bytecode`
  - `fn execute`
  - `fn check_threshold`
  - `fn evaluate_policy_constraint`
  - `fn test_numeric_comparison`
  - `fn test_policy_gate_passthrough`
  - `fn test_policy_gate_satisfied`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
