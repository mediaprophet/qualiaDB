# Vibe / ALL_BOUND surface gap review

**Generated:** 2026-09-06 04:03 UTC  
**Repo root:** `C:\github\qualiaDB`  
**Tool:** `scripts/vibe_surface_gap_review.py`

Heuristic inventory for human triage. High gap score ≠ mandatory Vibe bind.
Some surfaces correctly stay FRB, MCP, desktop-command, or WASM-engine only.

## Summary

- Workspace members scanned (declared): **25**
- `ids::ALL_BOUND` ids: **892** across **99** families
- `vibe::catalog::ALL_INVOKE_IDS`: **892** across **99** families
- Ids in catalog but not ALL_BOUND: **0**
- Ids in ALL_BOUND but not catalog: **0**
- Priority `.rs` modules with gap score ≥ 12: **93**

## Economics / finance libraries

- Bound `Econ.*` methods: **106**
  - sample: `Econ.abatement_net_benefit`, `Econ.agent_based_aggregate_wealth`, `Econ.aggregate_paper_fills`, `Econ.aggregate_wealth`, `Econ.atkinson`, `Econ.autocorrelation`, `Econ.bellman_update`, `Econ.bertrand_duopoly`, `Econ.bertrand_with_demand`, `Econ.binomial_option`, `Econ.black_scholes`, `Econ.block_bootstrap`
  - … +94 more
- Bound `FinancialModeling.*` methods: **3**
  - `FinancialModeling.black_scholes`, `FinancialModeling.gbm_var`, `FinancialModeling.portfolio_risk`
- Bound `Finance.*` methods: **3**
  - `Finance.convert_currency`, `Finance.ledger_balance`, `Finance.multisig_check`

### Computational economics submodules (surface scan)

| Path | pub fns | Suggested families | Bound match |
|---|---:|---|---|
| `crates/qualia-core-db/src/specialized_libs/computational_economics/mod.rs` | 0 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| `crates/qualia-core-db/src/poet_host/invoke/econ/computational_economics.rs` | 80 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Inference, MachineLearning, NLP, SymbolicAlgebra | Econ, Finance, FinancialModeling, Inference, MachineLearning, NLP, SymbolicAlgebra |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/assets.rs` | 37 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/compliance.rs` | 33 | Physics, ODE | Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/execution.rs` | 31 | Physics, ODE | Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/portfolio.rs` | 27 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/risk.rs` | 27 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/reporting.rs` | 20 | Physics, ODE | Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/rebalancing.rs` | 14 | Physics, ODE | Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/settlement.rs` | 17 | Physics, ODE | Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/performance.rs` | 21 | Physics, ODE | Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/pricing.rs` | 16 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/trading.rs` | 17 | Physics, ODE | Physics |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/fixed_income.rs` | 14 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/forensic_economics.rs` | 12 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, DeonticLogic, EpistemicLogic, ParaconsistentLogic, TemporalAndDescriptionLogic, SymbolicAndDefeasibleLogic, N3Logic, SHACL, GraphDatabase, GraphAuthoring, Audio, Animation | Animation, Audio, DeonticLogic, Econ, EpistemicLogic, Finance, FinancialModeling, GraphAuthoring, GraphDatabase, N3Logic, ParaconsistentLogic, SHACL, SymbolicAndDefeasibleLogic, TemporalAndDescriptionLogic |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/time_series.rs` | 17 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/poet/src/browser/cooperative_economics/model.rs` | 6 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE, Agency, Governance, Consent | Agency, Econ, Finance, FinancialModeling, Physics |
| `crates/qualia-client-core/src/wellfair/api/welfare_work.rs` | 10 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Agency, Governance, Consent | Agency, Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/poet_host/invoke/econ/finance_ext.rs` | 3 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/accounting.rs` | 5 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/agent_based.rs` | 6 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/asset_pricing.rs` | 7 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, SymbolicAlgebra | Econ, Finance, FinancialModeling, SymbolicAlgebra |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/behavioral.rs` | 6 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/capabilities.rs` | 1 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/categorical.rs` | 0 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/derivatives.rs` | 6 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/dynamic_programming.rs` | 5 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Inference, MachineLearning, NLP | Econ, Finance, FinancialModeling, Inference, MachineLearning, NLP |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/econometrics.rs` | 5 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/environmental_resource.rs` | 7 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/error.rs` | 6 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/game_theory.rs` | 9 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/input_output.rs` | 7 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/labor_household.rs` | 4 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/macro_models.rs` | 7 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/market_data.rs` | 5 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/market_design.rs` | 13 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/markov.rs` | 7 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/mechanism.rs` | 5 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/network_economics.rs` | 4 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/ontology_bridge.rs` | 4 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/paper_trading.rs` | 4 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/portfolio.rs` | 7 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/public_finance.rs` | 5 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/risk.rs` | 6 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/spatial_economics.rs` | 7 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/welfare.rs` | 12 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/computational_economics/yield_curve.rs` | 5 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance | Econ, Finance, FinancialModeling |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/library.rs` | 10 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/mod.rs` | 0 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| `crates/qualia-core-db/src/specialized_libs/financial_modeling/results.rs` | 1 | Physics, ODE | Physics |
| `crates/webizen-desktop/src/commands/wellfair/finance.rs` | 2 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Governance, DeonticLogic, Consent, Agency | Agency, DeonticLogic, Econ, Finance, FinancialModeling |
| `crates/webizen-desktop/src/commands/wellfair/welfare_support.rs` | 4 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Governance, DeonticLogic, Consent, Agency | Agency, DeonticLogic, Econ, Finance, FinancialModeling |
| `crates/webizen-studio/src/components/wellfair/host_client/finance.rs` | 8 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Governance, DeonticLogic, Consent, Agency | Agency, DeonticLogic, Econ, Finance, FinancialModeling |

> **Status:** `Econ.*` is already a large ALL_BOUND family (106 ids), wired via `poet_host/invoke/econ/computational_economics.rs`. Primary remaining gap is **Poet / Tool Chest consumption** (few Live scopes cite `Econ.*`) and deepening `FinancialModeling.*` / cooperative-economics UI — not inventing a new Economics family.

## Chat / chat-graph / relay

### Known chat-related modules

| Path | exists | pub fns (scan) |
|---|---|---:|
| `crates/qualia-client-core/src/chat_graph.rs` | yes | 6 |
| `crates/qualia-client-core/src/chat_relay.rs` | yes | 11 |
| `crates/qualia-client-core/src/chat_session.rs` | yes | 25 |
| `crates/qualia-client-core/src/chat_inference.rs` | yes | 10 |
| `crates/qualia-client-core/src/chat_agents.rs` | yes | 26 |
| `crates/qualia-client-core/src/chat_mesh_service.rs` | yes | 12 |
| `crates/qualia-client-core/src/social_connect.rs` | yes | 6 |

- Related bound ids (Chat/Pulse agent/Inference): **13**
  - `Inference.constrained_decode`, `Inference.detect_ungrounded`, `Inference.embed`, `Inference.grounding`, `Inference.load_model`, `Inference.run_classifier`, `Inference.run_reranker`, `Inference.run_transformer`, `Inference.unload_model`, `Inference.vector_search`, `Inference.verify_turn`, `Pulse.publish_agent_message`, `Pulse.publish_presence`

> **Gap signal:** no `Chat.*` / `ChatGraph.*` Family.method in ALL_BOUND. Chat-graph appears to be **client-core / desktop API** (`get_chat_graph`, relay, sessions) rather than a Vibe `capability.invoke` family. Decide whether Vibe should gain `ChatGraph.*` or remain FRB/desktop-only.

### Other chat-ish paths in scan

- `crates/qualia-client-core/src/api/chat.rs` (17 pub fns)
- `crates/qualia-client-core/src/chat_files.rs` (15 pub fns)
- `crates/qualia-client-core/src/chat_mesh.rs` (9 pub fns)
- `crates/qualia-client-core/src/chat_ontology.rs` (11 pub fns)
- `crates/qualia-client-core/src/chat_retrieval.rs` (1 pub fns)
- `crates/qualia-client-core/src/solid_chat.rs` (5 pub fns)
- `crates/qualia-client-core/src/wellfair/sync_relay_server.rs` (5 pub fns)

## Multi-seam exposure matrix (mechanical)

Seams scanned from source (not rustdoc unless `--rustdoc` supplied):

| Seam | Count | Notes |
|---|---:|---|
| Vibe `ALL_BOUND` | 892 | `capability.invoke` native/WASM host |
| MCP stable tools | 62 | `mcp_server::stable_mcp_tools` |
| WASM engine exports | 27 | `coverage::WASM_ENGINE` |
| Desktop command names | 542 | `webizen-desktop` generate_handler list |
| Poet Live / capability_scope | 39 | Tool Chest / registration strings |

### MCP tools

- `algebra_matrix_analyze`
- `algebra_solve_polynomial`
- `audio_features`
- `bioinformatics_align`
- `cas`
- `chemical_analysis`
- `chemical_descriptors`
- `clinical_risk`
- `computational_geometry`
- `computer_vision`
- `deontic_govern`
- `describe_qapp_surface_schema`
- `engineering_analysis_op`
- `evaluate_logic_rules`
- `evaluate_modality`
- `financial_model`
- `geometric_algebra_op`
- `geometry_manifests`
- `get_did_info`
- `get_graph_stats`
- `get_pending_tasks`
- `get_qapp_manifest`
- `get_system_status`
- `get_wallet_status`
- `graph_resolve`
- `ingest_ontology`
- `inject_test_quin`
- `inspect_qapp_readiness`
- `jural_correlate`
- `list_capabilities`
- `list_models`
- `list_ontologies`
- `list_qapp_updates`
- `list_qapps`
- `llm_chat`
- `llm_infer`
- `matrix_operation`
- `mcp_cooperate`
- `medical_score`
- `ml_inference`
- `ode_solve`
- `parse_csv`
- `parse_json`
- `parse_rdf`
- `qpu_dft`
- `qpu_optimize`
- `qpu_status`
- `query_graph`
- `query_sparql`
- `run_docs_tests`
- `serialize_csv`
- `serialize_json`
- `serialize_rdf`
- `shacl_credential_gate`
- `shacl_degrade_violations`
- `shacl_route`
- `statistical_analysis`
- `symbolic_logic_infer`
- `validate_enumerated_identity`
- `validate_shacl`
- `values_check`
- `values_evaluate`

### WASM engine exports

- `cas_differentiate_wasm` (math)
- `cas_simplify_wasm` (math)
- `cas_expand_wasm` (math)
- `cas_evaluate_wasm` (math)
- `cas_factor_wasm` (math)
- `cas_solve_quadratic_wasm` (math)
- `la_matmul_wasm` (math)
- `la_transpose_wasm` (math)
- `la_determinant_wasm` (math)
- `la_solve_wasm` (math)
- `la_eigen_symmetric_wasm` (math)
- `la_eigenvalues_wasm` (math)
- `la_svd_wasm` (math)
- `la_polynomial_roots_wasm` (math)
- `stats_describe_wasm` (stats)
- `stats_correlation_wasm` (stats)
- `stats_linear_regression_wasm` (stats)
- `num_bessel_j_wasm` (math)
- `num_gcd_lcm_wasm` (math)
- `num_is_prime_wasm` (math)
- `crypto_sha256` (crypto)
- `crypto_sha512` (crypto)
- `crypto_blake3` (crypto)
- `units_convert` (math)
- `xform_dft` (math)
- `graph_shortest_path` (graph)
- `graph_spreading_activation` (graph)

### Poet Live caps not in ALL_BOUND

None — Poet Live strings ⊆ ALL_BOUND.

### Selected ALL_BOUND families missing from Poet Live/scope (sample)

First 40 finance/inference/pulse/stats/clinical ids not referenced in poet capability strings:

- `ClinicalRisk.comorbidity`
- `ClinicalRisk.contraindication`
- `ClinicalRisk.drug_interaction`
- `ClinicalRisk.fhir_observation`
- `Finance.convert_currency`
- `Finance.ledger_balance`
- `Finance.multisig_check`
- `FinancialModeling.black_scholes`
- `FinancialModeling.gbm_var`
- `FinancialModeling.portfolio_risk`
- `Inference.constrained_decode`
- `Inference.embed`
- `Inference.load_model`
- `Inference.run_classifier`
- `Inference.run_reranker`
- `Inference.run_transformer`
- `Inference.unload_model`
- `Inference.vector_search`
- `Pulse.close_channel`
- `Pulse.open_channel`
- `Pulse.publish_agent_message`
- `Pulse.publish_graph_mutation`
- `Pulse.publish_notification`
- `Pulse.publish_sync`
- `Pulse.publish_telemetry`
- `Pulse.set_transport`
- `Statistics.adf_proxy`
- `Statistics.argmax`
- `Statistics.autocorrelation`
- `Statistics.beta_pdf`
- `Statistics.betai`
- `Statistics.binomial_cdf`
- `Statistics.binomial_pmf`
- `Statistics.bootstrap_means`
- `Statistics.chi_square_gof`
- `Statistics.chi_square_independence`
- `Statistics.chi_squared_cdf`
- `Statistics.chi_squared_pdf`
- `Statistics.chi_squared_quantile`
- `Statistics.chi_squared_upper_p`
- … +77 more

### Desktop chat / agent / MCP command names

- `add_chat_participant`
- `agent_qa_snapshot`
- `agent_qa_test_active_model`
- `agent_remote_connection_test`
- `agent_roster_add_remote`
- `agent_roster_get`
- `agent_roster_list`
- `agent_roster_remove`
- `agent_roster_upsert`
- `agent_runtime_status`
- `agent_set_allowed_mcp_tools`
- `append_chat_message`
- `browser_agent_ask`
- `browser_agent_tls_status`
- `cancel_chat_inference`
- `create_chat_session`
- `create_group_chat_session`
- `create_project_group_chat`
- `ensure_chat_session`
- `get_chat_graph`
- `get_chat_participants`
- `ingest_chat_cml`
- `list_chat_contacts`
- `list_chat_sessions`
- `load_chat_session`
- `mcp_call_tool_gated`
- `mcp_ensure_safe_tool_allowlist`
- `mcp_list_local_tools`
- `remove_chat_participant`
- `run_agent_inference`
- `schedule_agent_job`
- `stream_chat_inference`
- `toggle_nym_relay`
- `wellfair_sync_with_relay`

### rustdoc

Not run. Optional:

```bash
# requires nightly rustdoc JSON for qualia-core-db
python scripts/vibe_surface_gap_review.py --rustdoc path/to/qualia_core_db.json
```

Or: `node scripts/vibe-coverage-from-rustdoc.mjs <qualia_core_db.json>`

## Catalog vs ALL_BOUND drift

No drift between catalog and ALL_BOUND string sets.

## Top gap candidates (by score)

Suggested families come from path/name keywords. Bound match lists families that already exist in ALL_BOUND.

| Score | Crate | Path | #fns | Suggested | Bound families |
|---:|---|---|---:|---|---|
| 372 | `webizen-studio` | `crates/webizen-studio/src/components/mod.rs` | 0 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Chat, ChatGraph, Pulse, GraphDatabase, Inference, Statistics, ComputerVision, Crypto, FiduciaryCrypto, Security, ClinicalRisk, Medical, Health, Physics, ODE, Chemistry, HealthAsset, HealthKnowledge, MachineLearning, NLP, DeonticLogic, EpistemicLogic, ParaconsistentLogic, TemporalAndDescriptionLogic, SymbolicAndDefeasibleLogic, Render, Animation, N3Logic, SHACL, GraphAuthoring, Governance, Consent, Agency | Agency, Animation, Chemistry, ClinicalRisk, ComputerVision, DeonticLogic, Econ, EpistemicLogic, Finance, FinancialModeling, GraphAuthoring, GraphDatabase, Inference, MachineLearning, Medical, N3Logic, NLP, ParaconsistentLogic, Physics, Pulse, Render, SHACL, Statistics, SymbolicAndDefeasibleLogic, TemporalAndDescriptionLogic |
| 109 | `qualia-client-core` | `crates/qualia-client-core/src/lib.rs` | 0 | Chat, ChatGraph, Pulse, GraphDatabase, Inference, ComputerVision, ComputationalGeometry, Render, ClinicalRisk, Medical, Health, Physics, ODE, MachineLearning, NLP, Audio, Animation, Governance, DeonticLogic, Consent, Agency, Econ, Finance | Agency, Animation, Audio, ClinicalRisk, ComputationalGeometry, ComputerVision, DeonticLogic, Econ, Finance, GraphDatabase, Inference, MachineLearning, Medical, NLP, Physics, Pulse, Render |
| 96 | `poet` | `crates/poet/src/browser/mod.rs` | 27 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, ComputerVision, ComputationalGeometry, Render, Physics, ODE, Inference, MachineLearning, NLP, DeonticLogic, EpistemicLogic, ParaconsistentLogic, TemporalAndDescriptionLogic, SymbolicAndDefeasibleLogic, Pulse, Animation, Governance, Consent, Agency, GraphDatabase | Agency, Animation, ComputationalGeometry, ComputerVision, DeonticLogic, Econ, EpistemicLogic, Finance, FinancialModeling, GraphDatabase, Inference, MachineLearning, NLP, ParaconsistentLogic, Physics, Pulse, Render, SymbolicAndDefeasibleLogic, TemporalAndDescriptionLogic |
| 62 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/computational_geometry/mod.rs` | 0 | Statistics, ComputationalGeometry, Render, Animation | Animation, ComputationalGeometry, Render, Statistics |
| 52 | `poet` | `crates/poet/src/browser/project_views/mod.rs` | 0 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE, Governance, DeonticLogic, Consent | DeonticLogic, Econ, Finance, FinancialModeling, Physics |
| 49 | `qualia-client-core` | `crates/qualia-client-core/src/wellfair/mod.rs` | 0 | ClinicalRisk, Medical, Health, Inference, MachineLearning, NLP, Render, Animation, Governance, DeonticLogic, Consent, Agency, Econ, Finance, GraphDatabase | Agency, Animation, ClinicalRisk, DeonticLogic, Econ, Finance, GraphDatabase, Inference, MachineLearning, Medical, NLP, Render |
| 46 | `qualia-core-db` | `crates/qualia-core-db/src/modalities/mod.rs` | 0 | ComputationalGeometry, Render, DeonticLogic, EpistemicLogic, ParaconsistentLogic, TemporalAndDescriptionLogic, SymbolicAndDefeasibleLogic, Governance, Consent | ComputationalGeometry, DeonticLogic, EpistemicLogic, ParaconsistentLogic, Render, SymbolicAndDefeasibleLogic, TemporalAndDescriptionLogic |
| 44 | `webizen-studio` | `crates/webizen-studio/src/components/wellfair/mod.rs` | 0 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, ClinicalRisk, Medical, Health, Governance, DeonticLogic, Consent, Agency, GraphDatabase | Agency, ClinicalRisk, DeonticLogic, Econ, Finance, FinancialModeling, GraphDatabase, Medical |
| 41 | `poet` | `crates/poet/src/browser/health_views/mod.rs` | 0 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, ClinicalRisk, Medical, Health, Physics, ODE, Governance, DeonticLogic, Consent | ClinicalRisk, DeonticLogic, Econ, Finance, FinancialModeling, Medical, Physics |
| 40 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/chemistry_modeling/simulation.rs` | 80 | ComputerVision, ComputationalGeometry, Render, Physics, ODE, Chemistry, HealthAsset, HealthKnowledge | Chemistry, ComputationalGeometry, ComputerVision, Physics, Render |
| 40 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/medical_computing/privacy.rs` | 80 | ClinicalRisk, Medical, Health, Physics, ODE, Governance, DeonticLogic, Consent | ClinicalRisk, DeonticLogic, Medical, Physics |
| 35 | `webizen-desktop` | `crates/webizen-desktop/src/commands/mod.rs` | 13 | ComputerVision, ComputationalGeometry, Render, Physics, ODE, Animation, Audio, Governance, DeonticLogic, Consent, Agency, Econ, Finance, GraphDatabase | Agency, Animation, Audio, ComputationalGeometry, ComputerVision, DeonticLogic, Econ, Finance, GraphDatabase, Physics, Render |
| 33 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/statistical_computing/analytics.rs` | 62 | Statistics, Physics, ODE | Physics, Statistics |
| 32 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/cryptographic_library/security.rs` | 57 | Crypto, FiduciaryCrypto, Security, Physics, ODE | Physics |
| 31 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/computational_economics/mod.rs` | 0 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| 31 | `qualia-vision` | `crates/qualia-vision/src/lib.rs` | 0 | ComputerVision, Render, Animation, Audio | Animation, Audio, ComputerVision, Render |
| 29 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/engineering_analysis/structural.rs` | 48 | ComputationalGeometry, Render, Physics, ODE, LinearAlgebra, SymbolicAlgebra, Animation | Animation, ComputationalGeometry, LinearAlgebra, Physics, Render, SymbolicAlgebra |
| 28 | `webizen-desktop` | `crates/webizen-desktop/src/commands/wellfair/mod.rs` | 3 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Crypto, FiduciaryCrypto, Security, ClinicalRisk, Medical, Health, Governance, DeonticLogic, Consent, Agency | Agency, ClinicalRisk, DeonticLogic, Econ, Finance, FinancialModeling, Medical |
| 27 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/chemistry_modeling/kinetics.rs` | 45 | Physics, ODE, Chemistry, HealthAsset, HealthKnowledge | Chemistry, Physics |
| 27 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/mod.rs` | 0 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Statistics, ComputerVision, ComputationalGeometry, Render, Crypto, FiduciaryCrypto, Security, ClinicalRisk, Medical, Health, Physics, ODE, Chemistry, HealthAsset, HealthKnowledge, Inference, MachineLearning, NLP, LinearAlgebra, SymbolicAlgebra | Chemistry, ClinicalRisk, ComputationalGeometry, ComputerVision, Econ, Finance, FinancialModeling, Inference, LinearAlgebra, MachineLearning, Medical, NLP, Physics, Render, Statistics, SymbolicAlgebra |
| 27 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/physics_simulation/data_storage.rs` | 59 | Physics, ODE | Physics |
| 26 | `poet` | `crates/poet/src/browser/studio_views/mod.rs` | 0 | ComputationalGeometry, Render, Audio, Animation | Animation, Audio, ComputationalGeometry, Render |
| 26 | `qualia-audio` | `crates/qualia-audio/src/lib.rs` | 0 | Physics, ODE, Audio, Animation | Animation, Audio, Physics |
| 26 | `qualia-core-db` | `crates/qualia-core-db/src/poet_host/invoke/econ/computational_economics.rs` | 80 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Inference, MachineLearning, NLP, SymbolicAlgebra | Econ, Finance, FinancialModeling, Inference, MachineLearning, NLP, SymbolicAlgebra |
| 25 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/medical_computing/drug_discovery.rs` | 35 | ClinicalRisk, Medical, Health, Physics, ODE | ClinicalRisk, Medical, Physics |
| 24 | `qualia-core-db` | `crates/qualia-core-db/src/poet_host/invoke/research/mod.rs` | 73 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, ComputerVision, Physics, ODE, Inference, MachineLearning, NLP, DeonticLogic, EpistemicLogic, ParaconsistentLogic, TemporalAndDescriptionLogic, SymbolicAndDefeasibleLogic, Research, Investigation | ComputerVision, DeonticLogic, Econ, EpistemicLogic, Finance, FinancialModeling, Inference, MachineLearning, NLP, ParaconsistentLogic, Physics, Research, SymbolicAndDefeasibleLogic, TemporalAndDescriptionLogic |
| 24 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/machine_learning/inference.rs` | 74 | Inference, MachineLearning, NLP | Inference, MachineLearning, NLP |
| 24 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/medical_computing/compliance.rs` | 34 | ClinicalRisk, Medical, Health | ClinicalRisk, Medical |
| 24 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/medical_computing/diagnosis.rs` | 38 | ClinicalRisk, Medical, Health, Physics, ODE | ClinicalRisk, Medical, Physics |
| 23 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/engineering_analysis/mechanical.rs` | 31 | Statistics, Physics, ODE, Audio, Animation | Animation, Audio, Physics, Statistics |
| 23 | `webizen-desktop` | `crates/webizen-desktop/src/commands/mail.rs` | 70 | Chat, ChatGraph, Pulse, GraphDatabase, Inference, ComputationalGeometry, Render, Governance, DeonticLogic, Consent, Agency, Finance | Agency, ComputationalGeometry, DeonticLogic, Finance, GraphDatabase, Inference, Pulse, Render |
| 22 | `qualia-audio` | `crates/qualia-audio/src/features/mod.rs` | 0 | Audio, Animation | Animation, Audio |
| 22 | `qualia-client-core` | `crates/qualia-client-core/src/api/mod.rs` | 0 | Physics, ODE, Agency, GraphDatabase, Finance | Agency, Finance, GraphDatabase, Physics |
| 22 | `qualia-core-db` | `crates/qualia-core-db/src/poet_host/invoke/hypermedia/mod.rs` | 66 | ComputerVision, ComputationalGeometry, Render, Physics, ODE, Animation | Animation, ComputationalGeometry, ComputerVision, Physics, Render |
| 22 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/financial_modeling/assets.rs` | 37 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| 21 | `qualia-core-db` | `crates/qualia-core-db/src/poet_host/invoke/solvers.rs` | 63 | Chemistry, HealthAsset, HealthKnowledge | Chemistry |
| 21 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/financial_modeling/compliance.rs` | 33 | Physics, ODE | Physics |
| 21 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/financial_modeling/execution.rs` | 31 | Physics, ODE | Physics |
| 20 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/cryptographic_library/proofs.rs` | 29 | Crypto, FiduciaryCrypto, Security | QuantumAndCryptographic |
| 20 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/engineering_analysis/reliability.rs` | 25 | Statistics, Physics, ODE, Research, Investigation | Physics, Research, Statistics |
| 20 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/machine_learning/training.rs` | 61 | Inference, MachineLearning, NLP | Inference, MachineLearning, NLP |
| 20 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/physics_simulation/distributed.rs` | 38 | ComputationalGeometry, Render, Physics, ODE | ComputationalGeometry, Physics, Render |
| 18 | `poet` | `crates/poet/src/browser/native_daemon.rs` | 28 | ComputerVision, Physics, ODE, Inference, MachineLearning, NLP, Pulse, Render, Animation | Animation, ComputerVision, Inference, MachineLearning, NLP, Physics, Pulse, Render |
| 18 | `poet` | `crates/poet/src/tool_chest/core/mod.rs` | 0 | — | **none** |
| 18 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/physics_simulation/data_migration.rs` | 43 | Physics, ODE | Physics |
| 18 | `webizen-studio` | `crates/webizen-studio/src/components/wellfair/host_client/mod.rs` | 46 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Governance, DeonticLogic, Consent, Agency | Agency, DeonticLogic, Econ, Finance, FinancialModeling |
| 17 | `qualia-client-core` | `crates/qualia-client-core/src/accountability_store.rs` | 32 | — | **none** |
| 17 | `qualia-core-db` | `crates/qualia-core-db/src/poet_host/mod.rs` | 33 | ComputerVision, Pulse, N3Logic, SHACL, GraphDatabase, GraphAuthoring | ComputerVision, GraphAuthoring, GraphDatabase, N3Logic, Pulse, SHACL |
| 17 | `qualia-core-db` | `crates/qualia-core-db/src/q42/mod.rs` | 0 | Physics, ODE, Chemistry, HealthAsset, HealthKnowledge, Inference, MachineLearning, NLP, Render, Animation | Animation, Chemistry, Inference, MachineLearning, NLP, Physics, Render |
| 17 | `qualia-core-db` | `crates/qualia-core-db/src/q42/q42_volume.rs` | 46 | Physics, ODE, N3Logic, SHACL, GraphDatabase, GraphAuthoring | GraphAuthoring, GraphDatabase, N3Logic, Physics, SHACL |
| 16 | `poet` | `crates/poet/src/browser/dataset_views/mod.rs` | 0 | — | **none** |
| 16 | `poet` | `crates/poet/src/tool_chest/manifolds/mod.rs` | 1 | ComputationalGeometry, Render, ClinicalRisk, Medical, Health, Research, Investigation | ClinicalRisk, ComputationalGeometry, Medical, Render, Research |
| 16 | `qualia-client-core` | `crates/qualia-client-core/src/api/system.rs` | 47 | ComputerVision, Physics, ODE | ComputerVision, Physics |
| 16 | `qualia-client-core` | `crates/qualia-client-core/src/qpu_oracle.rs` | 25 | — | **none** |
| 16 | `qualia-core-db` | `crates/qualia-core-db/src/modalities/logic/mod.rs` | 0 | ComputationalGeometry, Render, DeonticLogic, EpistemicLogic, ParaconsistentLogic, TemporalAndDescriptionLogic, SymbolicAndDefeasibleLogic, N3Logic, SHACL, GraphDatabase, GraphAuthoring | ComputationalGeometry, DeonticLogic, EpistemicLogic, GraphAuthoring, GraphDatabase, N3Logic, ParaconsistentLogic, Render, SHACL, SymbolicAndDefeasibleLogic, TemporalAndDescriptionLogic |
| 16 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/computer_vision/cv/mod.rs` | 0 | ComputerVision, Physics, ODE | ComputerVision, Physics |
| 16 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/financial_modeling/portfolio.rs` | 27 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| 16 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/financial_modeling/risk.rs` | 27 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Physics, ODE | Econ, Finance, FinancialModeling, Physics |
| 16 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/medical_computing/imaging.rs` | 24 | ComputerVision, ClinicalRisk, Medical, Health | ClinicalRisk, ComputerVision, Medical |
| 16 | `webizen-desktop` | `crates/webizen-desktop/src/lib.rs` | 0 | Governance, DeonticLogic, Consent | DeonticLogic |
| 15 | `qualia-client-core` | `crates/qualia-client-core/src/api/tokens.rs` | 41 | FinancialModeling, Econ, Economics, ComputationalEconomics, Market, Risk, Finance, Chat, ChatGraph, Pulse, GraphDatabase, Inference, Physics, ODE, MachineLearning, NLP, Agency | Agency, Econ, Finance, FinancialModeling, GraphDatabase, Inference, MachineLearning, NLP, Physics, Pulse |
| 15 | `qualia-core-db` | `crates/qualia-core-db/src/poet_host/invoke/stats/distributions.rs` | 31 | — | **none** |
| 15 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/financial_modeling/reporting.rs` | 20 | Physics, ODE | Physics |
| 14 | `qualia-core-db` | `crates/qualia-core-db/src/q42/volume/range_volume.rs` | 20 | — | **none** |
| 14 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/engineering_analysis/dynamics.rs` | 26 | Physics, ODE | Physics |
| 14 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/linear_algebra.rs` | 24 | LinearAlgebra, SymbolicAlgebra | LinearAlgebra, SymbolicAlgebra |
| 14 | `webizen-studio` | `crates/webizen-studio/src/render/mod.rs` | 1 | ComputerVision, ComputationalGeometry, Render, Physics, ODE, Animation, Governance, DeonticLogic, Consent | Animation, ComputationalGeometry, ComputerVision, DeonticLogic, Physics, Render |
| 13 | `qualia-audio` | `crates/qualia-audio/src/features/spectral/mod.rs` | 0 | Audio, Animation | Animation, Audio |
| 13 | `qualia-client-core` | `crates/qualia-client-core/src/wellfair/api/host_core.rs` | 40 | ClinicalRisk, Medical, Health, N3Logic, SHACL, GraphDatabase, GraphAuthoring, Governance, DeonticLogic, Consent, Agency, Econ, Finance | Agency, ClinicalRisk, DeonticLogic, Econ, Finance, GraphAuthoring, GraphDatabase, Medical, N3Logic, SHACL |
| 13 | `qualia-core-db` | `crates/qualia-core-db/src/governance/mod.rs` | 0 | Physics, ODE, Governance, DeonticLogic, Consent | DeonticLogic, Physics |
| 13 | `qualia-core-db` | `crates/qualia-core-db/src/modalities/blackboard.rs` | 35 | N3Logic, SHACL, GraphDatabase, GraphAuthoring | GraphAuthoring, GraphDatabase, N3Logic, SHACL |
| 13 | `qualia-core-db` | `crates/qualia-core-db/src/modalities/logic/specialized_libs_shacl.rs` | 2 | Statistics, ComputationalGeometry, Render, Crypto, FiduciaryCrypto, Security, ClinicalRisk, Medical, Health, Physics, ODE, Inference, MachineLearning, NLP, SymbolicAlgebra, N3Logic, SHACL, GraphDatabase, GraphAuthoring | ClinicalRisk, ComputationalGeometry, GraphAuthoring, GraphDatabase, Inference, MachineLearning, Medical, N3Logic, NLP, Physics, Render, SHACL, Statistics, SymbolicAlgebra |
| 13 | `qualia-core-db` | `crates/qualia-core-db/src/poet_host/invoke/asset_store.rs` | 36 | DeonticLogic, EpistemicLogic, ParaconsistentLogic, TemporalAndDescriptionLogic, SymbolicAndDefeasibleLogic, N3Logic, SHACL, GraphDatabase, GraphAuthoring | DeonticLogic, EpistemicLogic, GraphAuthoring, GraphDatabase, N3Logic, ParaconsistentLogic, SHACL, SymbolicAndDefeasibleLogic, TemporalAndDescriptionLogic |
| 13 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/computational_geometry/authoring.rs` | 31 | ComputationalGeometry, Render, Physics, ODE, Governance, DeonticLogic, Consent | ComputationalGeometry, DeonticLogic, Physics, Render |
| 13 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/linear_algebra/computation.rs` | 12 | LinearAlgebra, SymbolicAlgebra | LinearAlgebra, SymbolicAlgebra |
| 13 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/machine_learning/types.rs` | 0 | Statistics, Physics, ODE, Inference, MachineLearning, NLP | Inference, MachineLearning, NLP, Physics, Statistics |
| 13 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/medical_computing/types.rs` | 1 | ComputerVision, ClinicalRisk, Medical, Health | ClinicalRisk, ComputerVision, Medical |
| 13 | `qualia-core-db` | `crates/qualia-core-db/src/specialized_libs/physics_simulation/solvers_eigen_opt.rs` | 29 | Physics, ODE | Physics |
| 13 | `webizen-desktop` | `crates/webizen-desktop/src/commands/social.rs` | 39 | Chat, ChatGraph, Pulse, GraphDatabase, Inference, ClinicalRisk, Medical, Health, Physics, ODE, MachineLearning, NLP, Governance, DeonticLogic, Consent | ClinicalRisk, DeonticLogic, GraphDatabase, Inference, MachineLearning, Medical, NLP, Physics, Pulse |
| 12 | `qualia-client-core` | `crates/qualia-client-core/src/chat_session.rs` | 25 | Chat, ChatGraph, Pulse, GraphDatabase, Inference | GraphDatabase, Inference, Pulse |

## ALL_BOUND families (counts)

| Family | Methods |
|---|---:|
| `Econ` | 106 |
| `Statistics` | 97 |
| `MachineLearning` | 81 |
| `Research` | 73 |
| `Render` | 53 |
| `Asset` | 21 |
| `Cosmic` | 20 |
| `ComputationalGeometry` | 19 |
| `NumberTheory` | 18 |
| `Physics` | 18 |
| `Scene` | 17 |
| `Audio` | 16 |
| `HID` | 16 |
| `Image` | 15 |
| `Dmx` | 14 |
| `LinearAlgebra` | 14 |
| `SpecialFunctions` | 12 |
| `Inference` | 11 |
| `ComputerVision` | 10 |
| `Pulse` | 10 |
| `Video` | 10 |
| `EngineeringAnalysis` | 8 |
| `GeometricAlgebra` | 8 |
| `NLP` | 8 |
| `Orchestration` | 8 |
| `ThreeD` | 8 |
| `Animation` | 7 |
| `Calculus` | 7 |
| `Chemistry` | 7 |
| `ClinicalRisk` | 7 |
| `FuzzyQuery` | 7 |
| `IntegralTransforms` | 7 |
| `VectorCalculus` | 7 |
| `World` | 7 |
| `Interpolation` | 6 |
| `SymbolicAlgebra` | 6 |
| `Agent` | 5 |
| `Capability` | 5 |
| `GraphDatabase` | 5 |
| `Poet` | 5 |
| `Spectral` | 5 |
| `sampler` | 5 |
| `HbbTV` | 4 |
| `Ode` | 4 |
| `Social` | 4 |
| `TemporalAndDescriptionLogic` | 4 |
| `CapabilityDiscovery` | 3 |
| `Finance` | 3 |
| `FinancialModeling` | 3 |
| `GraphMatch` | 3 |
| `Manifold` | 3 |
| `Medical` | 3 |
| `Optimization` | 3 |
| `Portal` | 3 |
| `QuantumAndCryptographic` | 3 |
| `agent` | 3 |
| `Avatar` | 2 |
| `Bioinformatics` | 2 |
| `CausalFuzzyAndControl` | 2 |
| `Corpus` | 2 |
| `Forensic` | 2 |
| `GraphReasoning` | 2 |
| `Interactive` | 2 |
| `MedicalComputing` | 2 |
| `Net` | 2 |
| `OrganicChemistry` | 2 |
| `SHACL` | 2 |
| `Sentinel` | 2 |
| `Sheet` | 2 |
| `biosignal` | 2 |
| `AdvancedLogic` | 1 |
| `Agency` | 1 |
| `CalculusWorkbench` | 1 |
| `ContractsIdentityAndConsensus` | 1 |
| `DeonticLogic` | 1 |
| `Document` | 1 |
| `EpistemicLogic` | 1 |
| `FormalLogic` | 1 |
| `GovernanceLogic` | 1 |
| `GraphAuthoring` | 1 |
| `Identity` | 1 |
| `InfraExtLogic` | 1 |
| `InfraLogic` | 1 |
| `LegalLogic` | 1 |
| `MedicalImaging` | 1 |
| `N3Logic` | 1 |
| `NumericalCalculus` | 1 |
| `OntologyAlignment` | 1 |
| `ParaconsistentLogic` | 1 |
| `PhysicalUnits` | 1 |
| `PhysicsAndODE` | 1 |
| `PhysicsWorkbench` | 1 |
| `Privacy` | 1 |
| `SecondScreen` | 1 |
| `SpatialLogic` | 1 |
| `SpecialFunctionsAndTransforms` | 1 |
| `SymbolicAndDefeasibleLogic` | 1 |
| `hash` | 1 |
| `nlp` | 1 |

## Specialized libs root modules

From `specialized_libs/mod.rs`:

- `category_theory` → suggested ['—']; bound: **no ALL_BOUND family match**
- `chemistry_modeling` → suggested ['Physics', 'ODE', 'Chemistry', 'HealthAsset', 'HealthKnowledge']; bound: `Physics`, `Chemistry`
- `computational_economics` → suggested ['FinancialModeling', 'Econ', 'Economics', 'ComputationalEconomics', 'Market', 'Risk', 'Finance']; bound: `FinancialModeling`, `Econ`, `Finance`
- `computational_geometry` → suggested ['ComputationalGeometry', 'Render']; bound: `ComputationalGeometry`, `Render`
- `computer_vision` → suggested ['ComputerVision']; bound: `ComputerVision`
- `constructibility` → suggested ['—']; bound: **no ALL_BOUND family match**
- `cryptographic_library` → suggested ['Crypto', 'FiduciaryCrypto', 'Security']; bound: **no ALL_BOUND family match**
- `engineering_analysis` → suggested ['—']; bound: **no ALL_BOUND family match**
- `financial_modeling` → suggested ['Physics', 'ODE']; bound: `Physics`
- `linear_algebra` → suggested ['LinearAlgebra', 'SymbolicAlgebra']; bound: `LinearAlgebra`, `SymbolicAlgebra`
- `machine_learning` → suggested ['Inference', 'MachineLearning', 'NLP']; bound: `Inference`, `MachineLearning`, `NLP`
- `medical_computing` → suggested ['ClinicalRisk', 'Medical', 'Health']; bound: `ClinicalRisk`, `Medical`
- `multivar_calculus` → suggested ['—']; bound: **no ALL_BOUND family match**
- `physics_simulation` → suggested ['Physics', 'ODE']; bound: `Physics`
- `polynomial_algebra` → suggested ['SymbolicAlgebra']; bound: `SymbolicAlgebra`
- `qpu_bridge` → suggested ['—']; bound: **no ALL_BOUND family match**
- `quantum_biology` → suggested ['—']; bound: **no ALL_BOUND family match**
- `statistical_computing` → suggested ['Statistics']; bound: `Statistics`
- `symbolic_algebra` → suggested ['SymbolicAlgebra']; bound: `SymbolicAlgebra`
- `symbolic_assumptions` → suggested ['SymbolicAlgebra']; bound: `SymbolicAlgebra`
- `symbolic_integration` → suggested ['SymbolicAlgebra']; bound: `SymbolicAlgebra`
- `symbolic_limits` → suggested ['SymbolicAlgebra']; bound: `SymbolicAlgebra`
- `symbolic_ode` → suggested ['Physics', 'ODE', 'SymbolicAlgebra']; bound: `Physics`, `SymbolicAlgebra`
- `symbolic_series` → suggested ['SymbolicAlgebra']; bound: `SymbolicAlgebra`
- `symbolic_solve` → suggested ['SymbolicAlgebra']; bound: `SymbolicAlgebra`
- `symbolic_trig` → suggested ['SymbolicAlgebra']; bound: `SymbolicAlgebra`
- `shared` → suggested ['—']; bound: **no ALL_BOUND family match**

## How to use this report

1. Review **Economics** and **Chat** sections first — known principal concerns.
2. For each high-score row with **none** bound families, decide: Vibe Family, MCP-only, FRB/desktop, or leave cold.
3. Do **not** invent Host widen / dotted `qualia.*` IDs; new binds must be approved `Family.method` in `ids.rs` + `ALL_INVOKE_IDS` together.
4. Re-run after large library landings:

```bash
python scripts/vibe_surface_gap_review.py
```

