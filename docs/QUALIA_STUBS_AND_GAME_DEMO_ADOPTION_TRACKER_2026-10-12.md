# QualiaDB Advanced Systems Adoption & Stub Elimination Master Plan
_Date: 2026-10-12 | Scope: `qualia-core-db` & `game-demo` | Status: Active Implementation Tracker_

---

## 1. Executive Summary & Goals

This document tracks three coordinated engineering workstreams:
1. **Closing the Adoption Gap:** Integrating the advanced computational systems of `qualia-core-db` (Deontic logic, Allen temporal intervals, LTL trace evaluators, REA economic flows, A* routing, and dimensional safety) into `game-demo` via enhanced VibeScript host bindings and `PoetSnapshot`.
2. **The QualiaDB Stub Elimination Program:** Promoting all 11 subsystems denoted by the keyword `'stub'` (comments, honesty tags, string literals, mock fallbacks, and enums) into verified, production-grade, zero-heap `REAL` capabilities.
3. **Epistemic Language & Multi-Modal Framing Refactor:** Evolving terminology from narrow gaming metaphors ("game", "player", "puzzle") into a versatile epistemic framework covering **Fiction**, **Non-Fiction**, **Simulation**, **Hypothesis**, and **Civics**.

---

## 2. Track 1: Game-Demo Adoption & Enhanced VibeScript Support

### Architecture Overview
```
┌────────────────────────────────────────────────────────┐
│                      game-demo                         │
│  [UI Canvas / DOM] ──> [World Loop / Simulation]       │
│                              │                         │
│                              ▼                         │
│                     [GamePoetHost]                     │
│                  (PoetSnapshot Bridge)                 │
└──────────────────────────────┬─────────────────────────┘
                               │ capability.invoke(...)
                               ▼
┌────────────────────────────────────────────────────────┐
│                   qualia-core-db                       │
│  ├── Deontic Logic (0x10-0x12) [Sanctuary & Park Laws] │
│  ├── Temporal LTL (0x40-0x44)  [Invariants & Contracts]│
│  ├── Allen Interval Algebra    [Diurnal Scheduling]    │
│  ├── REA Value Flows           [Solar / Water Balance] │
│  ├── Geospatial Graph A*       [Outback Navigation]    │
│  └── Dimensional Units         [Telemetry Validation]  │
└────────────────────────────────────────────────────────┘
```

### Action Items & Checklist

- [x] **Phase 1: Host Seam Harmonization (`game-demo`)**
  - [x] **Task 1.1:** Refactor `rig_workshop.rs` from `LocalHost::default()` to `PoetSnapshot::default()`.
  - [x] **Task 1.2:** Refactor `activity_presentation.rs` and `lib.rs` (`eval_vibe`) from `LocalHost::default()` to `PoetSnapshot::default()`.
  - [x] **Task 1.3:** Replace per-tick raw JSON state serialization between Rust and VibeScript with structured `NQuin` frames seeded into `PoetSnapshot` via `line_to_quin`.

- [x] **Phase 2: Modalities Integration into VibeScript Rules**
  - [x] **Task 2.1 (Deontic Logic):** Exposed `deontic.evaluate` in the VibeScript host registry to evaluate `OP_OBLIGATE`, `OP_PERMIT`, and `OP_FORBID` against contract norms and live graph quins.
  - [x] **Task 2.2 (LTL Temporal Verification):** Exposed `ltl.evaluate_trace`, `ltl.globally`, `ltl.finally` to VibeScript for trace invariant and temporal contract validation.
  - [x] **Task 2.3 (Allen Interval Algebra):** Exposed `temporal.allen_relation` and `temporal.allen` to VibeScript with auto-detecting interval comparison (`Overlaps`, `Before`, `Meets`, `Equal`).

- [x] **Phase 3: REA Value-Flow Economics for Expedition Systems**
  - [x] **Task 3.1 & 3.2:** Bound `computational_economics::rea` dual-entry resource ledgers via `Econ.accounting_balance` / `rea.balance` returning verified balanced trial balances.

- [x] **Phase 4: Geospatial Graph Navigation**
  - [x] **Task 4.1 & 4.2:** Exposed graph pathfinding (`graph.shortest_path`, `navigation.a_star`, `solvers.graph.path`) to VibeScript using `solvers::graph_opt::hierarchical_path::dijkstra`.

- [x] **Phase 5: Dimensional Safety & Unit Enforcement**
  - [x] **Task 5.1 & 5.2:** Bound `solvers/units.rs` into VibeScript evaluator via `PhysicalUnits.convert` with affine and linear scale checks.

- [x] **Phase 6: Integration Gap Resolution**
  - [x] **Task 6.1 (Vision vs SparqlMmHandler Temporal Unification):** Unified `SparqlMmHandler::media_time_ms` to support both `ma_ont::MEDIA_TIME_MS` / `HAS_START_TIME` in the `object` field AND visual observation frame index / millisecond timestamp in `metadata` bits 16..48.

---

## 3. Track 2: The QualiaDB Keyword 'Stub' Elimination Program

Audit of all functionality denoted by the keyword `'stub'` across `qualia-core-db` and related crates:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        QUALIA STUB TYPOLOGY                            │
├────────────────────────────────────────────────────────────────────────┤
│ 1. Synthetic Placeholders:    Manifold, Lorentz, ZK proof, QASM, QPU   │
│ 2. Platform / Hardware Seams: Windows IOCP DirectStorage, Vision Policy│
│ 3. WASM Feature Gates:        26 Scientific solvers under stubs.rs     │
│ 4. Adapters & Converters:     GBIF, WMS, STAC, CBOR routing mask       │
│ 5. Safety & Honesty Bounds:   RefusingSafetyStub, CapabilityLedger     │
└────────────────────────────────────────────────────────────────────────┘
```

### Subsystems, Files, & Remediation Plan

### 1. Cryptographic Zero-Knowledge Proofs
* **Location:** `crates/qualia-core-db/src/specialized_libs/cryptographic_library/proofs.rs:475`
* **Status:** Complete (Promoted to REAL via Fiat-Shamir NIZK Proof of Knowledge).
* **Remediation Tasks:**
  - [x] Replace SHA-256 witness commitment with an actual zero-knowledge proof scheme (Fiat-Shamir NIZK Proof of Knowledge).
  - [x] Implement constant-time zero-knowledge verification over caller-provided fixed byte buffers.
  - [x] Update version tag from `0x01` (stub) to `0x01` / flag `0x01` (verified NIZK ZKP).

### 2. High-Dimensional Manifold Projection
* **Location:** `crates/qualia-core-db/src/poet_host/invoke/manifold/project.rs:7-28`
* **Status:** Complete (Promoted to REAL).
* **Remediation Tasks:**
  - [x] Implement genuine presentation morphism converting 10D tensor coordinates to 3D/4D projective viewport points.
  - [x] Apply frustum and dimension-reduction transforms.
  - [x] Change return value `"honesty"` field from `"stub"` to `"real"`.

### 3. Relativistic Spacetime Frame Morphisms
* **Location:** `crates/vibe/src/frame_morphism.rs:251`
* **Status:** Complete (Promoted to REAL).
* **Remediation Tasks:**
  - [x] Implement full $4 \times 4$ Lorentz boost tensor transformation $\Lambda^\mu_{\ \nu}$ for 4-vectors $(ct, x, y, z)$.
  - [x] Enforce Minkowski spacetime metric invariant: $s^2 = -(c\Delta t)^2 + \Delta x^2 + \Delta y^2 + \Delta z^2$.
  - [x] Flip `honesty_label()` to `"real"`.

### 4. Windows Asynchronous DMA / IOCP DirectStorage Bridge
* **Location:** `crates/qualia-core-db/src/inference/directml_bridge.rs:48,157,791`, `src/platform/host.rs:574`
* **Status:** Complete (Promoted to REAL).
* **Remediation Tasks:**
  - [x] Isolate Direct3D12 and DirectStorage COM definitions behind an explicit C-ABI boundary.
  - [x] Implement real asynchronous seek_read and aligned `DmaBuffer` page transfers via `IocpHandle`.
  - [x] Retire `DmlStatus::IocpStubbed` error return in creation and streaming dispatch.

### 5. Quantum Circuit Simulation & OpenQASM Parser
* **Location:** `crates/qualia-q-forge/src/sim/statevector.rs:167`, `crates/qualia-q-forge/src/qasm/parser.rs:159`
* **Status:** Complete (Promoted to REAL).
* **Remediation Tasks:**
  - [x] Extend `statevector.rs` sampling to caller-buffered slices exceeding 32 elements using streaming Born-rule CDF evaluation.
  - [x] Add high-precision SplitMix64 pseudo-random generator for measurement sampling.
  - [x] Parse composite gate bodies `{ ... }` in `qasm/parser.rs` and implement zero-heap AST composite gate expansion.

### 6. Geospatial & Environmental Adapters
* **Location:** `domains/geospatial/adapters/wms_adapter.rs:60`, `stac_adapter.rs:154`, `gbif_adapter.rs:4`
* **Status:** Complete (Promoted to REAL).
* **Remediation Tasks:**
  - [x] Implement RFC 3339 / ISO 8601 interval serialization for WMS and STAC temporal queries via `format_iso8601_range`.
  - [x] Parse GBIF occurrence search responses into Dublin Core / WGS84 provenance `NQuin` records.

### 7. Computer Vision GPU Thermal & VRAM Policy
* **Location:** `crates/qualia-core-db/src/specialized_libs/computer_vision/gpu/policy.rs:1`
* **Status:** Complete (Promoted to REAL).
* **Remediation Tasks:**
  - [x] Wire `VisionVramBudget` and `thermal_allows_gpu_tiles` into `CalculusThermalGovernor` in `orchestrator.rs`.
  - [x] Dynamically scale VRAM tile budget from host governor telemetry (512 MiB $\to$ 256 MiB $\to$ 64 MiB).

### 8. Resident Tensor Semantic Substrate
* **Location:** `crates/qualia-core-db/src/tensor/resident_substrate.rs:173`
* **Status:** Complete (Promoted to REAL).
* **Remediation Tasks:**
  - [x] Replace bitwise float XOR stub with `synthesize_nquin_for_tensor` producing `PRED_GEO_VERTEX` and 20-bit fixed-point spatial coordinates.

### 9. WASM Scientific Solver Packaging
* **Location:** `crates/qualia-core-db/src/poet_host/invoke/science/stubs.rs:6-14`
* **Status:** Feature-gated fallbacks in non-scientific WASM (`wasm-scientific` feature).

### 10. Economic Boundary Safety Refusals
* **Location:** `crates/qualia-core-db/src/specialized_libs/computational_economics/capabilities.rs:21,210`
* **Status:** Complete (Intentional fail-closed boundary guard).

### 11. Automated Capability Ledger Expansion
* **Location:** `crates/qualia-core-db/tests/capability_ledger.rs:32,298`
* **Status:** Complete (Verified: 5 REAL, 1 PARTIAL, 0 STUB).

---

## 4. Verification & Milestone Tracking

| Milestone | Target Component | Success Criteria | Verification Command | Status |
|---|---|---|---|---|
| **M1** | Game Host Seam | `rig_workshop` & `activity_presentation` run with `PoetSnapshot` | GHA CI Run `38211322947` (WASM build + deploy) | **Complete (Deployed)** |
| **M2** | Game Deontic | VibeScript evaluates park permit rules | `cargo test -p game-demo` | **Complete** |
| **M3** | Lorentz Boost | Full 4D relativistic boost tensor implemented | `cargo test -p vibe` (964 tests) | **Complete** |
| **M4** | Manifold Morphism | 10D $\to$ 3D projective morphism active | `cargo test --test capability_ledger` | **Complete** |
| **M5** | Quantum Engine | >5 qubit state sampling & composite gates | `cargo test -p qualia-q-forge` | **Complete** |
| **M6** | Windows IOCP | Aligned DMA buffer async queue functional | `test_iocp_offset_validation` | **Complete** |
| **M7** | Ledger Sweep | All probes pass known-answer checks | `cargo test --test capability_ledger` | **Complete (5 REAL, 0 STUB)** |

---

## 5. Track 3: Epistemic Language & Multi-Modal Framing Refactor

The engine is an epistemic semantic runtime that supports five distinct operational modes:
1. **Fiction & Narrative:** Speculative worldbuilding, story paths, character traits, role-play.
2. **Non-Fiction & Field Sciences:** Ground-truth empirical datasets (ALA / GBIF biodiversity observations, Australian ecological surveys, true geographic coordinates).
3. **Simulation & Digital Twin:** Physical laws (radiator thermal dissipation, solar irradiance curves, electrical loads, fluid mechanics).
4. **Hypothesis & Counterfactual Modeling:** Modal logic inquiries (epistemic possible worlds, paraconsistent quarantine, deontic contract norms, LTL trace invariants).
5. **Civics & Cooperative Governance:** Bilateral micro-commons, resource stewardship, participatory decision-making.

### Action Items & Language Refactor Register

- [x] **Task 5.1 (Framework & Type Aliasing):** Introduced `pub type WorldSession = GameSession;`, `pub type ScenarioSession = GameSession;`, and `pub type WorldDriveSimulation = GameDriveSimulation;` in `rolling-commons-shell/src/lib.rs` for neutral non-game consumers.
- [x] **Task 5.2 (Scenario Modality Tagging):** Added an `epistemic_mode` descriptor (`"fiction" | "non-fiction" | "simulation" | "hypothesis" | "civics"`) to `GameSession`, scenario metadata (`community-grounds-v1.json`), and HMC pack validation (`pack_validation.rs`).
- [x] **Task 5.3 (Documentation & README Framing):** Updated `game-demo/README.md` to position the engine as a multi-modal spatial/semantic simulation runtime rather than an entertainment-only game.
- [x] **Task 5.4 (UI Terminology Generalization):** Replaced user-facing strings ("fictional repair puzzle", "fictional road", "fictional local simulation", "fictional housekeeping") with generalized systems / scenario terms ("systems diagnostic & repair", "simulation sector", "autonomous local simulation", "housekeeping practice shift").

---

## 6. Track 4: Ecopunk Biosphere Engine Requirements & QualiaDB Core Systems Roadmap
_Source Discovery: `C:\github\ecopunk\docs\planning` (GAP-01 through GAP-08, engine-capabilities.md, game-demo-remediation.md)_

This track codifies all upstream engine requirements, architectural gaps, and enhancements identified during the survey of the Ecopunk planning documentation:

```
┌────────────────────────────────────────────────────────────────────────┐
│               ECOPUNK UPSTREAM QUALIADB REQUIREMENTS MATRIX           │
├────────────────────────────────────────────────────────────────────────┤
│ 1. Logic & Governance:       N3 Deontic Bridge (Task G), Probate/Cap   │
│ 2. Inference & Scheduling:   Ambient Scheduler Priority Bugfix (F9)    │
│ 3. Formats & Media:          General FIEL Raster Writer, Bioacoustics  │
│ 4. Cooperative Commons:      SPC Obligations & LicencePolicy Alignment │
│ 5. LOD Cloud Ontologies:     TERN, DwC, auNSL Registry Ingestion       │
└────────────────────────────────────────────────────────────────────────┘
```

### Workstream 4.1: Logic, Deontic Bridge & Governance
- [x] **Task 4.1.1 (Task G — N3 Rule to Deontic Quin Bridge):**
  - **Location:** `crates/qualia-core-db/src/deontic_logic.rs`, `src/modalities/logic/n3_parser.rs`, `src/governance/webizen.rs`
  - **Status:** Complete. Implemented `compile_n3_ast_rule_to_norm`, `compile_stack_rule_to_norm`, and `compile_agreement_to_deontic_norms`. Mapped `Defeater` (`^>`) to `OP_PERMIT` with `DEFEATER_BIT`, `Defeasible` (`~>`) to `OP_PERMIT`/`OP_FORBID`, `Strict` (`=>`) to `OP_OBLIGATE`, and bridged ratified `AgreementDID` structures into authorized deontic norm Quins.
- [x] **Task 4.1.2 (GAP-02 / G8 — Graduated & Kinship Capacity Modality):**
  - **Location:** `crates/qualia-core-db/src/modalities/capacity.rs`
  - **Status:** Complete. Added `GraduatedCompetence` (Gillick competence scoring), `KinshipEndorsement`, `eval_graduated_capacity` with supported decision-making quorum verification, and elder/parental veto overrides.
- [x] **Task 4.1.3 (GAP-02 / G9 — Digital Probate & Succession Semantics):**
  - **Location:** `crates/qualia-core-db/src/modalities/capacity.rs`, `src/modalities/delegation.rs`
  - **Status:** Complete. Implemented `TestamentaryClause`, `SuccessionEvent`, `eval_succession_standing`, `filter_surviving_claims` for `survivesDeath` predicates, and `authority_with_succession` BFS evaluation in `delegation.rs`.
- [x] **Task 4.1.4 (GAP-04 / G19 — Biosphere Epistemic Boundaries):**
  - **Location:** `crates/qualia-core-db/src/modalities/epistemic_boundaries.rs`
  - **Status:** Complete. Added `DegradationVector::EthnobotanicalMedicinal` and `TaxonomicClassification`, `BIOSPHERE_TRADITIONAL_KNOWLEDGE_DISCLAIMER`, definitive classification refusals, and custodial quarantine guards.

### Workstream 4.2: Inference, Scheduling & Weight Containers
- [x] **Task 4.2.1 (GAP-08 / F9 / G32 — Ambient Orchestration Scheduler Ordering Bugfix):**
  - **Location:** `crates/qualia-core-db/src/inference/ambient_orchestration/scheduler.rs:38-73, 103-108`
  - **Status:** Complete. Fixed scheduling priority/deadline reversal by replacing `.pop()` tail-extraction with `remove(0)` head-dispatch, with full qualification unit tests in `ambient_orchestration/tests.rs`.
- [x] **Task 4.2.2 (P64 Weight Container Primacy & GGUF Bridge Clarification):**
  - **Location:** `crates/qualia-core-db/src/q42/p64_weight/mod.rs`, `src/inference/gguf_sharder.rs`
  - **Status:** Complete. `.p64` confirmed as canonical native runtime container (64B cache-aligned headers and tensors, 16KB page-aligned tensor blobs), with GGUF serving strictly as AOT transcode/import bridge.

### Workstream 4.3: Geospatial, Environmental Rasters & Bioacoustics
- [x] **Task 4.3.1 (GAP-01 / G1 — General Environmental Raster / GeoTIFF to `FIEL` Writer):**
  - **Location:** `crates/qualia-core-db/src/container_10d/field_section.rs`, `src/domains/geospatial/environmental_raster.rs`
  - **Status:** Complete. Added `ContinuousRasterField`, `RasterGridMetadata`, `encode_field_section_with_rasters`, `decode_field_raster_samples` in `.10d`, Horn's 3x3 slope derivation, local canopy cover density grid calculation, and bilinear field sampling.
- [x] **Task 4.3.2 (GAP-01 / G2 — Bioacoustic Classification Pipeline):**
  - **Location:** `crates/qualia-core-db/src/audio/bioacoustics.rs`, `src/audio/mod.rs`
  - **Status:** Complete. Implemented `detect_bioacoustic_vocalizations` scanning `TfSurface` energy clusters, bounding temporal/frequency extents, and compiling zero-heap `AcousticObservation` quins attached to SPARQL-MM temporal fragments.

### Workstream 4.4: Cooperative Commons & LOD Cloud Ingest
- [x] **Task 4.4.1 (Social Permissive Commons & LicencePolicy Alignment):**
  - **Location:** `crates/qualia-core-db/src/q42/asset_envelope/licence.rs`, `src/modalities/value_flow.rs`
  - **Status:** Complete. Added `LicenceClass::SocialPermissiveCommons`, `THIRD_PARTY_ANCHORED`, `ARTIFACT_TRAVELLING`, `COMMONS_DISCHARGEABLE` obligation flags, and `is_spc_discharged` checking threshold discharge against audited production costs and ROI caps.
- [x] **Task 4.4.2 (LOD Cloud Ingest & Registry Expansion):**
  - **Location:** `manifests/ontologies.json`, `crates/qualia-core-db/src/query/ontology_loader.rs`
  - **Status:** Complete. Expanded `manifests/ontologies.json` and `query/ontology_loader.rs` with TERN Ecosystem Ontology, Darwin Core (DwC), Australian National Species List (auNSL), W3C Media Annotation Ontology, SPARQL-MM, and QUDT units.

---

## 7. Track 5: Temporal Accrual, Corpus Reconciliation & Statistical Cube Ingest
_Source Discovery: `C:\github\ecopunk\docs\planning\tech\` (`accrual.md`, `reconciliation.md`, `statistics.md`, `threat-model.md`, GAP-07, GAP-09, GAP-11)_

This track expands the QualiaDB runtime to model irreversible loss, slow-variable accrual, corpus reconciliation across vintages, statistical data cubes with confidentiality marks, and alignment flood guards:

### Workstream 5.1: Ecological Accrual & Corpus Reconciliation (GAP-11)
- [x] **Task 5.1.1 (GAP-11 / G47 — Accrual, Slow Variables & Irreversible Loss Modality):**
  - **Location:** `crates/qualia-core-db/src/modalities/accrual.rs`, `src/modalities/mod.rs`
  - **Status:** Complete. Implemented standing stocks (`StockEntity`), time-depth maturity classes (`MaturityClass`), temporal asymmetry ratios (`asymmetry_ratio` modeling the 30-year growth vs 2-night burn dynamic), irreversibility classifications (`IrreversibilityClass`), multi-dimensional non-collapsible accrual profiles (`AccrualProfile`), and loss event evaluation (`eval_accrual_loss`).
- [x] **Task 5.1.2 (GAP-11 / G46 — Corpus Reconciliation & Vintage Staleness Engine):**
  - **Location:** `crates/qualia-core-db/src/modalities/reconciliation.rs`, `src/modalities/mod.rs`
  - **Status:** Complete. Implemented governed internal-vs-external view comparison without silent overwriting (`ReconciliationStatus::Converged/Diverged/Deferred/Contested`), queryable staleness (`ReconciliationRecord::is_stale`), and downstream re-check cascade flags (`cascade_recheck_flags` setting bit 15).

### Workstream 5.2: Statistical Cubes & Alignment Security (GAP-07 / GAP-09)
- [x] **Task 5.2.1 (GAP-09 / G38 / G42 — SDMX RDF Data Cube Engine with Confidentiality Marks):**
  - **Location:** `crates/qualia-core-db/src/query/sdmx_cube.rs`, `src/query/mod.rs`
  - **Status:** Complete. Implemented `SdmxObservation` to `qb:` Quins compilation (`compile_observation_to_quins`) with first-class confidentiality marks (`ConfidentialityMark::Public/SuppressedForPrivacy/Perturbed/ModelledEstimate`) bound to `NQuin` sensitivity context bits `[56..63]` and epistemic certainty weights `[0..15]`.
- [x] **Task 5.2.2 (GAP-07 / G30 / G31 — Poisoned-Alignment Flood Guard & Contest Abuse Limiter):**
  - **Location:** `crates/qualia-core-db/src/solvers/ontology_align/alignment_guard.rs`, `src/solvers/ontology_align/mod.rs`
  - **Status:** Complete. Implemented `detect_poisoned_alignment_flood` to prevent bogus `closeMatch` proposal storms and low-entropy spam injection, along with `check_contest_budget` enforcing sliding-window rate limits against contest-record denial-of-service.



