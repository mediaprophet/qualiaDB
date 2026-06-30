---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# engine Index

## Functionality Overview
Comprehensive index of functionality for `engine`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `ingestion.rs`
  - `struct SemanticBookmark`
  - `struct IngestionResult`
  - `fn process_pdf`
  - `fn parse_csv`
  - `fn parse_json`
  - `fn serialize_to_csv_file`
  - `fn serialize_to_json_file`
  - `fn serialize_to_rdf_file`
  - `enum RdfFormat`
- 📄 `llm_offload.rs`
  - `struct ModelInfo`
  - `struct InferenceTelemetry`
  - `fn discover_local_models`
  - `enum VectorOp`
  - `enum WebizenOp`
  - `fn execute_agent_inference`
- 📄 `mod.rs`
- 📄 `pdf_processor.rs`
  - `fn ingest_pdf_to_library`
- 📄 `q42_compiler.rs`
  - `fn compile_to_q42`
- 📄 `semantic.rs`
  - `fn execute_local_sparql`
  - `fn validate_local_shacl`
  - `fn execute_slg_vm`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
