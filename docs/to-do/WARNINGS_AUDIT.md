# Warnings Audit — qualia-core-db 0.0.28

_Generated 2026-07-01. 360 unique warning lines from `cargo build -p qualia-core-db --lib`._

This audit categorises every warning by **root cause** and identifies the
**half-done work** that produces each cluster. The goal is not to suppress
warnings but to identify what needs to be **fully implemented** (or deleted)
to resolve the underlying cause.

---

## Summary by category

| Category | Warnings | Action required |
|----------|----------|-----------------|
| 1. Scaffolding stubs (fields never read) | ~190 | Implement real logic or delete dead fields |
| 2. Dead code from refactors | ~25 | Delete or re-wire |
| 3. Half-wired data flow | ~25 | Wire parsed data into downstream consumers |
| 4. Glob re-export false positives | ~5 | Explicit imports or module-level `allow` |
| 5. Deprecated `Array::from_slice` | 6 | Migrate to `TryFrom<&[u8]>` |
| 6. Naming conventions | 4 | Rename variants |
| 7. Trivial unused imports | 8 | Remove import lines |
| 8. Misc (unused constants, dead assigns) | ~10 | Case-by-case |

---

## 1. Scaffolding stubs — fields declared, `new()` initialises empty containers, methods return hardcoded values

**~190 warnings across 7 files.** This is the dominant pattern. Each file
defines a deep type taxonomy (structs with many fields), a `new()` that
initialises every field to an empty `HashMap`/`Vec`/default, and methods that
either return hardcoded constants or delegate to a real implementation
elsewhere — never touching the scaffolding fields.

### 1a. `specialized_libs/medical_computing/mod.rs` — 48 warnings

**Pattern:** `RiskAssessment`, `AccessLogging`, `LogAnalysis`, `AnomalyDetection`,
`ConsentManagement`, `AccessControl`, `AuthenticationManager`, `SessionManager`,
`MultiFactorAuth`, `AuthorizationEngine`, `PermissionManager`, `RoleManager`,
`DiagnosticEngine`, `SymptomChecker`, `LabResultAnalyzer`, `ImagingAnalyzer`,
`HealthRiskAssessment`, `TreatmentPlanner`, `ClinicalDecisionSupport`, etc.

Each struct has fields like `risk_models: HashMap<String, RiskModel>`,
`detection_algorithms: HashMap<...>`, `consent_records: HashMap<...>` that are
initialised empty in `new()` and **never populated or read**.

**Root cause:** The medical computing module was built as a type taxonomy
scaffold (all the types are defined with full field layouts) but the actual
clinical engines (Framingham, CHA₂DS₂-VASc, SCORE2, drug interactions) were
implemented in `webizen.rs::execute_vm_frame` as `Native*` opcode handlers,
**not** in this module. The scaffolding structs here are orphaned.

**Resolution options:**
- **Wire in:** Make the `Native*` clinical handlers delegate to methods on
  these structs (e.g., `HealthRiskAssessment::compute_framingham(&self, patient)`
  instead of the inline computation in `webizen.rs`).
- **Or delete:** Remove the scaffolding structs and keep only the types that
  `webizen.rs` actually uses. This would remove ~48 warnings and ~1500 lines
  of dead type definitions.

### 1b. `specialized_libs/financial_modeling/mod.rs` — 37 warnings

Same pattern: `PortfolioOptimizer`, `RiskAnalyzer`, `PricingEngine`,
`VolatilityModel`, `YieldCurveModel`, `CreditRiskModel`, `MarketRiskModel`,
`OperationalRiskModel`, `LiquidityRiskModel`, `StressTestEngine`,
`BacktestEngine`, `ComplianceMonitor`, `RegulatoryReporter`, etc.

Fields like `optimization_methods`, `risk_models`, `pricing_models` are
initialised empty and never used. The real financial computations (Monte Carlo
VaR, Black-Scholes) are in `webizen.rs::execute_vm_frame` native handlers.

**Resolution:** Same as 1a — either wire the native handlers to use these
structs, or delete the scaffolding.

### 1c. `specialized_libs/cryptographic_library/mod.rs` — 37 warnings + 4 deprecated + 4 naming

`EncryptionEngine`, `KeyManagementSystem`, `DigitalSignature`, `HashFunction`,
`MacAlgorithm`, `KeyDerivation`, `RandomNumberGenerator`, `ZeroKnowledgeProof`,
`HomomorphicEncryption`, `SecureMultiPartyComputation`, `PostQuantumCrypto`,
`ThreatDetection`, `IntrusionDetection`, `SecurityAuditor`, `ComplianceMonitor`,
`AuditTrail`, `SecurityScheduler`, `SecurityAnalytics`, `IncidentResponse`,
`SecurityReporting`, `VulnerabilityManager`, `PatchManager`, etc.

Fields like `encryption_algorithms`, `key_store`, `signing_algorithms` are
initialised empty and never used. The real crypto (Ed25519, AES-GCM, BFV HE)
is in `identity/agency.rs`, `identity/key_vault.rs`, and
`specialized_libs/linear_algebra/privacy/`.

**Additional issues in this file:**
- **Naming:** `PCI_DSS` → should be `PciDss` (2 occurrences), `zkSNARKs` →
  `ZkSnarks`, `zkSTARKs` → `ZkStarks`. These are public enum variants; renaming
  is a breaking API change but the variants are never constructed so impact is
  nil.
- **Deprecated `Array::from_slice`:** 4 occurrences at lines 3357, 3361, 3387,
  3390. The `aead` crate deprecated `from_slice` in favour of `TryFrom`. Fix:
  `Nonce::from_slice(&bytes)` → `<Nonce as TryFrom<&[u8; 12]>>::try_from(&bytes)`
  or `Nonce::try_from(&bytes[..12]).unwrap()`.

### 1d. `specialized_libs/engineering_analysis/mod.rs` — 25 warnings

`StructuralAnalysis`, `ThermalAnalysis`, `FluidDynamics`, `ModalAnalysis`,
`HarmonicAnalysis`, `FatigueAnalysis`, `OptimizationEngine`, `MeshGenerator`,
`ResultExtractor`, `VisualizationEngine`, `ReportGenerator`, `ProbabilityAnalysis`,
`ReliabilityAnalysis`, `FailureAnalysis`, `PipeFlowAnalysis`, `HeatTransferAnalysis`,
`NavierStokesSolver`, `BucklingAnalysis`, etc.

Same scaffolding pattern. The real engineering computations (CFD, FEA) are
either in `engineering_analysis/cfd.rs` (which has its own warnings — see §2)
or not implemented at all.

### 1e. `specialized_libs/chemistry_modeling/mod.rs` — 24 warnings

`MolecularSimulator`, `SimulationEngine`, `ForceFieldCalculator`,
`MolecularIntegrator`, `BoundaryConditions`, `TimeStepControl`,
`EnsembleManager`, `TemperatureController`, `ForceField`, `BondedInteractions`,
`NonBondedInteractions`, `Electrostatics`, `VanDerWaals`, `QuantumChemistry`,
`SemiEmpiricalMethod`, `DftCalculator`, `ReactionNetwork`, `KineticsModel`,
`ThermodynamicCalculator`, `PhaseDiagram`, etc.

**Key finding:** `MolecularSimulator::run_simulation` **delegates to the real
`molecular_dynamics::run_md`** (Lennard-Jones + velocity-Verlet), but the
sub-structs (`SimulationEngine`, `ForceFieldCalculator`, `MolecularIntegrator`)
are scaffolding whose fields are never read — the real work happens entirely
in the `molecular_dynamics` submodule.

**Resolution:** The facade structs should either be removed (the
`MolecularSimulator` can hold the config directly) or the `molecular_dynamics`
module should be refactored to use them. The `basis_set.rs` submodule (Task H)
is the correct path forward — it provides real `ContractedShell` /
`MolecularBasis` types that the integral engine (Task I) can use.

### 1f. `net/acoustic_ble_mesh.rs` — 14 warnings

`AcousticChannelManager`, `AcousticModemController`, `AcousticProtocolHandler`,
`BleGattServer`, `BleAdvertisingManager`, `BleScanner`, `BleConnectionManager`,
`MeshRoutingTable`, `MeshCongestionControl`, `MeshMessageQueue`,
`MeshPersistenceManager`, `MeshPerformanceMonitor`, etc.

**Pattern:** `MeshNetworkManager::discover_nodes` returns **hardcoded fake
nodes** (`acoustic_node_0` through `acoustic_node_4` with fixed coordinates).
`send_message` just `thread::sleep(500ms)`. The sub-component fields are
initialised but never read because the methods are simulations.

**Resolution:** This is a crisis-mode networking module. Either:
- Implement real acoustic/BLE discovery and transmission (requires platform
  APIs), or
- Mark the entire module `#![allow(dead_code)]` with a doc comment explaining
  it's a simulation scaffold for delay-tolerant networking research, or
- Move it behind a `crisis-net` feature flag so it doesn't compile by default.

### 1g. `inference/ambient_orchestration.rs` — 13 warnings

`SubThresholdOrchestrator`, `WorkloadAnalyzer`, `ResourceAllocator`,
`AdaptationEngine`, `PowerManager`, `BatteryMonitor`, `ThermalMonitor`,
`PowerOptimizer`, `OptimizationEngine`, `TaskScheduler`, `AmbientMonitor`.

**Pattern:** `WorkloadAnalyzer::analyze_workload` returns hardcoded constants
(`current_load: 0.5, predicted_load: 0.6, ...`). `ResourceAllocator` has no
methods beyond `new()`. `AdaptationEngine::adapt_policy` uses the input
analysis but never touches its own `adaptation_history` / `learning_rate` fields.

**Resolution:** The `PowerManager` is partially wired (`can_execute` reads
`battery_monitor` and `thermal_monitor`), but the rest is scaffolding. Either
implement real workload prediction (the `WorkloadSample` / `PredictionModel`
types are ready for data) or reduce to just the parts `PowerManager` actually
uses.

---

## 2. Dead code from refactors — code left behind when functionality moved

### 2a. `services/daemon.rs` — 16 warnings

**Root cause:** The daemon was migrated from `warp` to `axum`
(`webizen_server::spawn_loopback_server`). The following warp-era helpers were
left behind as dead code:

- `OFFICIAL_WEB_HUB_ORIGIN`, `QUERY_PAYLOAD_LIMIT_BYTES`, `PROXY_FETCH_MAX_BYTES`
  — constants for a CORS proxy that was removed
- `proxy_target_allowed`, `ip_is_restricted` — SSRF guard functions for the
  removed proxy
- `ws_query_error_json` — WebSocket error formatter from the pre-axum era
- `decode_bench_load_b64` — manual base64 decoder (replaced by `base64` crate?)
- `NativeQueryRequest`, `OutputFormat`, `negotiate_format` — REST query
  endpoint types that were never wired into the axum routes
- `DaemonSecurity::token` — stored but never checked (auth moved elsewhere)
- `SinkExt` import — unused after warp removal

**Resolution:** Delete the dead warp-era helpers. If `negotiate_format` /
`OutputFormat` are intended for a future REST endpoint, wire them into the
axum router; otherwise remove them. The `proxy_target_allowed` /
`ip_is_restricted` SSRF guards should be kept **if** a proxy endpoint is
planned — they're security-critical and shouldn't be deleted just because
they're currently unused.

### 2b. `gguf_bridge/cpu_ops.rs` — 2 warnings

- `update_streaming_argmax` — convenience wrapper that calls
  `update_streaming_argmax_sieved` with `None`. All call sites use the sieved
  version directly. **Delete the wrapper.**
- `relu_inplace` — ReLU activation kernel. The LLM forward path uses SiLU
  (SwiGLU), not ReLU. **Delete unless a ReLU-based model is planned.**

### 2c. `gguf_bridge/mod.rs` — `mock_pipeline` + `elem_rms_norm_pipeline`

- `mock_pipeline` — legacy f32×f32 fallback pipeline, created in `init.rs` but
  never dispatched. Replaced by the real Q6_K embedding pipeline. **Delete the
  field and its creation in `init.rs`.**
- `elem_rms_norm_pipeline` — only read in `mc8_wasm/encode.rs`
  (`#[cfg(target_arch = "wasm32")]`). On native builds it's dead. **Gate the
  field with `#[cfg(target_arch = "wasm32")]`** or add `#[allow(dead_code)]`
  with a comment.

### 2d. `services/daemon_swarm.rs` — 2 warnings

`DNSSEC_TXT_RECORD`, `DNSSEC_CERT_RECORD` — constants for DNSSEC-based swarm
discovery that was never implemented. **Delete or implement.**

### 2e. `services/webizen_server.rs` — 2 warnings

`SinkExt` import (unused after axum migration) and
`QUERY_PAYLOAD_LIMIT_BYTES` (duplicate of the one in `daemon.rs` — both
unused). **Remove both.**

### 2f. `q42/p64_weight.rs:1092` — unused `std::io::Write`

Import left behind after a refactor. **Remove the import.**

### 2g. `render/gpu/mod.rs` — 2 warnings

`BloomChain` fields `hdr_texture`, `blur_a`, `blur_b` (the textures themselves
— only their `_view` counterparts are used in render passes), and
`half_width` / `half_height` (stored but never read). The textures are
owned resources that must be kept alive, so **add `#[allow(dead_code)]`** with
a comment explaining they're lifetime anchors. `half_width`/`half_height` can
be deleted.

### 2h. `render/pga.rs:365` — `approx_eq3` never used

Utility function for approximate equality of 3-vectors. **Delete or use in
tests.**

### 2i. `engineering_analysis/cfd.rs` — 2 warnings

- `apply_bc` — boundary condition application function, never called. The CFD
  solver probably needs this. **Wire it into the solver or delete.**
- `OPP` constant — never used. **Delete.**

---

## 3. Half-wired data flow — data is parsed/stored but never consumed

### 3a. `sparql_library/parsers/n3_star.rs:243` — `rule_type` parsed but discarded

`ParseResult::Rule { rule_type }` is constructed with the correct `RuleType`
(Strict, Defeasible, Defeater, Linear) but the consumer at line 277 matches
`ParseResult::Rule { .. }` and discards the type.

**Root cause:** The N3-Star parser correctly identifies rule types but the
downstream code doesn't differentiate behaviour based on type. This is the
same gap identified in AGENTS.md §2-B: "There is currently no compiler from
`Rule { rule_type: Defeater }` → deontic Quin. That is Task G."

**Resolution:** Wire `rule_type` into the rule compilation path so that
Defeasible/Defeater/Linear rules are handled differently (Task G).

### 3b. RDF-Star parsers — `context_hash` + string fields never consumed

**4 files:** `nquads_star.rs`, `ntriples_star.rs`, `trig_star.rs`,
`turtle_star.rs`

Two patterns:
1. `context_hash: u64` stored in each parser struct but never read during
   parsing (the graph/context is extracted per-line instead).
2. `subject_str`, `predicate_str`, `object_str`, `graph_str` (and
   `outer_predicate_str` etc.) allocated as `String` fields in `ParseResult`
   variants but never read — only the `u64` hashes are consumed.

**Resolution:**
- Remove `context_hash` from parser structs (or use it as a fallback when a
  line has no explicit graph).
- Remove the `*_str` fields from `ParseResult` — they allocate strings in the
  parsing hot path that are immediately discarded. This is a **performance
  bug**, not just a warning.

### 3c. `solvers/qpu/dispatcher.rs` — 2 warnings

`JobState` fields `job`, `enqueued_at_ms`, `retries` are stored but never read.
`InternalStatus` variants `Queued`, `Running`, `Completed` are never
constructed (only `Submitted` and `Failed` are used).

**Root cause:** The QPU job dispatcher stores state but never transitions
jobs through the full lifecycle. Jobs go straight from `Submitted` to
`Failed` — there's no `Running` → `Completed` transition.

**Resolution:** Implement the full job lifecycle: poll provider for status,
transition `Submitted` → `Running` → `Completed`, increment `retries` on
failure with backoff.

### 3d. `csd_storage.rs` — 3 warnings

`CsdScheduler` fields `running_operations`, `completion_queue` never used.
`CsdPerformanceMonitor` fields `device_metrics`, `function_metrics` never read.
Methods `bytes_to_f32_slice`, `serialize_dimensions` never called.

**Root cause:** `CsdScheduler::execute_operation` is a simulated stub that
returns hardcoded `Success` with `execution_time: 1000`. Operations go
directly from `pending` to `completions` without tracking running state.

**Resolution:** Either implement real CSD device interaction (track running
ops, collect completions asynchronously, populate metrics) or mark the module
as simulated with `#[allow(dead_code)]`.

### 3e. `inference/inference_agent.rs` — 2 warnings

`cpu_embedding_forward` and `lora_embedding_forward` — defined but never
called. The real embedding forward path uses
`GgufTensorIndex::dequantize_token_embedding_into`.

**Resolution:** Delete these functions or wire them as fallback paths when
no GGUF tensor index is available.

### 3f. `modalities/logic/deontic.rs:492` — `term_uri_hash` never used

Utility function for hashing term URIs. **Wire into deontic Quin construction
or delete.**

### 3g. `modalities/logic/rules.rs:26` — `RULE_EVAL_PREDICATE` never used

Constant for rule evaluation predicate hash. **Wire into rule evaluation or
delete.**

### 3h. `modalities/logic/specialized_libs_shacl.rs:352` — `require_solvent_model` never read

SHACL constraint field that's parsed but never enforced. **Implement the
solvent model check or remove the field.**

### 3i. `identity/agency.rs:5` — `LANE_KEY_LENGTH` never used

Constant for lane key length. **Use in `derive_lane_key` or delete.**

### 3j. `identity/vault_manifest.rs:97` — `cbor_ld_parser` never read

Field in vault manifest. **Wire into manifest validation or remove.**

### 3k. `q42/yaml_ld_q42.rs:21` — `buffer` and `cursor` never read

Parser state fields. **Wire into parsing logic or remove.**

### 3l. `q42_lex.rs:19` — `LEX_TAG_WEBIZEN` never used

Lexer tag constant. **Wire into lexer dispatch or delete.**

### 3m. `platform/host.rs:202` — `get_inactive_buffer_mut` / `get_inactive_buffer` never used

Double-buffering accessor methods. **Wire into rendering loop or delete.**

### 3n. `platform/local_scheduler.rs:221` — `job_receiver` / `wal_path` never read

Scheduler state fields. **Wire into job dispatch or remove.**

### 3o. `solvers/learning/gaussian_process/mod.rs:24` — `noise_var` never read

GP noise variance parameter. **Wire into GP inference or remove.**

### 3p. `solvers/learning/survival/cox.rs:48` — `iters` assigned but never read

Loop counter in Cox regression. **Use the iteration count for convergence
reporting or remove the assignment.**

### 3q. `governance/webizen_bytecode.rs:185` — `stats` assigned but never read

VM execution stats. **Wire into telemetry or remove.**

---

## 4. Glob re-export false positives — 5 warnings

### 4a. `gguf_bridge/gpu_params.rs` — `ELEM_OP_RMS_NORM`, `ELEM_OP_ADD_RESIDUAL`

These constants **are used** in `prefill_async.rs`, `forward.rs`, `ffn.rs`, and
`mc8_wasm/encode.rs` — but accessed via a two-level glob chain:
`pub(crate) use gpu_params::*;` in `mod.rs`, then `use super::*;` in each
submodule. Rust's dead-code analysis doesn't trace through glob re-exports,
producing a false positive.

**Resolution:** Either:
- Add explicit `use super::gpu_params::{ELEM_OP_RMS_NORM, ELEM_OP_ADD_RESIDUAL,
  ELEM_OP_SILU_MUL};` in each submodule that uses them, or
- Add `#![allow(dead_code)]` at the top of `gpu_params.rs` with a comment.

### 4b. `net/ebpf_filter.rs:395` — `XPC_SERVICE` associated constant

Same glob re-export pattern. **Same resolution.**

### 4c. `sparql_library/parsers/turtle_star.rs:25` — unused variants

`ExpectPredicate`, `ExpectObject`, etc. enum variants in a parser state
machine. Some variants may be unreachable due to grammar simplification.
**Review the state machine transitions.**

---

## 5. Deprecated `Array::from_slice` — 6 warnings

**Files:** `identity/key_vault.rs` (2), `specialized_libs/cryptographic_library/mod.rs` (4)

The `aead` crate's `hybrid_array::Array::from_slice` is deprecated. Replace:

```rust
// Before:
Nonce::from_slice(&nonce_bytes)

// After:
let nonce: Nonce = nonce_bytes[..12].try_into().unwrap();
// or for variable-length:
Nonce::try_from(&nonce_bytes[..]).map_err(|_| "nonce length")?
```

**Note:** This is a real API migration, not just cosmetic. The deprecated
method may be removed in a future `aead` release, breaking the build.

---

## 6. Naming conventions — 4 warnings

**File:** `specialized_libs/cryptographic_library/mod.rs`

| Current | Fixed |
|---------|-------|
| `PCI_DSS` (line 380) | `PciDss` |
| `PCI_DSS` (line 393) | `PciDss` |
| `zkSNARKs` (line 1314) | `ZkSnarks` |
| `zkSTARKs` (line 1315) | `ZkStarks` |

These are enum variants. Since they're never constructed (see §1c), renaming
has zero downstream impact. **Rename.**

---

## 7. Trivial unused imports — 8 warnings

| File | Import | Action |
|------|--------|--------|
| `services/daemon.rs:3` | `SinkExt` | Remove |
| `services/webizen_server.rs:12` | `SinkExt` | Remove |
| `q42/p64_weight.rs:1092` | `std::io::Write` | Remove |
| `linear_algebra/optimization.rs:6` | `super::privacy` | Remove |
| `linear_algebra/optimization.rs:1` | `Serialize` | Remove |
| `linear_algebra/optimization.rs:1` | `Deserialize` | Remove |
| `linear_algebra/computation.rs:6` | `super::privacy` | Remove |
| `sparql_library/parsers/cbor_parser.rs:233` | `std::io::Read` | Remove |

---

## 8. Manifest warnings — 3 warnings

- `profiles for the non root package will be ignored` — move `[profile.*]`
  sections from `crates/qualia-core-db/Cargo.toml` to the workspace root
  `Cargo.toml`.
- `unused manifest key: bench.1.atoi` — remove the `atoi` key from the
  `[[bench]]` section.
- `unused manifest key: bench.1.csv` — remove the `csv` key from the
  `[[bench]]` section.

---

## Recommended priority

1. **Fix the deprecated `from_slice` calls** (§5) — these will break when
   `aead` removes the deprecated method.
2. **Remove the `*_str` fields from RDF-Star `ParseResult`** (§3b) — these
   allocate strings in the parsing hot path that are immediately discarded.
   This is a performance bug.
3. **Delete the dead warp-era helpers in `daemon.rs`** (§2a) — but preserve
   `proxy_target_allowed` / `ip_is_restricted` if a proxy endpoint is planned.
4. **Delete confirmed dead code** (§2b, §2c, §2f, §2g, §2h, §7, §8) —
   low-risk, high-clarity.
5. **Wire `rule_type` into N3 rule compilation** (§3a) — this is Task G from
   AGENTS.md and closes a real semantic gap.
6. **Implement the QPU job lifecycle** (§3c) — `Queued` → `Running` →
   `Completed` transitions with retry backoff.
7. **Address the scaffolding stubs** (§1) — the largest cluster. Decision
   needed: wire the native handlers to use these structs, or delete the
   scaffolding. Given the 42MB Sentinel constraint, deleting ~1500 lines of
   dead type definitions across 5 files is the lower-risk path.
8. **Fix glob re-export false positives** (§4) — add explicit imports or
   module-level `allow`.
