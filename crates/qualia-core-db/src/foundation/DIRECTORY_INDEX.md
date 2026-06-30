---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# foundation Index

## Functionality Overview
Comprehensive index of functionality for `foundation`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `crdt.rs`
  - `struct DelegatedAccess`
  - `struct CrdtResolver`
  - `impl CrdtResolver`
  - `fn resolve_lww`
  - `fn verify_delegation`
  - `fn qualia_crdt_resolution`
  - `fn test_crdt_bifurcation`
  - `struct SuspendedTransaction`
  - `struct SuspendedTransactionQueue`
  - `impl SuspendedTransactionQueue`
  - `fn push`
  - `fn apply_consensus_token`
- 📄 `frame_layout.rs`
  - `fn pack_float_object`
  - `fn unpack_float_object`
  - `fn truth_degree`
  - `fn with_truth_degree`
  - `fn sealed`
  - `fn parity_valid`
  - `fn predicate_regions_do_not_collide`
  - `fn object_datatype_tags_are_distinct`
  - `fn low32_payload_is_disjoint_from_every_high_overlay`
  - `fn modality_flag_bits_are_pairwise_distinct`
  - `fn predicate_and_degree_round_trip`
  - `fn matches_deontic_packing`
  - `fn parity_ignores_computational_metadata`
- 📄 `fuzz_testing.rs`
  - `fn fuzz_query_compiler_no_panics`
  - `fn fuzz_raw_quin_memory_mapping`
  - `fn qualia_validate_volatile_scrubbing`
  - `fn test_daemon_swarm_fiduciary_boundary_stress`
- 📄 `mod.rs`
- 📄 `telemetry.rs`
  - `fn reset_telemetry`
  - `fn get_telemetry_snapshot`
  - `fn export_prometheus_metrics`
  - `fn log_federated_telemetry`
  - `fn test_telemetry_atomics`
- 📄 `topology_draft.rs`
  - `struct TopologyDraftMapper`
  - `fn new`
  - `fn concept_to_token_id`
  - `fn fill_draft_batch`
  - `fn mapper_is_stable_for_same_hash`
  - `fn fill_draft_batch_respects_gamma`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
