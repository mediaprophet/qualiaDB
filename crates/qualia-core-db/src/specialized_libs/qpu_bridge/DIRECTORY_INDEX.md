---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# qpu_bridge Index

## Functionality Overview
Comprehensive index of functionality for `qpu_bridge`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `circuit.rs`
  - `struct QuantumCircuitParams`
  - `enum QuantumCircuitType`
  - `struct QuantumCircuit`
  - `struct QuantumGate`
  - `enum QuantumGateType`
  - `impl QuantumCircuit`
  - `fn from_params`
  - `impl QuantumGate`
- 📄 `connection.rs`
  - `struct QPUBridgeManager`
  - `struct QPUConnectionState`
  - `enum QPUConnectionStatus`
  - `struct QPUAuthManager`
  - `struct QPUAPIConfig`
  - `enum QPUAuthState`
  - `impl QPUBridgeManager`
  - `fn new`
  - `fn initialize`
  - `fn connect`
  - `fn submit_job`
  - `fn get_job_result`
  - `fn submit_quantum_job`
  - `fn retrieve_quantum_result`
  - `fn submit_to_ibm_quantum`
  - *(...and 17 more)*
- 📄 `job.rs`
  - `struct QPUJobManager`
  - `struct QPUJob`
  - `enum QPUJobType`
  - `enum QPUJobPriority`
  - `enum QPUJobStatus`
  - `struct QPUJobCounters`
  - `enum QPUSubmissionState`
  - `struct QPURateLimiter`
  - `struct QPUJobSubmissionParams`
  - `struct QPUJobResult`
  - `impl QPUJobManager`
  - `fn allocate_job_slot`
  - `fn release_job_slot`
  - `fn add_active_job`
  - `fn find_active_job`
  - *(...and 7 more)*
- 📄 `metrics.rs`
  - `struct QPUMetrics`
  - `enum QPUErrorCode`
  - `impl QPUMetrics`
  - `enum QPUBridgeError`
- 📄 `misc.rs`
  - `impl From`
  - `fn from`
  - `impl core`
  - `fn fmt`
- 📄 `mod.rs`
- 📄 `tests.rs`
  - `struct ProblemDescription`
  - `enum QuantumProblemType`
  - `struct ProblemVariable`
  - `enum VariableDomain`
  - `struct ProblemConstraint`
  - `enum ConstraintType`
  - `struct ProblemObjective`
  - `enum ObjectiveType`
  - `struct QuboFormulation`
  - `impl QuboFormulation`
  - `fn new`
  - `fn add_linear`
  - `fn add_quadratic`
  - `struct CircuitFormulation`
  - `struct CircuitGate`
  - *(...and 24 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
