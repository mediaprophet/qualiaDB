---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# ingest Index

## Functionality Overview
Comprehensive index of functionality for `ingest`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `agent_intent.rs`
  - `fn ingest_agent_intent`
- 📄 `csv_mapper.rs`
  - `fn stream_csv_to_quins`
- 📄 `detect.rs`
  - `enum SemanticFormat`
  - `impl SemanticFormat`
  - `fn label`
  - `fn detect_format`
  - `fn tmp_file`
  - `fn detect_by_extension_nt`
  - `fn detect_by_extension_ttl`
  - `fn detect_by_extension_jsonld`
  - `fn detect_by_magic_q42`
  - `fn detect_by_magic_xml_rdf`
  - `fn detect_by_magic_kml`
  - `fn detect_by_magic_cbor_ld`
  - `fn detect_json_content_without_ld_extension`
  - `fn detect_unknown_returns_none`
  - `fn label_round_trips`
- 📄 `json_mapper.rs`
  - `fn stream_json_to_quins`
- 📄 `mapper.rs`
  - `enum TargetDatatype`
  - `struct ColumnMapping`
  - `struct MappingProfile`
  - `fn compile_shacl_mapping`
  - `fn parse_prefixes`
  - `fn extract_iri_value`
  - `fn extract_string_literal`
  - `fn find_matching_bracket`
  - `fn map_datatype`
  - `fn write_shacl`
  - `fn compile_basic_shacl`
  - `fn shacl_field_source_keys`
  - `fn shacl_datatypes_parsed_correctly`
  - `fn shacl_predicate_hashes_are_nonzero`
  - `fn shacl_error_on_missing_file`
  - *(...and 1 more)*
- 📄 `mod.rs`
  - `enum IngestError`
  - `impl From`
  - `fn from`
  - `impl std`
  - `fn fmt`
  - `struct IngestStats`
  - `fn ingest_ntriples`
  - `fn ingest_rdf_xml`
  - `fn parse_nt_line`
  - `fn ingest_chk`
  - `fn ingest_cbor`
  - `fn ingest_turtle_star`
  - `fn ingest_kml`
  - `fn ingest_asset`
  - `fn ingest_json_ld_star`
  - *(...and 1 more)*
- 📄 `pipeline.rs`
  - `struct RawUnsortedQuin`
  - `struct IncrementalIngestor`
  - `impl IncrementalIngestor`
  - `fn new`
  - `fn execute_stream_compilation`
  - `fn execute_stream_to_wal`
  - `fn build_external_merge_lexicon`
  - `struct IngestionCellWorkerPool`
  - `impl IngestionCellWorkerPool`
  - `fn execute_parallel_cell_resolution`
- 📄 `writer.rs`
  - `struct SuperBlockWriter`
  - `impl SuperBlockWriter`
  - `fn new`
  - `fn push`
  - `fn flush_block`
  - `impl Drop`
  - `fn drop`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
