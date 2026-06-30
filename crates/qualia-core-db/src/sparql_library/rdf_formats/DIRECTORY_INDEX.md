---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# rdf_formats Index

## Functionality Overview
Comprehensive index of functionality for `rdf_formats`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `collector.rs`
  - `struct QuinCollector`
  - `impl QuinCollector`
  - `fn new`
  - `fn push`
  - `fn as_slice`
  - `impl QuinSink`
- 📄 `mod.rs`
  - `enum RdfFormat`
  - `impl RdfFormat`
  - `fn from_str`
  - `fn as_str`
  - `fn supports_quads`
- 📄 `parse.rs`
  - `enum RdfParseError`
  - `impl std`
  - `fn fmt`
  - `fn map_sink_err`
  - `fn parse_rdf`
  - `fn parse_ntriples_into_collector`
  - `fn plain_serialize_round_trip`
  - `fn rdf_format_from_str_aliases`
- 📄 `serialize.rs`
  - `enum RdfStarMode`
  - `enum RdfSerializeError`
  - `impl std`
  - `fn fmt`
  - `fn serialize_rdf`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
