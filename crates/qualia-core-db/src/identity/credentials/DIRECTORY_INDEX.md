---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# credentials Index

## Functionality Overview
Comprehensive index of functionality for `credentials`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `codecs.rs`
  - `enum CodecError`
  - `trait CredentialCodec`
  - `fn encode`
  - `fn decode`
  - `struct OpenBadgeCodec`
  - `impl CredentialCodec`
  - `struct PdfCodec`
  - `fn test_credential`
  - `fn openbadge_roundtrip`
  - `fn pdf_roundtrip`
- 📄 `mod.rs`
  - `enum VcError`
  - `struct Credential`
  - `struct Presentation`
  - `struct Proof`
  - `trait SelectiveDisclosure`
  - `fn generate_selective_presentation`
  - `fn verify_selective_presentation`
  - `trait ZkDisclosure`
  - `fn generate_zk_proof`
  - `fn verify_zk_proof`
  - `trait CredentialStatus`
  - `fn check_status`
  - `enum StatusResult`
  - `struct VcRuntime`
  - `impl VcRuntime`
  - *(...and 11 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
