# Vibe incorporation backlog (exhaustive)

**Generated:** 2026-09-06 04:37 UTC  
**Methodology:** `docs/work-in-progress/VIBE_INCORPORATION_METHODOLOGY_2026-09-06.md`  
**Tool:** `scripts/vibe_incorporation_backlog.py`

Pipeline: **crate `pub fn` → Host `Family.method` → Poet Live**.
Exact method-name join only counts as Host-bound. Fuzzy hints are not coverage.

## Summary

| Metric | Count |
|---|---:|
| Crates scanned | 25 |
| `ALL_BOUND` | 897 |
| `ALL_INVOKE_IDS` | 897 |
| Distinct Host method suffixes | 861 |
| Public fns scanned | 19362 |
| Public fns with exact Host method twin | 2892 |
| **Q1 Host-missing** (listed / total) | 400 / 12315 |
| **Q2 Poet-consume** (Host id not in Poet Live) | 841 |
| Poet Live ids | 56 |
| Poet Live stale (not in ALL_BOUND) | 0 |
| Catalog drift (bound-only / invoke-only) | 0 / 0 |

## Q0 — Catalog drift

None — `ALL_BOUND` and `ALL_INVOKE_IDS` string sets match.

## Q2 — Host bound but not in Poet Live (by family)

These are **vibescript-ready**: Poet dual-path next (no new Host id).

| Family | Uncited Host ids |
|---|---:|
| `Econ` | 96 |
| `Statistics` | 91 |
| `MachineLearning` | 81 |
| `Research` | 73 |
| `Render` | 51 |
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
| `Video` | 10 |
| `EngineeringAnalysis` | 8 |
| `GeometricAlgebra` | 8 |
| `Inference` | 8 |
| `Orchestration` | 8 |
| `ThreeD` | 8 |
| `Calculus` | 7 |
| `Chemistry` | 7 |
| `FuzzyQuery` | 7 |
| `IntegralTransforms` | 7 |
| `NLP` | 7 |
| `Pulse` | 7 |
| `VectorCalculus` | 7 |
| `World` | 7 |
| `Interpolation` | 6 |
| `Agent` | 5 |
| `Capability` | 5 |
| `Spectral` | 5 |
| `SymbolicAlgebra` | 5 |
| `sampler` | 5 |
| `Animation` | 4 |
| `ClinicalRisk` | 4 |
| `HbbTV` | 4 |
| `Ode` | 4 |
| `Poet` | 4 |
| `Social` | 4 |
| `CapabilityDiscovery` | 3 |
| `ChatGraph` | 3 |
| `ComputerVision` | 3 |
| `Finance` | 3 |
| `FinancialModeling` | 3 |
| `GraphMatch` | 3 |
| `Manifold` | 3 |
| `Medical` | 3 |
| `Optimization` | 3 |
| `Portal` | 3 |
| `QuantumAndCryptographic` | 3 |
| `TemporalAndDescriptionLogic` | 3 |
| `agent` | 3 |
| `Avatar` | 2 |
| `Bioinformatics` | 2 |
| `Corpus` | 2 |
| `Forensic` | 2 |
| `GraphDatabase` | 2 |
| `GraphReasoning` | 2 |
| `Interactive` | 2 |
| `MedicalComputing` | 2 |
| `Net` | 2 |
| `OrganicChemistry` | 2 |
| `Sheet` | 2 |
| `biosignal` | 2 |
| `AdvancedLogic` | 1 |
| `Agency` | 1 |
| `CalculusWorkbench` | 1 |
| `CausalFuzzyAndControl` | 1 |
| `ContractsIdentityAndConsensus` | 1 |
| `CooperativeWork` | 1 |
| `FormalLogic` | 1 |
| `GovernanceLogic` | 1 |
| `InfraExtLogic` | 1 |
| `InfraLogic` | 1 |
| `LegalLogic` | 1 |
| `MedicalImaging` | 1 |
| `NumericalCalculus` | 1 |
| `OntologyAlignment` | 1 |
| `PhysicalUnits` | 1 |
| `PhysicsAndODE` | 1 |
| `PhysicsWorkbench` | 1 |
| `Privacy` | 1 |
| `SHACL` | 1 |
| `SecondScreen` | 1 |
| `Sentinel` | 1 |
| `SpatialLogic` | 1 |
| `SpecialFunctionsAndTransforms` | 1 |
| `SymbolicAndDefeasibleLogic` | 1 |
| `hash` | 1 |

### Sample uncited ids (first 80)

- `AdvancedLogic.compute`
- `Agency.evaluate`
- `Agent.evaluate`
- `Agent.execute`
- `Agent.plan`
- `Agent.trace`
- `Agent.verify`
- `Animation.list_presets`
- `Animation.sclerp_step`
- `Animation.spring_step`
- `Animation.squad_step`
- `Asset.add_temporal`
- `Asset.add_topic`
- `Asset.compile`
- `Asset.count`
- `Asset.create`
- `Asset.list`
- `Asset.persist`
- `Asset.persist_add_temporal`
- `Asset.persist_add_topic`
- `Asset.persist_compile`
- `Asset.persist_create`
- `Asset.persist_query_aspects`
- `Asset.persist_set_spatial`
- `Asset.persist_temporal_span`
- `Asset.query_aspects`
- `Asset.resolve`
- `Asset.resolve_by_spatial`
- `Asset.resolve_by_temporal`
- `Asset.resolve_by_topic`
- `Asset.set_spatial`
- `Asset.temporal_span`
- `Audio.compressor`
- `Audio.delay`
- `Audio.envelope`
- `Audio.eq`
- `Audio.filter`
- `Audio.lfo`
- `Audio.loudness_meter`
- `Audio.midi_note`
- `Audio.oscillator`
- `Audio.phase_meter`
- `Audio.quantize`
- `Audio.reverb`
- `Audio.spectrum`
- `Audio.transport`
- `Audio.transpose`
- `Audio.waveform_meter`
- `Avatar.move`
- `Avatar.set_appearance`
- `Bioinformatics.align`
- `Bioinformatics.compute`
- `Calculus.adaptive_derivative`
- `Calculus.adaptive_simpson`
- `Calculus.discrete_maximum_principle_holds`
- `Calculus.newton_solve`
- `Calculus.numerical_hessian`
- `Calculus.numerical_jacobian`
- `Calculus.solve_poisson_dirichlet`
- `CalculusWorkbench.compute`
- `Capability.audit`
- `Capability.declare`
- `Capability.grant`
- `Capability.revoke`
- `Capability.test_gating`
- `CapabilityDiscovery.catalog`
- `CapabilityDiscovery.coverage`
- `CapabilityDiscovery.list`
- `CausalFuzzyAndControl.t_norm`
- `ChatGraph.link_reply`
- `ChatGraph.session_summary`
- `ChatGraph.validate_fragment`
- `Chemistry.atomic_number`
- `Chemistry.element_symbol`
- `Chemistry.lda_correlation_vwn`
- `Chemistry.lda_exchange`
- `Chemistry.parse_bse_json`
- `Chemistry.standard_atomic_weight`
- `Chemistry.sto3g_h2`
- `ClinicalRisk.comorbidity`
- … +761 more (see JSON)

## Q1 — Host-missing public functions (priority order)

Proposed `Family.method` is a **suggestion** for vibescript bind packets.

| Pri | Crate | Module | fn | Proposed Host id | Path |
|---:|---|---|---|---|---|
| 100 | `qualia-core-db` | `` | `calculate_parity` | `CoreDb.calculate_parity` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `evaluate_permissive_runtime_gate` | `CoreDb.evaluate_permissive_runtime_gate` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `extract_clean_metadata_value` | `CoreDb.extract_clean_metadata_value` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `extract_lamport_clock` | `CoreDb.extract_lamport_clock` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `from_bytes` | `CoreDb.from_bytes` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `get_quin_type` | `CoreDb.get_quin_type` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `get_sensitivity_byte` | `CoreDb.get_sensitivity_byte` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `get_sensitivity_tier` | `CoreDb.get_sensitivity_tier` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `get_subject_literal_id` | `CoreDb.get_subject_literal_id` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `identify_routing_lane` | `CoreDb.identify_routing_lane` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `is_subject_nested` | `CoreDb.is_subject_nested` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `lookup_embedded_triple` | `CoreDb.lookup_embedded_triple` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `lookup_hash` | `CoreDb.lookup_hash` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `new_conduct_violation` | `CoreDb.new_conduct_violation` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `open` | `CoreDb.open` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `recalculate_parity` | `CoreDb.recalculate_parity` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `set_lamport_clock` | `CoreDb.set_lamport_clock` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `set_quin_type` | `CoreDb.set_quin_type` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `set_sensitivity_byte` | `CoreDb.set_sensitivity_byte` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `set_sensitivity_tier` | `CoreDb.set_sensitivity_tier` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `verify_ecc_parity` | `CoreDb.verify_ecc_parity` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `` | `view` | `CoreDb.view` | `crates/qualia-core-db/src/lib.rs` |
| 100 | `qualia-core-db` | `agent_runtime::corpus` | `all_tags` | `CoreDb.all_tags` | `crates/qualia-core-db/src/agent_runtime/corpus.rs` |
| 100 | `qualia-core-db` | `agent_runtime::corpus` | `cases_with_tag` | `CoreDb.cases_with_tag` | `crates/qualia-core-db/src/agent_runtime/corpus.rs` |
| 100 | `qualia-core-db` | `agent_runtime::corpus` | `is_empty` | `CoreDb.is_empty` | `crates/qualia-core-db/src/agent_runtime/corpus.rs` |
| 100 | `qualia-core-db` | `agent_runtime::corpus` | `len` | `CoreDb.len` | `crates/qualia-core-db/src/agent_runtime/corpus.rs` |
| 100 | `qualia-core-db` | `agent_runtime::corpus` | `load_corpus_from_file` | `CoreDb.load_corpus_from_file` | `crates/qualia-core-db/src/agent_runtime/corpus.rs` |
| 100 | `qualia-core-db` | `agent_runtime::corpus` | `parse_corpus` | `CoreDb.parse_corpus` | `crates/qualia-core-db/src/agent_runtime/corpus.rs` |
| 100 | `qualia-core-db` | `agent_runtime::evaluator` | `compute_metrics` | `CoreDb.compute_metrics` | `crates/qualia-core-db/src/agent_runtime/evaluator.rs` |
| 100 | `qualia-core-db` | `agent_runtime::evaluator` | `eval_case` | `CoreDb.eval_case` | `crates/qualia-core-db/src/agent_runtime/evaluator.rs` |
| 100 | `qualia-core-db` | `agent_runtime::evaluator` | `evaluate_corpus` | `CoreDb.evaluate_corpus` | `crates/qualia-core-db/src/agent_runtime/evaluator.rs` |
| 100 | `qualia-core-db` | `agent_runtime::evaluator` | `score_case` | `CoreDb.score_case` | `crates/qualia-core-db/src/agent_runtime/evaluator.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `active_agents` | `CoreDb.active_agents` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `add_capability` | `CoreDb.add_capability` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `all_capabilities` | `CoreDb.all_capabilities` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `as_str` | `CoreDb.as_str` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `create_blackboard` | `CoreDb.create_blackboard` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `create_phase_leaser` | `CoreDb.create_phase_leaser` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `create_session` | `CoreDb.create_session` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `execute_session` | `CoreDb.execute_session` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `find_agent_for_capability` | `CoreDb.find_agent_for_capability` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `get` | `CoreDb.get` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `has_capability` | `CoreDb.has_capability` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `plan_session` | `CoreDb.plan_session` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `register` | `CoreDb.register` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `session_to_pipeline` | `CoreDb.session_to_pipeline` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::orchestration` | `unregister` | `CoreDb.unregister` | `crates/qualia-core-db/src/agent_runtime/orchestration.rs` |
| 100 | `qualia-core-db` | `agent_runtime::planner` | `classify` | `CoreDb.classify` | `crates/qualia-core-db/src/agent_runtime/planner.rs` |
| 100 | `qualia-core-db` | `agent_runtime::planner` | `plan_task` | `CoreDb.plan_task` | `crates/qualia-core-db/src/agent_runtime/planner.rs` |
| 100 | `qualia-core-db` | `agent_runtime::planner` | `plan_to_record_map` | `CoreDb.plan_to_record_map` | `crates/qualia-core-db/src/agent_runtime/planner.rs` |
| 100 | `qualia-core-db` | `archive` | `decompress_frame` | `CoreDb.decompress_frame` | `crates/qualia-core-db/src/archive.rs` |
| 100 | `qualia-core-db` | `archive` | `open` | `CoreDb.open` | `crates/qualia-core-db/src/archive.rs` |
| 100 | `qualia-core-db` | `archive` | `physical_offset` | `CoreDb.physical_offset` | `crates/qualia-core-db/src/archive.rs` |
| 100 | `qualia-core-db` | `archive` | `preamble` | `CoreDb.preamble` | `crates/qualia-core-db/src/archive.rs` |
| 100 | `qualia-core-db` | `archive` | `read_dictionary` | `CoreDb.read_dictionary` | `crates/qualia-core-db/src/archive.rs` |
| 100 | `qualia-core-db` | `archive` | `read_jump_table` | `CoreDb.read_jump_table` | `crates/qualia-core-db/src/archive.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `acoustic_effective_mode` | `CoreDb.acoustic_effective_mode` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `acoustic_enabled_for_mode` | `CoreDb.acoustic_enabled_for_mode` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `acoustic_params_from_tensor` | `CoreDb.acoustic_params_from_tensor` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `apply_binaural_to_uniform` | `CoreDb.apply_binaural_to_uniform` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `drain_sonic_tokens` | `CoreDb.drain_sonic_tokens` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `from_tensor` | `CoreDb.from_tensor` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `is_empty` | `CoreDb.is_empty` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `len` | `CoreDb.len` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `pop_sonic_token` | `CoreDb.pop_sonic_token` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `push_sonic_token` | `CoreDb.push_sonic_token` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `sonic_token_ring` | `CoreDb.sonic_token_ring` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `sonify_tensor_node` | `CoreDb.sonify_tensor_node` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `to_phenomenal_uniform` | `CoreDb.to_phenomenal_uniform` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `to_uniform` | `CoreDb.to_uniform` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `try_pop` | `CoreDb.try_pop` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_plane` | `try_push` | `CoreDb.try_push` | `crates/qualia-core-db/src/audio/acoustic_plane.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_sab` | `init_acoustic_sab` | `CoreDb.init_acoustic_sab` | `crates/qualia-core-db/src/audio/acoustic_sab.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_sab` | `push_token_to_sab` | `CoreDb.push_token_to_sab` | `crates/qualia-core-db/src/audio/acoustic_sab.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_sab` | `read_uniform_from_sab` | `CoreDb.read_uniform_from_sab` | `crates/qualia-core-db/src/audio/acoustic_sab.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_sab` | `write_uniform_to_sab` | `CoreDb.write_uniform_to_sab` | `crates/qualia-core-db/src/audio/acoustic_sab.rs` |
| 100 | `qualia-core-db` | `audio::acoustic_sab` | `write_uniform_to_sab_with_mirror` | `CoreDb.write_uniform_to_sab_with_mirror` | `crates/qualia-core-db/src/audio/acoustic_sab.rs` |
| 100 | `qualia-core-db` | `audio::audio_sidecar_link` | `bake_audio_sidecar_into` | `CoreDb.bake_audio_sidecar_into` | `crates/qualia-core-db/src/audio/audio_sidecar_link.rs` |
| 100 | `qualia-core-db` | `audio::audio_sidecar_link` | `compile_spectral_sheet_quin` | `CoreDb.compile_spectral_sheet_quin` | `crates/qualia-core-db/src/audio/audio_sidecar_link.rs` |
| 100 | `qualia-core-db` | `audio::audio_sidecar_link` | `enrich_preview_from_sidecar` | `CoreDb.enrich_preview_from_sidecar` | `crates/qualia-core-db/src/audio/audio_sidecar_link.rs` |
| 100 | `qualia-core-db` | `audio::audio_sidecar_link` | `format_sidecar_relpath` | `CoreDb.format_sidecar_relpath` | `crates/qualia-core-db/src/audio/audio_sidecar_link.rs` |
| 100 | `qualia-core-db` | `audio::audio_sidecar_link` | `link_tensor_audio_sidecar` | `CoreDb.link_tensor_audio_sidecar` | `crates/qualia-core-db/src/audio/audio_sidecar_link.rs` |
| 100 | `qualia-core-db` | `audio::audio_sidecar_link` | `sidecar_content_hash` | `CoreDb.sidecar_content_hash` | `crates/qualia-core-db/src/audio/audio_sidecar_link.rs` |
| 100 | `qualia-core-db` | `audio::audio_sidecar_link` | `write_sidecar_file` | `CoreDb.write_sidecar_file` | `crates/qualia-core-db/src/audio/audio_sidecar_link.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `bake_spectral_v2_from_samples` | `CoreDb.bake_spectral_v2_from_samples` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `copy_sidecar_frame_to_preview_bins` | `CoreDb.copy_sidecar_frame_to_preview_bins` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `from_tensor_preview` | `CoreDb.from_tensor_preview` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `is_valid` | `CoreDb.is_valid` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `is_valid` | `CoreDb.is_valid` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `parse_sidecar_header` | `CoreDb.parse_sidecar_header` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `parse_v2_subheader` | `CoreDb.parse_v2_subheader` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `payload_bytes` | `CoreDb.payload_bytes` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `preview_bins_from_tensor` | `CoreDb.preview_bins_from_tensor` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `sidecar_frame_view` | `CoreDb.sidecar_frame_view` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `sidecar_mel_frame_view` | `CoreDb.sidecar_mel_frame_view` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `sidecar_mfcc_frame_view` | `CoreDb.sidecar_mfcc_frame_view` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::audio_spectral_sheet` | `v2_sidecar_size` | `CoreDb.v2_sidecar_size` | `crates/qualia-core-db/src/audio/audio_spectral_sheet.rs` |
| 100 | `qualia-core-db` | `audio::cqt_bake` | `bake_cqt_sidecar_from_preview` | `CoreDb.bake_cqt_sidecar_from_preview` | `crates/qualia-core-db/src/audio/cqt_bake.rs` |
| 100 | `qualia-core-db` | `audio::cqt_bake` | `bake_cqt_sidecar_from_samples` | `CoreDb.bake_cqt_sidecar_from_samples` | `crates/qualia-core-db/src/audio/cqt_bake.rs` |
| 100 | `qualia-core-db` | `audio::cqt_bake` | `forward_cqt` | `CoreDb.forward_cqt` | `crates/qualia-core-db/src/audio/cqt_bake.rs` |
| 100 | `qualia-core-db` | `audio::cqt_bake` | `preview_to_cqt_frame` | `CoreDb.preview_to_cqt_frame` | `crates/qualia-core-db/src/audio/cqt_bake.rs` |
| 100 | `qualia-core-db` | `audio::dsp::effects` | `reset` | `CoreDb.reset` | `crates/qualia-core-db/src/audio/dsp/effects.rs` |
| 100 | `qualia-core-db` | `audio::dsp::effects` | `reset` | `CoreDb.reset` | `crates/qualia-core-db/src/audio/dsp/effects.rs` |
| 100 | `qualia-core-db` | `audio::dsp::effects` | `reset` | `CoreDb.reset` | `crates/qualia-core-db/src/audio/dsp/effects.rs` |
| 100 | `qualia-core-db` | `audio::dsp::effects` | `set_band_gains` | `CoreDb.set_band_gains` | `crates/qualia-core-db/src/audio/dsp/effects.rs` |
| 100 | `qualia-core-db` | `audio::dsp::effects` | `set_delay_samples` | `CoreDb.set_delay_samples` | `crates/qualia-core-db/src/audio/dsp/effects.rs` |
| 100 | `qualia-core-db` | `audio::dsp::effects` | `tick` | `CoreDb.tick` | `crates/qualia-core-db/src/audio/dsp/effects.rs` |
| 100 | `qualia-core-db` | `audio::dsp::effects` | `tick` | `CoreDb.tick` | `crates/qualia-core-db/src/audio/dsp/effects.rs` |
| 100 | `qualia-core-db` | `audio::dsp::effects` | `tick` | `CoreDb.tick` | `crates/qualia-core-db/src/audio/dsp/effects.rs` |
| 100 | `qualia-core-db` | `audio::dsp::effects` | `tick` | `CoreDb.tick` | `crates/qualia-core-db/src/audio/dsp/effects.rs` |
| 100 | `qualia-core-db` | `audio::dsp::envelope` | `note_off` | `CoreDb.note_off` | `crates/qualia-core-db/src/audio/dsp/envelope.rs` |
| 100 | `qualia-core-db` | `audio::dsp::envelope` | `note_on` | `CoreDb.note_on` | `crates/qualia-core-db/src/audio/dsp/envelope.rs` |
| 100 | `qualia-core-db` | `audio::dsp::envelope` | `reset` | `CoreDb.reset` | `crates/qualia-core-db/src/audio/dsp/envelope.rs` |
| 100 | `qualia-core-db` | `audio::dsp::envelope` | `stage` | `CoreDb.stage` | `crates/qualia-core-db/src/audio/dsp/envelope.rs` |
| 100 | `qualia-core-db` | `audio::dsp::envelope` | `tick` | `CoreDb.tick` | `crates/qualia-core-db/src/audio/dsp/envelope.rs` |
| 100 | `qualia-core-db` | `audio::dsp::envelope` | `value` | `CoreDb.value` | `crates/qualia-core-db/src/audio/dsp/envelope.rs` |
| 100 | `qualia-core-db` | `audio::dsp::filter` | `recalculate` | `CoreDb.recalculate` | `crates/qualia-core-db/src/audio/dsp/filter.rs` |
| 100 | `qualia-core-db` | `audio::dsp::filter` | `reset` | `CoreDb.reset` | `crates/qualia-core-db/src/audio/dsp/filter.rs` |
| 100 | `qualia-core-db` | `audio::dsp::filter` | `set_cutoff` | `CoreDb.set_cutoff` | `crates/qualia-core-db/src/audio/dsp/filter.rs` |
| 100 | `qualia-core-db` | `audio::dsp::filter` | `set_q` | `CoreDb.set_q` | `crates/qualia-core-db/src/audio/dsp/filter.rs` |
| 100 | `qualia-core-db` | `audio::dsp::filter` | `tick` | `CoreDb.tick` | `crates/qualia-core-db/src/audio/dsp/filter.rs` |
| 100 | `qualia-core-db` | `audio::dsp::lfo` | `reset` | `CoreDb.reset` | `crates/qualia-core-db/src/audio/dsp/lfo.rs` |
| 100 | `qualia-core-db` | `audio::dsp::lfo` | `set_depth` | `CoreDb.set_depth` | `crates/qualia-core-db/src/audio/dsp/lfo.rs` |
| 100 | `qualia-core-db` | `audio::dsp::lfo` | `set_frequency` | `CoreDb.set_frequency` | `crates/qualia-core-db/src/audio/dsp/lfo.rs` |
| 100 | `qualia-core-db` | `audio::dsp::lfo` | `tick` | `CoreDb.tick` | `crates/qualia-core-db/src/audio/dsp/lfo.rs` |
| 100 | `qualia-core-db` | `audio::dsp::meters` | `analyse` | `CoreDb.analyse` | `crates/qualia-core-db/src/audio/dsp/meters.rs` |
| 100 | `qualia-core-db` | `audio::dsp::meters` | `analyse` | `CoreDb.analyse` | `crates/qualia-core-db/src/audio/dsp/meters.rs` |
| 100 | `qualia-core-db` | `audio::dsp::meters` | `tick` | `CoreDb.tick` | `crates/qualia-core-db/src/audio/dsp/meters.rs` |
| 100 | `qualia-core-db` | `audio::dsp::meters` | `waveform_display` | `CoreDb.waveform_display` | `crates/qualia-core-db/src/audio/dsp/meters.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `clear_loop` | `CoreDb.clear_loop` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `freq_to_midi_note` | `CoreDb.freq_to_midi_note` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `metronome_phase` | `CoreDb.metronome_phase` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `metronome_sample` | `CoreDb.metronome_sample` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `midi_note_to_freq` | `CoreDb.midi_note_to_freq` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `midi_to_note_name` | `CoreDb.midi_to_note_name` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `note_name_to_midi` | `CoreDb.note_name_to_midi` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `pause` | `CoreDb.pause` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `play` | `CoreDb.play` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `position_beats` | `CoreDb.position_beats` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `position_seconds` | `CoreDb.position_seconds` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `record` | `CoreDb.record` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `set_loop` | `CoreDb.set_loop` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `set_metronome` | `CoreDb.set_metronome` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `set_tempo` | `CoreDb.set_tempo` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `stop` | `CoreDb.stop` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::midi_transport` | `tick` | `CoreDb.tick` | `crates/qualia-core-db/src/audio/dsp/midi_transport.rs` |
| 100 | `qualia-core-db` | `audio::dsp::oscillator` | `reset` | `CoreDb.reset` | `crates/qualia-core-db/src/audio/dsp/oscillator.rs` |
| 100 | `qualia-core-db` | `audio::dsp::oscillator` | `set_frequency` | `CoreDb.set_frequency` | `crates/qualia-core-db/src/audio/dsp/oscillator.rs` |
| 100 | `qualia-core-db` | `audio::dsp::oscillator` | `set_gain` | `CoreDb.set_gain` | `crates/qualia-core-db/src/audio/dsp/oscillator.rs` |
| 100 | `qualia-core-db` | `audio::dsp::oscillator` | `tick` | `CoreDb.tick` | `crates/qualia-core-db/src/audio/dsp/oscillator.rs` |
| 100 | `qualia-core-db` | `audio::dsp_kernel` | `configure_voice_from_tensor` | `CoreDb.configure_voice_from_tensor` | `crates/qualia-core-db/src/audio/dsp_kernel.rs` |
| 100 | `qualia-core-db` | `audio::dsp_kernel` | `epistemic_fm_index` | `CoreDb.epistemic_fm_index` | `crates/qualia-core-db/src/audio/dsp_kernel.rs` |
| 100 | `qualia-core-db` | `audio::dsp_kernel` | `epistemic_temperature_from_q` | `CoreDb.epistemic_temperature_from_q` | `crates/qualia-core-db/src/audio/dsp_kernel.rs` |
| 100 | `qualia-core-db` | `audio::dsp_kernel` | `parametric_sample` | `CoreDb.parametric_sample` | `crates/qualia-core-db/src/audio/dsp_kernel.rs` |
| 100 | `qualia-core-db` | `audio::dsp_kernel` | `sigma_dominant_frequency` | `CoreDb.sigma_dominant_frequency` | `crates/qualia-core-db/src/audio/dsp_kernel.rs` |
| 100 | `qualia-core-db` | `audio::hrtf` | `binaural_analytic` | `CoreDb.binaural_analytic` | `crates/qualia-core-db/src/audio/hrtf.rs` |
| 100 | `qualia-core-db` | `audio::hrtf` | `binaural_from_position` | `CoreDb.binaural_from_position` | `crates/qualia-core-db/src/audio/hrtf.rs` |
| 100 | `qualia-core-db` | `audio::hrtf` | `binaural_kemar_lite` | `CoreDb.binaural_kemar_lite` | `crates/qualia-core-db/src/audio/hrtf.rs` |
| 100 | `qualia-core-db` | `audio::hrtf` | `binaural_render` | `CoreDb.binaural_render` | `crates/qualia-core-db/src/audio/hrtf.rs` |
| 100 | `qualia-core-db` | `audio::hrtf` | `convolve_fir` | `CoreDb.convolve_fir` | `crates/qualia-core-db/src/audio/hrtf.rs` |
| 100 | `qualia-core-db` | `audio::hrtf` | `head_relative_position` | `CoreDb.head_relative_position` | `crates/qualia-core-db/src/audio/hrtf.rs` |
| 100 | `qualia-core-db` | `audio::hrtf` | `hrtf_profile` | `CoreDb.hrtf_profile` | `crates/qualia-core-db/src/audio/hrtf.rs` |
| 100 | `qualia-core-db` | `audio::hrtf` | `room_damp_from_manifold` | `CoreDb.room_damp_from_manifold` | `crates/qualia-core-db/src/audio/hrtf.rs` |
| 100 | `qualia-core-db` | `audio::hrtf` | `set_hrtf_profile` | `CoreDb.set_hrtf_profile` | `crates/qualia-core-db/src/audio/hrtf.rs` |
| 100 | `qualia-core-db` | `audio::hrtf` | `synthesize_hrir` | `CoreDb.synthesize_hrir` | `crates/qualia-core-db/src/audio/hrtf.rs` |
| 100 | `qualia-core-db` | `audio::istft` | `inverse_stft` | `CoreDb.inverse_stft` | `crates/qualia-core-db/src/audio/istft.rs` |
| 100 | `qualia-core-db` | `audio::stft` | `bake_stft_sidecar_from_samples` | `CoreDb.bake_stft_sidecar_from_samples` | `crates/qualia-core-db/src/audio/stft.rs` |
| 100 | `qualia-core-db` | `audio::stft` | `forward_stft` | `CoreDb.forward_stft` | `crates/qualia-core-db/src/audio/stft.rs` |
| 100 | `qualia-core-db` | `audio::stft` | `stft_magnitudes` | `CoreDb.stft_magnitudes` | `crates/qualia-core-db/src/audio/stft.rs` |
| 100 | `qualia-core-db` | `audio::stft_bake` | `bake_stft_sidecar_from_preview` | `CoreDb.bake_stft_sidecar_from_preview` | `crates/qualia-core-db/src/audio/stft_bake.rs` |
| 100 | `qualia-core-db` | `audio::stft_bake` | `bake_tensor_stft_sidecar` | `CoreDb.bake_tensor_stft_sidecar` | `crates/qualia-core-db/src/audio/stft_bake.rs` |
| 100 | `qualia-core-db` | `audio::stft_bake` | `synthesize_stft_frame` | `CoreDb.synthesize_stft_frame` | `crates/qualia-core-db/src/audio/stft_bake.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `bin_to_freq_linear` | `CoreDb.bin_to_freq_linear` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `bin_to_freq_log` | `CoreDb.bin_to_freq_log` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `frame_energy` | `CoreDb.frame_energy` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `frame_to_time` | `CoreDb.frame_to_time` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `freq_gradient` | `CoreDb.freq_gradient` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `get` | `CoreDb.get` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `ridge_bin` | `CoreDb.ridge_bin` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `ridges` | `CoreDb.ridges` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `sample_bilinear` | `CoreDb.sample_bilinear` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `spectral_flux` | `CoreDb.spectral_flux` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `time_gradient` | `CoreDb.time_gradient` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `to_height_mesh` | `CoreDb.to_height_mesh` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface` | `total_energy` | `CoreDb.total_energy` | `crates/qualia-core-db/src/audio/tf_surface.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `apply_gain` | `CoreDb.apply_gain` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `bin_span` | `CoreDb.bin_span` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `copy_patch` | `CoreDb.copy_patch` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `crossfade` | `CoreDb.crossfade` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `fade_in` | `CoreDb.fade_in` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `fade_out` | `CoreDb.fade_out` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `frame_span` | `CoreDb.frame_span` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `full` | `CoreDb.full` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `is_valid` | `CoreDb.is_valid` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `pitch_shift` | `CoreDb.pitch_shift` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `spectral_gate` | `CoreDb.spectral_gate` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `audio::tf_surface_edit` | `time_stretch` | `CoreDb.time_stretch` | `crates/qualia-core-db/src/audio/tf_surface_edit.rs` |
| 100 | `qualia-core-db` | `bundle::reader` | `as_bytes` | `CoreDb.as_bytes` | `crates/qualia-core-db/src/bundle/reader.rs` |
| 100 | `qualia-core-db` | `bundle::reader` | `entries` | `CoreDb.entries` | `crates/qualia-core-db/src/bundle/reader.rs` |
| 100 | `qualia-core-db` | `bundle::reader` | `entry` | `CoreDb.entry` | `crates/qualia-core-db/src/bundle/reader.rs` |
| 100 | `qualia-core-db` | `bundle::reader` | `flags` | `CoreDb.flags` | `crates/qualia-core-db/src/bundle/reader.rs` |
| 100 | `qualia-core-db` | `bundle::reader` | `get` | `CoreDb.get` | `crates/qualia-core-db/src/bundle/reader.rs` |
| 100 | `qualia-core-db` | `bundle::reader` | `open` | `CoreDb.open` | `crates/qualia-core-db/src/bundle/reader.rs` |
| 100 | `qualia-core-db` | `bundle::reader` | `reader` | `CoreDb.reader` | `crates/qualia-core-db/src/bundle/reader.rs` |
| 100 | `qualia-core-db` | `bundle::reader` | `segment` | `CoreDb.segment` | `crates/qualia-core-db/src/bundle/reader.rs` |
| 100 | `qualia-core-db` | `bundle::reader` | `verify_entry` | `CoreDb.verify_entry` | `crates/qualia-core-db/src/bundle/reader.rs` |
| 100 | `qualia-core-db` | `bundle::writer` | `add_file` | `CoreDb.add_file` | `crates/qualia-core-db/src/bundle/writer.rs` |
| 100 | `qualia-core-db` | `bundle::writer` | `build` | `CoreDb.build` | `crates/qualia-core-db/src/bundle/writer.rs` |
| 100 | `qualia-core-db` | `bundle::writer` | `is_empty` | `CoreDb.is_empty` | `crates/qualia-core-db/src/bundle/writer.rs` |
| 100 | `qualia-core-db` | `bundle::writer` | `len` | `CoreDb.len` | `crates/qualia-core-db/src/bundle/writer.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `cha2ds2_vasc_score` | `CoreDb.cha2ds2_vasc_score` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `check_contraindications` | `CoreDb.check_contraindications` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `check_drug_interactions` | `CoreDb.check_drug_interactions` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `ckd_epi_egfr` | `CoreDb.ckd_epi_egfr` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `cockcroft_gault_crcl` | `CoreDb.cockcroft_gault_crcl` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `evaluate_gene_expression` | `CoreDb.evaluate_gene_expression` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `framingham_10yr_risk` | `CoreDb.framingham_10yr_risk` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `longitudinal_trend` | `CoreDb.longitudinal_trend` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `one_compartment_pk_model` | `CoreDb.one_compartment_pk_model` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `score2_risk` | `CoreDb.score2_risk` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `sofa_score` | `CoreDb.sofa_score` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `clinical_engine` | `validate_fhir_observation` | `CoreDb.validate_fhir_observation` | `crates/qualia-core-db/src/clinical_engine.rs` |
| 100 | `qualia-core-db` | `container_10d::conformance` | `assert_layout_invariants` | `CoreDb.assert_layout_invariants` | `crates/qualia-core-db/src/container_10d/conformance.rs` |
| 100 | `qualia-core-db` | `container_10d::crc32c` | `crc32c` | `CoreDb.crc32c` | `crates/qualia-core-db/src/container_10d/crc32c.rs` |
| 100 | `qualia-core-db` | `container_10d::crc32c` | `crc32c_update` | `CoreDb.crc32c_update` | `crates/qualia-core-db/src/container_10d/crc32c.rs` |
| 100 | `qualia-core-db` | `container_10d::field_section` | `decode_field_section` | `CoreDb.decode_field_section` | `crates/qualia-core-db/src/container_10d/field_section.rs` |
| 100 | `qualia-core-db` | `container_10d::field_section` | `encode_field_section` | `CoreDb.encode_field_section` | `crates/qualia-core-db/src/container_10d/field_section.rs` |
| 100 | `qualia-core-db` | `container_10d::field_section` | `field_section_size` | `CoreDb.field_section_size` | `crates/qualia-core-db/src/container_10d/field_section.rs` |
| 100 | `qualia-core-db` | `container_10d::header` | `encode` | `CoreDb.encode` | `crates/qualia-core-db/src/container_10d/header.rs` |
| 100 | `qualia-core-db` | `container_10d::header` | `encode_to_vec64` | `CoreDb.encode_to_vec64` | `crates/qualia-core-db/src/container_10d/header.rs` |
| 100 | `qualia-core-db` | `container_10d::header` | `proposed` | `CoreDb.proposed` | `crates/qualia-core-db/src/container_10d/header.rs` |
| 100 | `qualia-core-db` | `container_10d::integrity` | `compute_whole_file_crc32c` | `CoreDb.compute_whole_file_crc32c` | `crates/qualia-core-db/src/container_10d/integrity.rs` |
| 100 | `qualia-core-db` | `container_10d::integrity` | `seal_whole_file_crc32c` | `CoreDb.seal_whole_file_crc32c` | `crates/qualia-core-db/src/container_10d/integrity.rs` |
| 100 | `qualia-core-db` | `container_10d::integrity` | `verify_whole_file_crc32c` | `CoreDb.verify_whole_file_crc32c` | `crates/qualia-core-db/src/container_10d/integrity.rs` |
| 100 | `qualia-core-db` | `container_10d::mesh_section` | `decode_mesh_section` | `CoreDb.decode_mesh_section` | `crates/qualia-core-db/src/container_10d/mesh_section.rs` |
| 100 | `qualia-core-db` | `container_10d::mesh_section` | `encode_mesh_section` | `CoreDb.encode_mesh_section` | `crates/qualia-core-db/src/container_10d/mesh_section.rs` |
| 100 | `qualia-core-db` | `container_10d::mesh_section` | `encoded_len` | `CoreDb.encoded_len` | `crates/qualia-core-db/src/container_10d/mesh_section.rs` |
| 100 | `qualia-core-db` | `container_10d::mesh_section` | `fits_u16_indices` | `CoreDb.fits_u16_indices` | `crates/qualia-core-db/src/container_10d/mesh_section.rs` |
| 100 | `qualia-core-db` | `container_10d::mesh_section` | `parse_mesh_header` | `CoreDb.parse_mesh_header` | `crates/qualia-core-db/src/container_10d/mesh_section.rs` |
| 100 | `qualia-core-db` | `container_10d::mesh_section` | `payload_bytes` | `CoreDb.payload_bytes` | `crates/qualia-core-db/src/container_10d/mesh_section.rs` |
| 100 | `qualia-core-db` | `container_10d::mesh_section` | `raw_geometry_len` | `CoreDb.raw_geometry_len` | `crates/qualia-core-db/src/container_10d/mesh_section.rs` |
| 100 | `qualia-core-db` | `container_10d::metric_check` | `probe_folded_axes` | `CoreDb.probe_folded_axes` | `crates/qualia-core-db/src/container_10d/metric_check.rs` |
| 100 | `qualia-core-db` | `container_10d::metric_check` | `verify_descriptor_against_reality` | `CoreDb.verify_descriptor_against_reality` | `crates/qualia-core-db/src/container_10d/metric_check.rs` |
| 100 | `qualia-core-db` | `container_10d::node_section` | `parse_node_header` | `CoreDb.parse_node_header` | `crates/qualia-core-db/src/container_10d/node_section.rs` |
| 100 | `qualia-core-db` | `container_10d::node_section` | `read_node` | `CoreDb.read_node` | `crates/qualia-core-db/src/container_10d/node_section.rs` |
| 100 | `qualia-core-db` | `container_10d::node_section` | `read_node_aos` | `CoreDb.read_node_aos` | `crates/qualia-core-db/src/container_10d/node_section.rs` |
| 100 | `qualia-core-db` | `container_10d::node_section` | `read_node_soa` | `CoreDb.read_node_soa` | `crates/qualia-core-db/src/container_10d/node_section.rs` |
| 100 | `qualia-core-db` | `container_10d::node_section` | `read_node_soa_lane` | `CoreDb.read_node_soa_lane` | `crates/qualia-core-db/src/container_10d/node_section.rs` |
| 100 | `qualia-core-db` | `container_10d::node_section` | `transpose_aos_to_soa` | `CoreDb.transpose_aos_to_soa` | `crates/qualia-core-db/src/container_10d/node_section.rs` |
| 100 | `qualia-core-db` | `container_10d::node_section` | `transpose_soa_to_aos` | `CoreDb.transpose_soa_to_aos` | `crates/qualia-core-db/src/container_10d/node_section.rs` |
| 100 | `qualia-core-db` | `container_10d::node_section` | `write_node_q_at` | `CoreDb.write_node_q_at` | `crates/qualia-core-db/src/container_10d/node_section.rs` |
| 100 | `qualia-core-db` | `container_10d::node_section` | `write_node_section_aos` | `CoreDb.write_node_section_aos` | `crates/qualia-core-db/src/container_10d/node_section.rs` |
| 100 | `qualia-core-db` | `container_10d::node_section` | `write_node_section_soa` | `CoreDb.write_node_section_soa` | `crates/qualia-core-db/src/container_10d/node_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `decode_provenance_section` | `CoreDb.decode_provenance_section` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `encode_provenance_section` | `CoreDb.encode_provenance_section` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `encoded_len` | `CoreDb.encoded_len` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `licence` | `CoreDb.licence` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `semantic_metadata` | `CoreDb.semantic_metadata` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `source_bytes` | `CoreDb.source_bytes` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `source_digest` | `CoreDb.source_digest` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `source_digest` | `CoreDb.source_digest` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `source_media_type` | `CoreDb.source_media_type` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `timestamp_epoch_s` | `CoreDb.timestamp_epoch_s` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `validate_provenance` | `CoreDb.validate_provenance` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `vc` | `CoreDb.vc` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `version_hash` | `CoreDb.version_hash` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `with_metadata` | `CoreDb.with_metadata` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::provenance_section` | `with_vc` | `CoreDb.with_vc` | `crates/qualia-core-db/src/container_10d/provenance_section.rs` |
| 100 | `qualia-core-db` | `container_10d::section` | `encode_container` | `CoreDb.encode_container` | `crates/qualia-core-db/src/container_10d/section.rs` |
| 100 | `qualia-core-db` | `container_10d::section` | `parse_section_table` | `CoreDb.parse_section_table` | `crates/qualia-core-db/src/container_10d/section.rs` |
| 100 | `qualia-core-db` | `container_10d::section` | `tier` | `CoreDb.tier` | `crates/qualia-core-db/src/container_10d/section.rs` |
| 100 | `qualia-core-db` | `container_10d::section` | `typ` | `CoreDb.typ` | `crates/qualia-core-db/src/container_10d/section.rs` |
| 100 | `qualia-core-db` | `container_10d::spatial_index_section` | `decode_spatial_index_section` | `CoreDb.decode_spatial_index_section` | `crates/qualia-core-db/src/container_10d/spatial_index_section.rs` |
| 100 | `qualia-core-db` | `container_10d::spatial_index_section` | `encode_spatial_index_section` | `CoreDb.encode_spatial_index_section` | `crates/qualia-core-db/src/container_10d/spatial_index_section.rs` |
| 100 | `qualia-core-db` | `container_10d::spatial_index_section` | `encoded_len` | `CoreDb.encoded_len` | `crates/qualia-core-db/src/container_10d/spatial_index_section.rs` |
| 100 | `qualia-core-db` | `container_10d::topology_section` | `decode_topology_section` | `CoreDb.decode_topology_section` | `crates/qualia-core-db/src/container_10d/topology_section.rs` |
| 100 | `qualia-core-db` | `container_10d::topology_section` | `encode_topology_section` | `CoreDb.encode_topology_section` | `crates/qualia-core-db/src/container_10d/topology_section.rs` |
| 100 | `qualia-core-db` | `container_10d::topology_section` | `encoded_len` | `CoreDb.encoded_len` | `crates/qualia-core-db/src/container_10d/topology_section.rs` |
| 100 | `qualia-core-db` | `crypto::deontic_circuit` | `generate_deontic_crs` | `CoreDb.generate_deontic_crs` | `crates/qualia-core-db/src/crypto/deontic_circuit.rs` |
| 100 | `qualia-core-db` | `crypto::deontic_circuit` | `verify_access` | `CoreDb.verify_access` | `crates/qualia-core-db/src/crypto/deontic_circuit.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `check_compliance` | `CoreDb.check_compliance` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `check_quantum_readiness` | `CoreDb.check_quantum_readiness` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `check_quantum_readiness` | `CoreDb.check_quantum_readiness` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `clear_audit_log` | `CoreDb.clear_audit_log` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `context_manager` | `CoreDb.context_manager` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `create_context` | `CoreDb.create_context` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `export_secret_key` | `CoreDb.export_secret_key` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `fiduciary_standards` | `CoreDb.fiduciary_standards` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `from_keypair` | `CoreDb.from_keypair` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `from_public_key` | `CoreDb.from_public_key` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `from_secret_key` | `CoreDb.from_secret_key` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `generate` | `CoreDb.generate` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `generate_key` | `CoreDb.generate_key` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `generate_key` | `CoreDb.generate_key` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `generate_keypair` | `CoreDb.generate_keypair` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `get_audit_log` | `CoreDb.get_audit_log` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `get_audit_log` | `CoreDb.get_audit_log` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `get_context` | `CoreDb.get_context` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `get_default_signer` | `CoreDb.get_default_signer` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `get_signer` | `CoreDb.get_signer` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `hash_token` | `CoreDb.hash_token` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `issue_vc_mldsa` | `CoreDb.issue_vc_mldsa` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `key_id` | `CoreDb.key_id` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `list_keys` | `CoreDb.list_keys` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `list_keys` | `CoreDb.list_keys` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `public_key` | `CoreDb.public_key` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `public_key` | `CoreDb.public_key` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `quantum_resistance_threshold` | `CoreDb.quantum_resistance_threshold` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `remove_key` | `CoreDb.remove_key` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `rotation_policy` | `CoreDb.rotation_policy` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `set_key_id` | `CoreDb.set_key_id` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `should_rotate_key` | `CoreDb.should_rotate_key` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `should_rotate_key` | `CoreDb.should_rotate_key` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `sign` | `CoreDb.sign` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `sign` | `CoreDb.sign` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `sign` | `CoreDb.sign` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `sign_with_managed_context` | `CoreDb.sign_with_managed_context` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `sign_with_secret` | `CoreDb.sign_with_secret` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `verify_vc_mldsa` | `CoreDb.verify_vc_mldsa` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::fiduciary_crypto` | `verify_with_public` | `CoreDb.verify_with_public` | `crates/qualia-core-db/src/crypto/fiduciary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::pq_kem_shim` | `as_bytes` | `CoreDb.as_bytes` | `crates/qualia-core-db/src/crypto/pq_kem_shim.rs` |
| 100 | `qualia-core-db` | `crypto::pq_kem_shim` | `as_bytes` | `CoreDb.as_bytes` | `crates/qualia-core-db/src/crypto/pq_kem_shim.rs` |
| 100 | `qualia-core-db` | `crypto::pq_kem_shim` | `as_bytes` | `CoreDb.as_bytes` | `crates/qualia-core-db/src/crypto/pq_kem_shim.rs` |
| 100 | `qualia-core-db` | `crypto::pq_kem_shim` | `decapsulate` | `CoreDb.decapsulate` | `crates/qualia-core-db/src/crypto/pq_kem_shim.rs` |
| 100 | `qualia-core-db` | `crypto::pq_kem_shim` | `encapsulate` | `CoreDb.encapsulate` | `crates/qualia-core-db/src/crypto/pq_kem_shim.rs` |
| 100 | `qualia-core-db` | `crypto::pq_kem_shim` | `from_bytes` | `CoreDb.from_bytes` | `crates/qualia-core-db/src/crypto/pq_kem_shim.rs` |
| 100 | `qualia-core-db` | `crypto::pq_kem_shim` | `from_bytes` | `CoreDb.from_bytes` | `crates/qualia-core-db/src/crypto/pq_kem_shim.rs` |
| 100 | `qualia-core-db` | `crypto::pq_kem_shim` | `from_bytes` | `CoreDb.from_bytes` | `crates/qualia-core-db/src/crypto/pq_kem_shim.rs` |
| 100 | `qualia-core-db` | `crypto::pq_kem_shim` | `generate_kyber768_keypair` | `CoreDb.generate_kyber768_keypair` | `crates/qualia-core-db/src/crypto/pq_kem_shim.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit` | `chain_hash` | `CoreDb.chain_hash` | `crates/qualia-core-db/src/crypto/sanctuary_audit.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit` | `from_secret` | `CoreDb.from_secret` | `crates/qualia-core-db/src/crypto/sanctuary_audit.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit` | `generate` | `CoreDb.generate` | `crates/qualia-core-db/src/crypto/sanctuary_audit.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit` | `open_sealed` | `CoreDb.open_sealed` | `crates/qualia-core-db/src/crypto/sanctuary_audit.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit` | `seal_to` | `CoreDb.seal_to` | `crates/qualia-core-db/src/crypto/sanctuary_audit.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit` | `secret_bytes` | `CoreDb.secret_bytes` | `crates/qualia-core-db/src/crypto/sanctuary_audit.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit` | `unwrap_key` | `CoreDb.unwrap_key` | `crates/qualia-core-db/src/crypto/sanctuary_audit.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit` | `wrap_key` | `CoreDb.wrap_key` | `crates/qualia-core-db/src/crypto/sanctuary_audit.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit_dag` | `canonical_bytes` | `CoreDb.canonical_bytes` | `crates/qualia-core-db/src/crypto/sanctuary_audit_dag.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit_dag` | `derive_sessions` | `CoreDb.derive_sessions` | `crates/qualia-core-db/src/crypto/sanctuary_audit_dag.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit_dag` | `recomputed_id` | `CoreDb.recomputed_id` | `crates/qualia-core-db/src/crypto/sanctuary_audit_dag.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_audit_dag` | `verify_chain` | `CoreDb.verify_chain` | `crates/qualia-core-db/src/crypto/sanctuary_audit_dag.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_crypto` | `decrypt_sanctuary_chunk` | `CoreDb.decrypt_sanctuary_chunk` | `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_crypto` | `decrypt_sanctuary_chunk_in_place` | `CoreDb.decrypt_sanctuary_chunk_in_place` | `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_crypto` | `derive_chacha_nonce` | `CoreDb.derive_chacha_nonce` | `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_crypto` | `derive_chunk_nonce` | `CoreDb.derive_chunk_nonce` | `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_crypto` | `derive_lane_cipher_key` | `CoreDb.derive_lane_cipher_key` | `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_crypto` | `derive_sanctuary_key_material` | `CoreDb.derive_sanctuary_key_material` | `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_crypto` | `derive_sanctuary_key_material_argon2` | `CoreDb.derive_sanctuary_key_material_argon2` | `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_crypto` | `derive_xchacha_nonce` | `CoreDb.derive_xchacha_nonce` | `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_crypto` | `encrypt_sanctuary_chunk` | `CoreDb.encrypt_sanctuary_chunk` | `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_crypto` | `encrypt_sanctuary_chunk_in_place` | `CoreDb.encrypt_sanctuary_chunk_in_place` | `crates/qualia-core-db/src/crypto/sanctuary_crypto.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_keychain` | `delete_pepper` | `CoreDb.delete_pepper` | `crates/qualia-core-db/src/crypto/sanctuary_keychain.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_keychain` | `generate_pepper` | `CoreDb.generate_pepper` | `crates/qualia-core-db/src/crypto/sanctuary_keychain.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_keychain` | `get_pepper` | `CoreDb.get_pepper` | `crates/qualia-core-db/src/crypto/sanctuary_keychain.rs` |
| 100 | `qualia-core-db` | `crypto::sanctuary_keychain` | `store_pepper` | `CoreDb.store_pepper` | `crates/qualia-core-db/src/crypto/sanctuary_keychain.rs` |
| 100 | `qualia-core-db` | `crypto::verifiable_credential` | `decode_credential` | `CoreDb.decode_credential` | `crates/qualia-core-db/src/crypto/verifiable_credential.rs` |
| 100 | `qualia-core-db` | `crypto::verifiable_credential` | `encode_credential` | `CoreDb.encode_credential` | `crates/qualia-core-db/src/crypto/verifiable_credential.rs` |
| 100 | `qualia-core-db` | `crypto::verifiable_credential` | `issue` | `CoreDb.issue` | `crates/qualia-core-db/src/crypto/verifiable_credential.rs` |
| 100 | `qualia-core-db` | `crypto::verifiable_credential` | `verify_grounded` | `CoreDb.verify_grounded` | `crates/qualia-core-db/src/crypto/verifiable_credential.rs` |
| 100 | `qualia-core-db` | `crypto::zk_predicates` | `prove` | `CoreDb.prove` | `crates/qualia-core-db/src/crypto/zk_predicates.rs` |
| 100 | `qualia-core-db` | `crypto::zk_predicates` | `prove` | `CoreDb.prove` | `crates/qualia-core-db/src/crypto/zk_predicates.rs` |
| 100 | `qualia-core-db` | `crypto::zk_predicates` | `prove_range` | `CoreDb.prove_range` | `crates/qualia-core-db/src/crypto/zk_predicates.rs` |
| 100 | `qualia-core-db` | `crypto::zk_predicates` | `prove_threshold` | `CoreDb.prove_threshold` | `crates/qualia-core-db/src/crypto/zk_predicates.rs` |
| 100 | `qualia-core-db` | `crypto::zk_predicates` | `size_bytes` | `CoreDb.size_bytes` | `crates/qualia-core-db/src/crypto/zk_predicates.rs` |
| 100 | `qualia-core-db` | `crypto::zk_predicates` | `verify_range` | `CoreDb.verify_range` | `crates/qualia-core-db/src/crypto/zk_predicates.rs` |
| 100 | `qualia-core-db` | `crypto::zk_predicates` | `verify_threshold` | `CoreDb.verify_threshold` | `crates/qualia-core-db/src/crypto/zk_predicates.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `add_variable` | `CoreDb.add_variable` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `add_variable` | `CoreDb.add_variable` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `create_circuit` | `CoreDb.create_circuit` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `create_circuit` | `CoreDb.create_circuit` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `field_element_to_fr` | `CoreDb.field_element_to_fr` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `generate_keys` | `CoreDb.generate_keys` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `generate_proof` | `CoreDb.generate_proof` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `generate_proof` | `CoreDb.generate_proof` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `generate_proof` | `CoreDb.generate_proof` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `generate_proving_key` | `CoreDb.generate_proving_key` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `generate_semantic_proof` | `CoreDb.generate_semantic_proof` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `generate_verifying_key` | `CoreDb.generate_verifying_key` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `generate_witness` | `CoreDb.generate_witness` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `get_circuit` | `CoreDb.get_circuit` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `get_circuit_info` | `CoreDb.get_circuit_info` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `get_global_stats` | `CoreDb.get_global_stats` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `get_performance_stats` | `CoreDb.get_performance_stats` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `get_proving_key` | `CoreDb.get_proving_key` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `get_verifying_key` | `CoreDb.get_verifying_key` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `i128_to_field_element` | `CoreDb.i128_to_field_element` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `list_circuits` | `CoreDb.list_circuits` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `list_circuits` | `CoreDb.list_circuits` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `prove_matrix_multiply` | `CoreDb.prove_matrix_multiply` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `prove_matrix_multiply` | `CoreDb.prove_matrix_multiply` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `setup` | `CoreDb.setup` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `store_proving_key` | `CoreDb.store_proving_key` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `store_verifying_key` | `CoreDb.store_verifying_key` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `update_proof_metrics` | `CoreDb.update_proof_metrics` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `verify_proof` | `CoreDb.verify_proof` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `verify_proof` | `CoreDb.verify_proof` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `verify_proof` | `CoreDb.verify_proof` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `crypto::zk_proofs` | `verify_semantic_proof` | `CoreDb.verify_semantic_proof` | `crates/qualia-core-db/src/crypto/zk_proofs.rs` |
| 100 | `qualia-core-db` | `csd_storage` | `add_matrix_multiply` | `CoreDb.add_matrix_multiply` | `crates/qualia-core-db/src/csd_storage.rs` |
| 100 | `qualia-core-db` | `csd_storage` | `build` | `CoreDb.build` | `crates/qualia-core-db/src/csd_storage.rs` |

## How to use

1. Fix Q0 if non-empty.
2. Ship Q2 Poet Live for families already bound.
3. Packetize Q1 into Host binds (paired catalogs + handler), then Poet Live.
4. Re-run: `python scripts/vibe_incorporation_backlog.py`

See methodology for full rules.
