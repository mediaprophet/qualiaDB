# QualiaDB Whole-Biosphere, Corpus Estimation, Living-Systems & Selective Streaming Reference

**Status:** Complete (`PHASE-1`–`PHASE-5` Delivered & Verified — All Unit & Zero-Heap Tests Passing, `0` Warnings)  
**Crates:** `qualia-core-db`, `qualia-geospatial`, `qualia-cli`, `vibe`  
**Governing Contracts:** `AGENTS.md` (§0-A Two-Tier Zero-Heap, §0-B Sub-500-Line Module Hygiene), `C1–C5` Corpus/Cohort Estimation, `BS-30` Selective HMC Streaming, `BS-40` Typed Physical Cues, `BIO-FN/MC/EC` Living-Systems Coupling, `EAS-01..04` Earth & Space Weather

---

## 1. Architectural Invariants

1. **Tier-1 Mandatory Zero-Heap (`AGENTS.md` §0-A)**:
   - Comparable-cohort estimation (`cohort_estimator`), N3 defeasible/deontic/epistemic rule compilation (`n3_deontic_bridge`), sensitivity/permission gating (`cohort_access_gate`), biosphere trophic/mycorrhizal/microbial coupling (`domains::biosphere`), space-weather heliophysical evaluation (`space_weather_bridge`), selective HMC byte-range streaming (`selective_streamer`), CPU/GPU VRAM residency governance (`residency_budget`), and typed EMF/acoustic/physical cue evaluation & GPU packing (`typed_cue_profile`, `emf_cue_bridge`) execute with **zero heap allocations** (`assert_zero_alloc` verified).
2. **Sub-500-Line Module Hygiene (`AGENTS.md` §0-B)**:
   - Every created and decomposed `.rs` implementation file is strictly `< 500` lines (`180`–`436` lines), with clean directory-backed `mod.rs` routing.
3. **Clean-Room Provenance & Monolith Compile Decoupling**:
   - `poet_host/invoke/studio_domains/` is decomposed into four focused submodules (`receipt.rs`, `office_domains.rs`, `media_cad_domains.rs`, `sim_lifecycle_domains.rs`) with zero external `*-craft` code, and `Cargo.toml` exposes `minimal-fast-build` and `const fn` domain capability bitmasks (`active_domain_mask()`, `active_specialized_lib_mask()`).

---

## 2. Module & API Reference

### 2.1 Phase 1 — Clean-Room Host Invoke Decomposition & Capability Bitmasks

| Module | Path | Key Types & Entrypoints | Lines |
| :--- | :--- | :--- | :---: |
| **`studio_domains::receipt`** | [`crates/qualia-core-db/src/poet_host/invoke/studio_domains/receipt.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/poet_host/invoke/studio_domains/receipt.rs) | `DomainOpError`, `emit_domain_quin_from_args`, `validate_payload_budget` (`MAX_STUDIO_PAYLOAD_BYTES`) | 164 |
| **`studio_domains::office_domains`** | [`crates/qualia-core-db/src/poet_host/invoke/studio_domains/office_domains.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/poet_host/invoke/studio_domains/office_domains.rs) | Clean-room `NQuin` dispatchers for `doc_*`, `sheet_*`, `deck_*`, `pdf_*` | 158 |
| **`studio_domains::media_cad_domains`** | [`crates/qualia-core-db/src/poet_host/invoke/studio_domains/media_cad_domains.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/poet_host/invoke/studio_domains/media_cad_domains.rs) | Clean-room `NQuin` dispatchers for `photo_*`, `vec_*`, `audio_*`, `video_*`, `comp_*`, `cad_*` | 200 |
| **`studio_domains::sim_lifecycle_domains`** | [`crates/qualia-core-db/src/poet_host/invoke/studio_domains/sim_lifecycle_domains.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/poet_host/invoke/studio_domains/sim_lifecycle_domains.rs) | `sim_botanical_growth`, `sim_space_weather` (wired to real Shue/Burton/Chapman physics), `lifecycle_verify_gate` | 288 |
| **`domains` & `specialized_libs`** | [`crates/qualia-core-db/src/domains/mod.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/mod.rs), [`specialized_libs/mod.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/specialized_libs/mod.rs) | `active_domain_mask()`, `active_specialized_lib_mask()`, `minimal-fast-build` Cargo feature | 77 / 136 |

### 2.2 Phase 2 — Source-Preserving Corpus Ingest, Symbolic Logic & Cohort Estimation (`C1–C5`, `Task G`)

| Module | Path | Key Types & Entrypoints | Lines |
| :--- | :--- | :--- | :---: |
| **`csv_parser` & `json_parser`** | [`crates/qualia-core-db/src/sparql_library/parsers/csv_parser.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/sparql_library/parsers/csv_parser.rs), [`json_parser.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/sparql_library/parsers/json_parser.rs) | Deterministic `DOM-AX-7` subject hashing, `context[56..63]` sensitivity packing, `INVALID_VALUE_METADATA_FLAG` (`1 << 31`), `PRED_RAW_INVALID_LEXICAL` (never coerces malformed numbers to `0`) | 301 / 356 |
| **`cohort_estimator`** | [`crates/qualia-core-db/src/solvers/statistics/cohort_estimator.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/solvers/statistics/cohort_estimator.rs) | `CohortObservation`, `CohortEstimationMethod` (`Median`, `TrimmedMean`, `WinsorizedMean`, `Mean`, `UserDefinedWeighted`, `CustomFn`), `evaluate_cohort_estimate` (Tier-1 zero-heap) | 428 |
| **`cohort_estimation` (Vibe)** | [`crates/qualia-core-db/src/poet_host/invoke/stats/cohort_estimation.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/poet_host/invoke/stats/cohort_estimation.rs) | `Statistics.estimate_cohort` VibeScript host bridge | 174 |
| **`n3_deontic_bridge`** | [`crates/qualia-core-db/src/modalities/n3_deontic_bridge.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/modalities/n3_deontic_bridge.rs) | `compile_n3_rule_to_quin`, `compile_n3_rules_batch`, `compile_agreement_structure_to_deontic_norms`, `evaluate_bridged_deontic_and_epistemic` (`RuleType::{Strict, Defeasible, Defeater}` -> `DEFEATER_BIT`) | 430 |
| **`cohort_access_gate`** | [`crates/qualia-core-db/src/governance/cohort_access_gate.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/governance/cohort_access_gate.rs) | `PrincipalAndAgentGrant`, `derive_cohort_sensitivity_class`, `authorize_quin_query_slice`, `authorize_cohort_estimate_and_tag`, `authorize_explanation_trace`, `authorize_hmc_derivative_export` | 425 |

### 2.3 Phase 3 — Biosphere Living-Systems & Earth/Space Weather Engine (`BIO-FN/MC/EC`, `EAS-01..04`)

| Module | Path | Key Types & Entrypoints | Lines |
| :--- | :--- | :--- | :---: |
| **`funga_network`** | [`crates/qualia-core-db/src/domains/biosphere/funga_network.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/biosphere/funga_network.rs) | `HyphalNode`, `HyphalEdge`, `SporocarpStage`, `step_mycelial_nutrient_transport`, `advance_sporocarp_stage`, `emit_funga_state_quins` | 339 |
| **`microbial_colony`** | [`crates/qualia-core-db/src/domains/biosphere/microbial_colony.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/biosphere/microbial_colony.rs) | `ScaleBand` (`MicrometerUm`, `MillimeterMm`, `MeterM`), `ScaleBandTransform`, `transform_coordinate_to_mm`, `step_microbial_colony_monod` | 365 |
| **`ecological_coupling`** | [`crates/qualia-core-db/src/domains/biosphere/ecological_coupling.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/domains/biosphere/ecological_coupling.rs) | `TrophicRelationKind`, `TrophicCouplingRule`, `evaluate_ecological_couplings` (Tier-1 zero-heap) | 314 |
| **`space_weather_bridge`** | [`crates/qualia-core-db/src/poet_host/invoke/space_weather_bridge.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/poet_host/invoke/space_weather_bridge.rs) | `SpaceWeatherInput`, `SpaceWeatherOutput`, `evaluate_space_weather_zero_heap` (Shue magnetopause, Farris-Russell bow shock, Burton Dst, Newell Kp, Chapman TEC) | 269 |
| **`qualia_geospatial::frame`** | [`crates/qualia-geospatial/src/frame.rs`](file:///C:/github/qualiaDB/crates/qualia-geospatial/src/frame.rs) | `VerticalDatum` (`Wgs84Ellipsoidal`, `AhdOrthometric`, `Egm2008Geoid`, `LocalChartDatum`), `TemporalBasis`, `VerticalDatumTransform`, `validate_height_compatibility` | 394 |

### 2.4 Phase 5 — Selective HMC Streaming, Residency Budget & Typed Physical Cues (`BS-30`, `BS-40`)

| Module | Path | Key Types & Entrypoints | Lines |
| :--- | :--- | :--- | :---: |
| **`selective_streamer`** | [`crates/qualia-core-db/src/q42/selective_streamer.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/q42/selective_streamer.rs) | `SelectiveStreamQuery`, `HmcIndexSliceEntry`, `ResolvedByteRange`, `StreamReadOracle`, `resolve_selective_hmc_ranges_zero_heap`, `stream_verified_subrange_into` | 377 |
| **`residency_budget`** | [`crates/qualia-core-db/src/render/gpu/residency_budget.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/render/gpu/residency_budget.rs) | `ResidencyPoolCaps`, `ResidencySlot`, `ResidencyGovernor<MAX_SLOTS>`, `admit_or_evict_lru_zero_heap` (pinned slot immunity + fail-closed headroom enforcement) | 435 |
| **`typed_cue_profile`** | [`crates/qualia-core-db/src/modalities/typed_cue_profile.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/modalities/typed_cue_profile.rs) | `PhysicalCueDomain`, `PhysicalUnitKind`, `CalibrationStatus`, `PhysicalCueEmission`, `OrganismReceptorProfile`, `evaluate_receptor_cue_response_zero_heap`, `emit_typed_cue_quins_zero_heap` | 358 |
| **`emf_cue_bridge`** | [`crates/qualia-core-db/src/render/gpu/emf_cue_bridge.rs`](file:///C:/github/qualiaDB/crates/qualia-core-db/src/render/gpu/emf_cue_bridge.rs) | `EmfAcousticGpuDescriptor` (`#[repr(C)]`, 64 bytes, `Pod`, `Zeroable`), `pack_calibrated_cues_for_gpu_zero_heap` | 238 |
