---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# graph_match Index

## Functionality Overview
Comprehensive index of functionality for `graph_match`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `approximate.rs`
  - `struct MatchResult`
  - `fn index`
  - `fn score`
  - `fn approximate_match`
  - `fn t`
  - `fn finds_an_embedded_pattern`
  - `fn partial_match_scores_lower_than_full`
  - `fn returns_a_degree_not_an_assertion`
  - `fn guards`
- 📄 `fuzzy_similarity.rs`
  - `struct FuzzyTriple`
  - `fn to_map`
  - `fn fuzzy_jaccard`
  - `fn fuzzy_dice`
  - `fn t`
  - `fn identical_graphs_are_one`
  - `fn disjoint_graphs_are_zero`
  - `fn partial_overlap_with_degrees`
  - `fn empty_graphs_are_similar`
- 📄 `mod.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
