---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# lora Index

## Functionality Overview
Comprehensive index of functionality for `lora`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `adapter_manager.rs`
  - `enum LoRAError`
  - `impl std`
  - `fn fmt`
  - `struct LoRATensor`
  - `impl LoRATensor`
  - `fn new`
  - `fn matvec_add`
  - `struct LoRAMetadata`
  - `impl LoRAMetadata`
  - `fn scaling`
  - `struct LoRAAdapter`
  - `impl LoRAAdapter`
  - `fn apply_cpu`
  - `fn compute_delta`
  - `struct RawHeader`
  - *(...and 35 more)*
- 📄 `context_detector.rs`
  - `enum ContextType`
  - `impl ContextType`
  - `fn from_metadata_bits`
  - `fn to_metadata_bits`
  - `fn adapter_filename`
  - `fn all`
  - `impl std`
  - `fn fmt`
  - `struct NGramAnalyzer`
  - `impl NGramAnalyzer`
  - `fn new`
  - `fn score`
  - `struct ContextDetector`
  - `impl ContextDetector`
  - `fn analyze_text`
  - *(...and 10 more)*
- 📄 `mod.rs`
- 📄 `webgpu_lora.rs`
  - `struct LoraGpuParams`
  - `struct LoRAGpuApplicator`
  - `impl LoRAGpuApplicator`
  - `fn new`
  - `fn new_async`
  - `fn apply`
  - `fn buf_rw`
  - `fn buf_r`
  - `fn buf_uniform`
  - `fn storage_rw_entry`
  - `fn storage_r_entry`
  - `fn uniform_entry`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
