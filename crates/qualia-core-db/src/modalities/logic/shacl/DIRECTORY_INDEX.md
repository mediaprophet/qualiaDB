---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# shacl Index

## Functionality Overview
Comprehensive index of functionality for `shacl`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
- 📄 `shacl_compiler.rs`
  - `struct ShaclCompiler`
  - `impl ShaclCompiler`
  - `fn new`
  - `fn compile`
  - `fn compile_class`
  - `fn compile_shape`
  - `fn compile_constraints`
  - `fn compile_extension_shape`
  - `fn push_constraint`
  - `fn push_terminal`
  - `fn parse_str`
  - `fn node_kind_to_tag`
  - `fn datatype_to_tag`
  - `fn compile_shape_returns_opcodes`
  - `fn or_constraint_emits_soft_checks`
  - *(...and 1 more)*
- 📄 `shacl_extension_bridge.rs`
  - `fn append_extension_opcodes`
  - `fn epistemic_extension_appends_opcodes`
  - `fn computational_maths_extensions_append_opcodes`
- 📄 `shacl_types.rs`
  - `enum ShaclSeverity`
  - `enum ProteinScoringMatrix`
  - `enum ClinicalRiskModel`
  - `enum CalcComputeTarget`
  - `enum NodeKindType`
  - `enum PropertyPath`
  - `enum ShaclTarget`
  - `struct ValidationReport`
  - `struct ValidationResult`
  - `enum ShaclConstraint`
  - `struct CompiledShape`
  - `impl CompiledShape`
  - `fn new`
  - `fn is_empty`
  - `fn evaluate_numeric`
- 📄 `text_input.rs`
  - `struct ShapeSpec`
  - `struct ConstraintSpec`
  - `fn term_str`
  - `fn intern`
  - `fn encode_object`
  - `fn build_graph`
  - `fn severity_of`
  - `fn constraint_of`
  - `fn shape_from_spec`
  - `fn validate_text`
  - `fn validate_json`
  - `fn end_to_end_age_mininclusive`
  - `fn end_to_end_pattern_with_resolver`
  - `fn end_to_end_class_and_node_kind`
  - `fn invalid_json_is_reported`
- 📄 `validate.rs`
  - `fn object_as_f64`
  - `fn node_kind_of`
  - `fn node_kind_matches`
  - `fn datatype_tag`
  - `fn label`
  - `struct ShaclEngine`
  - `fn new`
  - `fn validate`
  - `fn validate_focus`
  - `fn values_at`
  - `fn is_a`
  - `fn shape_named`
  - `fn focus_conforms`
  - `fn check`
  - `fn values_for_path`
  - *(...and 24 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
