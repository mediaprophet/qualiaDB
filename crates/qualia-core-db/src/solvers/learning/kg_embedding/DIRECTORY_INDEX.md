---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# kg_embedding Index

## Functionality Overview
Comprehensive index of functionality for `kg_embedding`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
  - `enum KgEmbeddingError`
  - `impl core`
  - `fn fmt`
  - `impl std`
  - `struct EmbeddingTable`
  - `impl EmbeddingTable`
  - `fn zeros`
  - `fn entity`
  - `fn relation`
  - `fn entity_mut`
  - `fn relation_mut`
  - `fn score`
  - `fn table_dims_and_accessors`
  - `fn rotate_table_relation_is_angles_only`
  - `fn zeros_fails_closed_on_degenerate`
- 📄 `predict.rs`
  - `enum RankFilter`
  - `fn rank_tail`
  - `fn mean_rank`
  - `fn mean_reciprocal_rank`
  - `fn hits_at_k`
  - `fn hand_table`
  - `fn true_tail_ranks_first`
  - `fn metrics_on_a_perfect_table`
  - `fn filtered_protocol_excludes_known_true`
  - `fn empty_test_set_fails_closed`
- 📄 `score.rs`
  - `enum ScoreModel`
  - `impl ScoreModel`
  - `fn dims`
  - `fn check`
  - `fn score`
  - `fn gradient`
  - `fn transe_distance`
  - `fn transe_score`
  - `fn transe_grad`
  - `fn distmult_score`
  - `fn distmult_grad`
  - `fn complex_score`
  - `fn complex_grad`
  - `fn rotate_score`
  - `fn rotate_grad`
  - *(...and 7 more)*
- 📄 `train.rs`
  - `struct TrainConfig`
  - `impl Default`
  - `fn default`
  - `fn sigmoid`
  - `fn train`
  - `fn apply`
  - `fn apply_reg`
  - `fn normalise`
  - `fn chain`
  - `fn mean_pos_minus_neg`
  - `fn transe_learns_to_rank_true_tail_first`
  - `fn distmult_separates_positives_from_negatives`
  - `fn complex_separates_positives`
  - `fn rotate_learns_a_chain`
  - `fn empty_corpus_fails_closed`
  - *(...and 1 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
