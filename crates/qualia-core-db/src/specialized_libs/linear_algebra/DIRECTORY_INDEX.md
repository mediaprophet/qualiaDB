---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# linear_algebra Index

## Functionality Overview
Comprehensive index of functionality for `linear_algebra`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `computation.rs`
  - `struct ComputationEngine`
  - `enum MatrixOperation`
  - `enum DecompositionType`
  - `struct OperationScheduler`
  - `struct ExecutionEngine`
  - `enum ExecutionEngineType`
  - `struct ComputationUnit`
  - `enum ComputationUnitType`
  - `struct ComputationCapabilities`
  - `struct PerformanceMetrics`
  - `struct ParallelExecutor`
  - `struct WorkerThread`
  - `struct MatrixTask`
  - `enum TaskPriority`
  - `struct ThreadPerformance`
  - *(...and 19 more)*
- 📄 `core_types.rs`
  - `struct MatrixMetadata`
  - `enum DataType`
  - `enum StorageFormat`
  - `enum CompressionType`
  - `struct Matrix`
  - `struct LinearAlgebraResult`
  - `enum LinearAlgebraError`
  - `impl std`
  - `fn fmt`
- 📄 `optimization.rs`
  - `struct OptimizationEngine`
  - `struct MatrixOptimizer`
  - `enum OptimizationStrategy`
  - `struct OptimizationRecord`
  - `struct MatrixAnalyzer`
  - `enum AnalysisAlgorithm`
  - `struct PatternRecognition`
  - `enum MatrixPattern`
  - `struct PatternLibrary`
  - `struct OptimizationHint`
  - `struct MatrixTransformer`
  - `enum TransformationRule`
  - `struct TransformationRecord`
  - `impl OptimizationEngine`
  - `fn new`
  - *(...and 11 more)*
- 📄 `performance.rs`
  - `struct LAPerformanceMonitor`
  - `struct OperationMetrics`
  - `struct MatrixMetrics`
  - `struct SystemMetrics`
  - `impl LAPerformanceMonitor`
  - `fn new`
  - `fn record_operation`
  - `fn get_system_metrics`
- 📁 `privacy/`
  - `mod.rs` — privacy facade, fixed-capacity key metadata, secure-aggregation capabilities
  - `bfv.rs` + `bfv/tests.rs` — feature-gated exact packed BFV encryption/add/multiply/dot product, 48-byte ciphertext references, and focused tests
  - `differential_privacy.rs` + `differential_privacy/tests.rs` — caller-buffered Laplace/Gaussian releases, basic/advanced/RDP accounting, and focused tests
  - `struct PrivacyEngine`
  - `struct HomomorphicOperations`
  - `enum HomomorphicOperation`
  - `struct HomomorphicKeyManager`
  - `struct HomomorphicKey`
  - `enum HomomorphicKeyType`
  - `struct KeyRotationPolicy`
  - `struct SecureAggregation`
  - `enum AggregationProtocol`
  - `struct PrivacyBudget`
  - `struct DifferentialPrivacy`
  - `enum NoiseMechanism`
  - `struct PrivacyAccountant`
  - `enum CompositionMethod`
  - `impl PrivacyEngine`
  - `struct BfvEngine`
  - `struct HeCiphertextRef`
  - `fn encode_fixed_point_into`
  - `fn decode_fixed_point_into`
- 📄 `storage.rs`
  - `struct MatrixStorage`
  - `struct MatrixZone`
  - `enum ZoneType`
  - `enum AccessPattern`
  - `struct MatrixAllocator`
  - `enum AllocationStrategy`
  - `struct MemoryBlock`
  - `struct MatrixCache`
  - `struct CacheEntry`
  - `enum CachePolicy`
  - `struct StorageBackend`
  - `enum BackendType`
  - `impl MatrixStorage`
  - `fn new`
  - `fn initialize`
  - *(...and 13 more)*

## Changelog
- **2026-07-01**: Replaced the privacy metadata stub with real BFV homomorphic
  arithmetic and calibrated differential privacy; split the module by concern.
- **2026-06-30**: Automated full index generation, extracting code definitions.
