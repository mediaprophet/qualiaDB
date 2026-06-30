---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# wasm_bridge Index

## Functionality Overview
Comprehensive index of functionality for `wasm_bridge`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[engine](engine/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `bio.rs`
  - `fn align_sequences_wasm`
  - `struct AlignResult`
  - `struct FastaParams`
  - `fn validate_fasta_wasm`
  - `struct FastaResult`
  - `fn predict_receptor_binding_wasm`
- 📄 `chemistry.rs`
  - `struct SmilesParams`
  - `fn compute_molecular_descriptors_wasm`
  - `struct Desc`
  - `fn evaluate_lipinski_wasm`
  - `struct Filters`
  - `fn detect_functional_groups_wasm`
  - `struct GroupResult`
  - `struct ReactionMetricsParams`
  - `fn compute_reaction_metrics_wasm`
  - `struct RxnResult`
  - `fn compute_thermochemistry_wasm`
  - `struct ThermResult`
- 📄 `compute.rs`
  - `fn run_semantic_simulation`
  - `struct SimResult`
  - `struct AlignmentParams`
  - `fn compute_pid_step_wasm`
  - `struct PidOut`
  - `struct GbmPathParams`
  - `fn simulate_gbm_path_wasm`
  - `struct GbmOut`
  - `fn black_scholes_wasm`
  - `struct BsOut`
  - `fn solve_sat_wasm`
  - `struct SatInput`
  - `struct SatOut`
  - `struct OdeDecayParams`
  - `fn solve_ode_exponential_decay_wasm`
  - *(...and 1 more)*
- 📄 `dataio.rs`
  - `struct JsonLdFlatTriple`
  - `fn parse_json_wasm`
  - `struct QOut`
  - `struct QuinJson`
  - `struct CsvParseParams`
  - `struct CsvFieldMapping`
  - `fn parse_csv_wasm`
  - `struct ParseResult`
  - `struct JsonParseParams`
  - `struct JsonFieldMapping`
  - `fn parse_json_mapping_wasm`
  - `struct CsvSerializeParams`
  - `fn serialize_csv_wasm`
  - `struct SerializeResult`
  - `struct JsonSerializeParams`
  - *(...and 1 more)*
- 📄 `medical.rs`
  - `struct FraminghamParams`
  - `fn compute_framingham_risk_wasm`
  - `struct RiskResult`
  - `struct FhirObsParams`
  - `fn validate_fhir_observation_wasm`
  - `struct ValidationResult`
  - `fn check_drug_interactions_wasm`
  - `struct Interaction`
- 📄 `meta.rs`
  - `struct SimulationParams`
  - `struct DrugInteractionParams`
  - `struct ThermochemParams`
  - `struct PidStepParams`
  - `fn resolve_lww_wasm`
  - `struct BlackScholesParams`
  - `struct EngineInfo`
  - `fn get_engine_version`
  - `fn get_engine_info`
  - `fn list_capabilities_wasm`
- 📄 `mod.rs`
- 📄 `semantic.rs`
  - `struct ShaclValidateParams`
  - `fn validate_shacl_constraint_wasm`
  - `struct ValidationOut`
  - `fn execute_ntriples_query`
  - `struct MatchOut`
  - `struct Res`
  - `fn compile_query_to_json`
  - `struct InstructionOut`
  - `struct ProgramOut`
  - `fn parse_turtle_wasm`
  - `struct QOut`
  - `fn parse_n3logic_wasm`
  - `fn parse_cbor_ld_wasm`
  - `fn forward_chain_wasm`
  - `struct RuleInput`
  - *(...and 5 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
