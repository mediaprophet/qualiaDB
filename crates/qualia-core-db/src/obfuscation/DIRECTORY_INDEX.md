---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# obfuscation Index

## Functionality Overview
Comprehensive index of functionality for `obfuscation`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `domain_transformer.rs`
  - `struct DomainTransformer`
  - `struct DomainParameters`
  - `enum TransformationState`
  - `impl DomainTransformer`
  - `fn transform_to_domain`
  - `fn get_domain_parameters`
  - `fn apply_transformation`
  - `fn apply_matrix_transformation`
  - `fn apply_polynomial_transformation`
  - `fn apply_hamiltonian_transformation`
  - `fn apply_optimization_transformation`
  - `fn validate_transformation`
  - `fn validate_matrix_transformation`
  - `fn validate_polynomial_transformation`
  - `fn validate_hamiltonian_transformation`
  - *(...and 6 more)*
- 📄 `hybrid_state_manager.rs`
  - `struct HybridStateManager`
  - `struct QuantumState`
  - `struct ClassicalState`
  - `struct ClinicalInferenceState`
  - `struct DefeasibleContext`
  - `struct TemporalPharmacokineticState`
  - `struct ConvergenceTracker`
  - `enum ConvergenceState`
  - `struct DomainStateHandler`
  - `enum HybridStateDomain`
  - `impl HybridStateManager`
  - `fn sync_quantum_to_domain`
  - `fn extract_quantum_state`
  - `fn extract_payload_from_quin`
  - `fn update_classical_state_preserving_reasoning`
  - *(...and 29 more)*
- 📄 `mod.rs`
  - `struct ClassicalState`
  - `struct ClinicalInferenceState`
  - `struct DefeasibleContext`
  - `struct TemporalPharmacokineticState`
  - `impl Default`
  - `fn default`
- 📄 `polynomial_obfuscator.rs`
  - `struct PolynomialObfuscator`
  - `enum ObfuscationDomain`
  - `impl PolynomialObfuscator`
  - `fn encode_to_quin`
  - `fn decode_from_quin`
  - `fn hash_to_fixed_fingerprint`
  - `fn initialize_randomization`
  - `fn generate_polynomial_coefficients`
  - `fn generate_polynomial_system_coeffs`
  - `fn generate_matrix_transformation_coeffs`
  - `fn generate_hamiltonian_operator_coeffs`
  - `fn generate_optimization_problem_coeffs`
  - `fn apply_domain_transformation`
  - `fn pack_into_quin`
  - `fn unpack_from_quin`
  - *(...and 7 more)*
- 📄 `semantic_stripper.rs`
  - `struct SemanticStripper`
  - `struct ContextMapping`
  - `enum RemovalStrategy`
  - `enum StrippingState`
  - `impl SemanticStripper`
  - `fn strip_context`
  - `fn find_context_mapping`
  - `fn extract_mathematical_structure`
  - `fn extract_clinical_structure`
  - `fn extract_biology_structure`
  - `fn extract_linear_algebra_structure`
  - `fn apply_removal_strategy`
  - `fn complete_stripping`
  - `fn structure_preserving_stripping`
  - `fn minimal_stripping`
  - *(...and 24 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
