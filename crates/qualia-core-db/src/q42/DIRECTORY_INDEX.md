---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# q42 Index

## Functionality Overview
Comprehensive index of functionality for `q42`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `design_encode.rs`
  - `struct DesignPart`
  - `fn default_state`
  - `fn default_intensity`
  - `struct DesignRelation`
  - `struct SparqlContextHit`
  - `struct DesignDocument`
  - `fn default_design_type`
  - `fn default_design_version`
  - `enum DesignEncodeError`
  - `struct DesignEncodeStats`
  - `fn manifold_w`
  - `fn topology_v`
  - `fn epistemic_q`
  - `fn spectral_sigma`
  - `fn auto_position`
  - *(...and 8 more)*
- 📄 `mod.rs`
- 📄 `p64_weight.rs`
  - `fn crc32c_update`
  - `fn crc32c`
  - `struct P64WeightHeader`
  - `struct P64TensorEntry`
  - `struct P64HParams`
  - `fn align_up`
  - `impl P64WeightHeader`
  - `fn read_le`
  - `fn write_le`
  - `impl P64HParams`
  - `fn write_manifold_coordinate`
  - `fn write_tensor_entry`
  - `fn p64_tensor_name`
  - `fn compile_gguf_to_p64`
  - `fn compile_gguf_to_p64_legacy`
  - *(...and 49 more)*
- 📄 `q42_kvp.rs`
  - `struct Q42KvPageHeader`
  - `struct Q42ChunkPolicy`
  - `struct Q42QuerySketch`
  - `impl Q42KvPageHeader`
  - `fn new`
  - `impl Default`
  - `fn default`
  - `fn test_qkvp_alignment`
- 📄 `q42_lexicon.rs`
  - `enum CborLdError`
  - `struct SemanticPayload`
  - `struct Q42Context`
  - `impl Q42Context`
  - `fn new`
  - `fn from_volume`
  - `fn resolve_semantic_term`
  - `fn context_hash`
  - `fn expand_to_hash`
  - `impl Default`
  - `fn default`
  - `struct Q42CborLdParser`
  - `impl Q42CborLdParser`
  - `fn lexicon`
  - `fn parse`
  - *(...and 13 more)*
- 📄 `q42_reader.rs`
  - `fn read_c_q42_quins`
  - `fn read_q42_quins`
  - `fn write_test_c_q42`
  - `fn roundtrip_read_c_q42`
- 📄 `q42_volume.rs`
  - `struct Q42VolumeHeader`
  - `impl Q42VolumeHeader`
  - `fn verify_version`
  - `fn new_v3`
  - `struct BlockDirectoryEntry`
  - `impl BlockDirectoryEntry`
  - `fn write_to`
  - `fn from_bytes`
  - `fn migrate_v2_to_v3`
  - `fn is_unified_volume`
  - `fn encode_lex`
  - `fn encode_lex_with_entries`
  - `fn encode_bidx`
  - `fn encode_superblock`
  - `fn header_to_bytes`
  - *(...and 32 more)*
- 📄 `yaml_ld_q42.rs`
  - `enum YamlToken`
  - `struct YamlStreamingLexer`
  - `fn new`
  - `fn next_token`
  - `struct WebizenWorkspace`
  - `struct Page`
  - `struct Pane`
  - `fn compile_yaml_ld_to_quins`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
