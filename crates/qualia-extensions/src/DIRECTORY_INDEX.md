---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# src Index

## Functionality Overview
Comprehensive index of functionality for `src`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[bin](bin/DIRECTORY_INDEX.md)`
- 📁 `[webgpu_extension](webgpu_extension/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `lib.rs`
  - `struct ExtensionCapability`
  - `struct ResourceRequirements`
  - `struct ExtensionJob`
  - `struct ExtensionResult`
  - `struct NQuin`
  - `struct ExtensionManager`
  - `impl ExtensionManager`
  - `fn new`
  - `fn register_extension`
  - `fn execute_job`
  - `fn list_capabilities`
  - `trait Extension`
  - `fn capability`
  - `fn execute`
  - `fn shutdown`
  - *(...and 16 more)*
- 📄 `pinn_extension.rs`
  - `struct PinnExtension`
  - `struct NativePinnBackend`
  - `struct TernaryPinnModelManager`
  - `struct TernaryQuantizationConfig`
  - `struct SmxFormatter`
  - `struct SmxMetadataSchema`
  - `struct TrainingMetadata`
  - `struct TernaryPinnModel`
  - `enum PhysicsDomain`
  - `struct BoundaryCondition`
  - `enum BoundaryType`
  - `struct PhysicsConstraint`
  - `enum EquationType`
  - `struct PinnJobParams`
  - `struct TernaryTensor`
  - *(...and 49 more)*
- 📄 `qpu_extension.rs`
  - `struct QpuExtension`
  - `struct QpuApiClient`
  - `struct QpuProvider`
  - `enum PricingModel`
  - `struct QuantumCircuit`
  - `struct QuantumGate`
  - `struct QuantumMeasurement`
  - `struct QpuJobParams`
  - `struct QpuExecutionResult`
  - `impl QpuExtension`
  - `fn new`
  - `fn execute_circuit`
  - `fn validate_circuit`
  - `fn send_to_provider`
  - `fn execute_ibm_quantum`
  - *(...and 10 more)*
- 📄 `snn_extension.rs`
  - `struct SnnExtension`
  - `struct SnnNetworkManager`
  - `struct NoisyGradientCrdt`
  - `struct SpikingNetwork`
  - `enum NetworkType`
  - `struct SpikingNeuron`
  - `enum NeuronType`
  - `struct TemporalState`
  - `struct Synapse`
  - `enum PlasticityType`
  - `struct CrdtWeight`
  - `enum ConflictResolution`
  - `struct TemporalConfig`
  - `enum SpikeEncoding`
  - `struct CrdtConfig`
  - *(...and 61 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
