---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# quantum_biology Index

## Functionality Overview
Comprehensive index of functionality for `quantum_biology`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `context.rs`
  - `struct QuantumBiologyContext`
  - `impl Default`
  - `fn default`
- 📄 `entities.rs`
  - `struct BiologicalEntity`
  - `enum BiologicalEntityType`
  - `enum QuantumComputationType`
  - `impl Default`
  - `fn default`
- 📄 `gpu_pipeline.rs`
  - `struct QuantumGPUPipeline`
  - `enum GPUComputationState`
  - `struct GPUShaderParams`
- 📄 `mod.rs`
- 📄 `orchestrator.rs`
  - `struct QuantumBiologyOrchestrator`
  - `enum QuantumBiologyError`
  - `impl core`
  - `fn fmt`
  - `impl QuantumBiologyOrchestrator`
  - `fn new`
  - `fn register_entity`
  - `fn active_computations`
  - `impl Default`
  - `fn default`
- 📄 `qpu_bridge.rs`
  - `struct QPUBridge`
  - `enum QPUBridgeState`
  - `struct QPUJobParams`
- 📄 `quantum_state.rs`
  - `struct QuantumState`
- 📄 `results.rs`
  - `enum QuantumResultType`
  - `struct QuantumBiologyResult`
  - `impl Default`
  - `fn default`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
