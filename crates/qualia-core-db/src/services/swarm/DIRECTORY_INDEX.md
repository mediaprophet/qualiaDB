---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# swarm Index

## Functionality Overview
Comprehensive index of functionality for `swarm`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `dispatch.rs`
  - `struct DispatchOutcome`
  - `fn run_job`
  - `fn dense_job`
  - `struct LyingExecutor`
  - `impl JobExecutor`
  - `fn execute`
  - `fn paid_mode`
  - `fn honest_paid_job_verifies_and_pays`
  - `fn lying_paid_job_is_rejected_and_refunded_never_paid`
  - `fn personal_job_runs_with_no_settlement`
  - `fn collaborative_job_runs_with_no_settlement`
- 📄 `executor.rs`
  - `trait JobExecutor`
  - `fn execute`
  - `struct LocalKernelExecutor`
  - `impl JobExecutor`
  - `fn local_executor_computes_a_real_product`
  - `fn local_executor_trains_a_real_artifact`
  - `fn malformed_job_fails_closed`
- 📄 `isolate.rs`
  - `fn field_to_f64`
  - `fn fold_result`
  - `fn isolate_b_compute`
  - `fn quin`
  - `fn output_is_real_not_a_constant`
  - `fn computation_is_deterministic`
  - `fn parity_is_valid`
  - `fn metadata_constraint_changes_the_result`
- 📄 `job.rs`
  - `enum JobKind`
  - `enum JobMode`
  - `enum JobInput`
  - `impl JobInput`
  - `fn kind`
  - `fn is_well_formed`
  - `enum JobResult`
  - `impl JobResult`
  - `struct JobSpec`
  - `impl JobSpec`
  - `fn new`
  - `fn content_id`
  - `fn dense`
  - `fn content_id_is_deterministic_and_input_sensitive`
  - `fn well_formedness_catches_bad_dims`
  - *(...and 1 more)*
- 📄 `mod.rs`
  - `enum SwarmError`
  - `impl core`
  - `fn fmt`
  - `impl std`
- 📄 `settlement.rs`
  - `enum EscrowState`
  - `enum SettlementOutcome`
  - `struct Escrow`
  - `impl Escrow`
  - `fn offer`
  - `fn hold`
  - `fn settle`
  - `fn price_paid_job`
  - `fn energy_viable`
  - `fn held_escrow`
  - `fn verified_releases_a_payment_instruction`
  - `fn rejected_refunds_and_pays_no_provider`
  - `fn cannot_settle_before_holding`
  - `fn cannot_double_settle`
  - `fn price_is_roi_capped`
  - *(...and 1 more)*
- 📄 `verify.rs`
  - `enum VerificationVerdict`
  - `impl VerificationVerdict`
  - `fn is_verified`
  - `struct VerifyPolicy`
  - `impl Default`
  - `fn default`
  - `fn verify`
  - `fn verify_product`
  - `fn verify_artifact`
  - `fn freivalds_accepts_a_correct_product`
  - `fn freivalds_rejects_a_wrong_product`
  - `fn freivalds_rejects_a_subtly_wrong_product`
  - `fn artifact_verification_accepts_a_learned_table_and_rejects_garbage`
  - `fn mismatched_kinds_rejected`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
