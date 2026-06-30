---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# p2p Index

## Functionality Overview
Comprehensive index of functionality for `p2p`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
- 📄 `protocol.rs`
  - `struct NQuin`
  - `enum QualiaRequest`
  - `impl QualiaRequest`
  - `fn from_semantic_payload`
  - `enum QualiaResponse`
  - `impl QualiaResponse`
  - `fn key`
  - `fn kv`
  - `fn get`
  - `fn u64v`
  - `fn header`
  - `fn is_cbor_ld`
  - `fn encode`
  - `fn encode_request`
  - `fn decode_request`
  - *(...and 17 more)*
- 📄 `routing.rs`
  - `struct CivicsRoutingTable`
  - `impl CivicsRoutingTable`
  - `fn new`
  - `fn hydrate_from_db`
  - `fn add_trusted_group`
  - `fn is_authorized`
- 📄 `swarm.rs`
  - `struct QualiaBehaviour`
  - `fn build_behaviour`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
