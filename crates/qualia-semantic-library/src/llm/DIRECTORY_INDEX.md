---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# llm Index

## Functionality Overview
Comprehensive index of functionality for `llm`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
  - `trait LlmBackend`
  - `fn embed`
  - `fn generate`
  - `fn embed_dim`
  - `fn model_id`
  - `struct StoredChunk`
  - `fn embed_container`
  - `fn analyze_container`
  - `fn parse_tags`
  - `fn truncate_chars`
- 📄 `ollama.rs`
  - `struct OllamaConfig`
  - `impl Default`
  - `fn default`
  - `struct OllamaBackend`
  - `impl OllamaBackend`
  - `fn new`
  - `fn post_json`
  - `struct EmbeddingResp`
  - `struct GenerateResp`
  - `impl LlmBackend`
  - `fn embed`
  - `fn generate`
  - `fn embed_dim`
  - `fn model_id`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
