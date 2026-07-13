---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# governance Index

## Functionality Overview
Comprehensive index of functionality for `governance`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `coordination.rs`
  - `enum CoordFault`
  - `fn eval_authorization_grant`
  - `struct ResourceContract`
  - `impl ResourceContract`
  - `fn tick_cycles`
  - `fn burn_tokens`
  - `fn eval_resource_declaration`
  - `struct PerformanceRecord`
  - `fn eval_performance_rating`
  - `fn require_privileged`
  - `fn compute_priority`
  - `struct CoordContext`
  - `struct CoordOutcome`
  - `enum CoordVmError`
  - `fn perf_vc_hash`
  - *(...and 10 more)*
- 📄 `illocution.rs`
  - `enum EngineState`
  - `fn is_exemptive`
  - `fn effective_weight`
  - `enum Resolution`
  - `struct Norm`
  - `impl Norm`
  - `fn new`
  - `fn exemptive`
  - `fn immune`
  - `fn resolve_conflict`
  - `fn soft_directive_never_overrides_a_hard_commissive`
  - `fn exemptive_overrides_a_derogable_obligation`
  - `fn exemptive_cannot_defeat_a_non_derogable_norm`
  - `fn equal_force_is_a_genuine_paraconsistent_conflict`
  - `fn directive_weight_scales_with_speaker_authority`
- 📄 `mod.rs`
- 📄 `modal_kind.rs`
  - `fn tag_kind`
  - `fn resolve_kind`
  - `fn kind_name`
  - `fn resolves_dictionary_identifier_kind`
  - `fn resolves_full_64bit_identifier_kind`
  - `fn distinct_identifiers_keep_distinct_kinds`
  - `fn unkinded_identifier_resolves_none`
  - `fn kinds_are_distinct_and_tag_free`
- 📄 `provenance.rs`
  - `fn label_with_worker_did`
  - `fn record_moderation`
  - `fn record_cleaning`
  - `fn full_attribution`
  - `fn contest_assertion`
  - `fn resolve_contest`
  - `fn write_activity`
  - `fn is_contested`
  - `fn labellers`
  - `fn make_prov`
  - `fn make_contest`
  - `fn label_produces_correct_context`
  - `fn contest_produces_four_quins`
  - `fn is_contested_detects_dispute`
  - `fn labellers_returns_worker_dids`
- 📄 `web_civics.rs`
  - `fn derive_webizen_ipv6`
- 📄 `webizen.rs`
  - `fn fast_hash_goal`
  - `fn rule_has_variables`
  - `fn unify_field`
  - `fn resolve_term`
  - `fn join_premise`
  - `struct SlgArena`
  - `impl SlgArena`
  - `fn new`
  - `fn register_rule`
  - `fn rule_count`
  - `fn collect_active_quins`
  - `fn fire_registered_rules`
  - `fn fire_guard_rules`
  - `fn has_quin`
  - `fn check_table`
  - *(...and 54 more)*
- 📄 `webizen_bytecode.rs`
  - `enum VmError`
  - `struct GuardianshipContext`
  - `struct ExecutionStats`
  - `fn execute_program`
  - `fn execute_program_with_stats`
  - `fn execute_program_simd`
  - `fn make_quin`
  - `fn full_match`
  - `fn wildcard_predicate_matches_multiple`
  - `fn output_buffer_full_returns_error`
  - `fn empty_db_returns_zero_matches_and_zero_cycles`
  - `fn cycle_count_is_positive_for_non_empty_db`
  - `fn cycle_count_scales_with_db_size`
- 📄 `webizen_sync.rs`
  - `fn pull_foaf_graph`
- 📄 `webizen_validator.rs`
  - `struct VmRegisters`
  - `impl VmRegisters`
  - `fn bind`
  - `fn execute_query_program`
  - `enum WebRuleType`
  - `struct WebRule`
  - `impl WebRule`
  - `fn new`
  - `fn with_context`
  - `fn with_predicate`
  - `fn with_confidence`
  - `fn with_domain`
  - `enum WebVerdict`
  - `struct WebValidator`
  - `impl WebValidator`
  - *(...and 19 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
