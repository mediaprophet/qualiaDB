---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# serialisers Index

## Functionality Overview
Comprehensive index of functionality for `serialisers`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `csv_serializer.rs`
  - `struct CsvSerializationProfile`
  - `enum CsvDatatype`
  - `fn serialize_quins_to_csv`
  - `fn format_quin_value`
- 📄 `json_serializer.rs`
  - `struct JsonSerializationProfile`
  - `enum JsonDatatype`
  - `fn serialize_quins_to_json`
  - `fn format_quin_value`
- 📄 `mod.rs`
- 📄 `rdf_dispatch.rs`
  - `enum RdfDispatchError`
  - `fn serialize_quins`
  - `fn serialize_plain`
  - `fn serialize_star`
  - `fn write_with_serializer`
- 📄 `rdf_serializers.rs`
  - `fn serialize_to_ntriples`
  - `fn serialize_to_turtle`
  - `fn serialize_to_nquads`
  - `fn serialize_to_trig`
  - `fn serialize_to_n3`
  - `fn serialize_to_jsonld`
  - `fn format_hash`
- 📄 `sparql_results.rs`
  - `struct ResultFormatter`
  - `impl ResultFormatter`
  - `fn format_value_xml`
  - `fn format_value_json`
  - `fn format_value_tsv`
  - `fn format_xml`
  - `fn format_json`
  - `fn format_tsv`
  - `fn format_csv`
  - `fn format_ntriples`
  - `fn format_ask_xml`
  - `fn format_ask_json`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
