---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# biological Index

## Functionality Overview
Comprehensive index of functionality for `biological`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `bioinformatics.rs`
  - `struct AlignmentScore`
  - `struct AlignmentResult`
  - `struct GapPenalty`
  - `impl Default`
  - `fn default`
  - `struct NucleotideMatrix`
  - `fn blosum62_idx`
  - `fn blosum62_score`
  - `fn smith_waterman`
  - `fn traceback_local`
  - `fn empty_result`
  - `fn needleman_wunsch`
  - `fn align_nucleotide`
  - `fn align_protein`
  - `fn align_sequences`
  - *(...and 32 more)*
- 📄 `mod.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
