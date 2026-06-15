# Known Issues — QualiaDB

Tracking file for known-failing tests and other defects that are **not yet fixed** but are
understood. Keep this current; link to it from PRs/plans rather than copying lists around.

---

## Pre-existing failing lib tests (`qualia-core-db`)

**Status:** open, pre-existing, **unrelated to the 2026-06-15 crypto work** (see
[`CRYPTO_IMPLEMENTATION_PLAN.md`](CRYPTO_IMPLEMENTATION_PLAN.md)).

**Reproduce:** `cargo test -p qualia-core-db --lib`
**Result:** ~26–28 failing tests (the count is **nondeterministic** — varies 26/27/28 across
runs). The **host build and the `wasm32-unknown-unknown` build both succeed** — these are
*test* failures, not compile failures. All crypto and `sparql_did` tests pass.

### Confirmed root causes / triage

- **`webizen_identifiers::webizen_tests::*`** (4 tests) — assert
  `WebizenIdentity::is_webizen_id(identity.webizen_id)` and hit `"ID not in Webizen range"`.
  A randomly-generated Webizen ID sometimes falls outside the expected range → the ID
  generation or the range check in `webizen_identifiers.rs` is the bug. **Nondeterministic.**
- **`*::test_node_discovery`, `*::test_device_discovery`** (`acoustic_ble_mesh`,
  `ambient_orchestration`) — discovery tests driven by randomness/timing. **Nondeterministic.**
- **Deterministic WIP failures** (fail every run) in math/logic modules:
  `geometric_algebra` (SIMD kernel + integration), `graph_theory` (centrality / community /
  motif), `dialectical` (do-calculus / counterfactual / confounding), `argumentation`
  (grounded / skeptical / status), `control_feedback`, `modalities::logic::shacl`
  (`test_validation_report`), `modalities::logic::core` (`test_webizen_float_logic`),
  `spatio_temporal` (RCC8 / region-quin), `lora::adapter_manager`
  (`test_checksum_corruption_detected`), `neuro_symbolic_sieve`
  (`sieve_builds_masks_from_mmap_lex`), `orchestrator` (`test_e2e_llm_to_wal_pipeline`).

### Full failing list (snapshot 2026-06-15, 27 failed)

```
acoustic_ble_mesh::tests::test_node_discovery
ambient_orchestration::tests::test_device_discovery
geometric_algebra::integration_tests::test_comprehensive_operations
geometric_algebra::simd_kernel::tests::test_outer_product
geometric_algebra::simd_kernel::tests::test_rotor_creation
geometric_algebra::simd_kernel::tests::test_translator
lora::adapter_manager::tests::test_checksum_corruption_detected
modalities::argumentation::tests::{test_argument_status, test_grounded_extension, test_skeptical_resolution}
modalities::control_feedback::tests::{test_control_state_update, test_power_system_controller}
modalities::dialectical::tests::{test_adjust_for_confounding, test_counterfactual_query, test_do_intervention}
modalities::graph_theory::tests::{test_centrality_calculation, test_community_detection, test_motif_detection}
modalities::logic::core::tests::test_webizen_float_logic
modalities::logic::shacl::tests::test_validation_report
modalities::spatio_temporal::tests::{test_rcc8_basic_relations, test_region_quin_conversion}
neuro_symbolic_sieve::tests::sieve_builds_masks_from_mmap_lex
orchestrator::tests::test_e2e_llm_to_wal_pipeline
webizen_identifiers::webizen_tests::{test_registry_registration, test_registry_webid_lookup, test_verify_signature_stub, test_webizen_identity_creation}
```

### Suggested first fix

Start with the **`webizen_identifiers` ID-range** cluster — it's a single root cause
(random ID vs. expected range) and likely a one-line fix in the ID generator or
`is_webizen_id` bounds, which would also stabilise the nondeterministic suite count.

> Why "not caused by crypto": every failing test is in a subsystem the crypto pass never
> touched; the only edited file appearing here is `webizen_identifiers.rs`, where the change
> was a doc-comment only; and static code changes cannot produce a nondeterministic failure
> count. Build (host + WASM) is clean.
