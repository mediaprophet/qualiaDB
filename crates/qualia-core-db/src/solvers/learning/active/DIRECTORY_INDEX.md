---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# active Index

## Functionality Overview
Comprehensive index of functionality for `active`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `committee.rs`
  - `fn vote_entropy`
  - `fn consensus`
  - `fn consensus_entropy`
  - `fn average_kl_disagreement`
  - `fn rank_by_disagreement`
  - `fn vote_entropy_max_on_even_split`
  - `fn disagreeing_committee_scores_above_agreeing`
  - `fn consensus_is_the_mean_distribution`
  - `fn pool_ranking_puts_the_split_sample_first`
  - `fn fails_closed`
- 📄 `density.rs`
  - `fn cosine_similarity`
  - `fn representativeness`
  - `fn information_density`
  - `fn rank_by_density`
  - `fn cosine_basic`
  - `fn density_demotes_an_uncertain_outlier`
  - `fn beta_zero_is_plain_uncertainty`
  - `fn fails_closed_on_mismatch`
- 📄 `mod.rs`
  - `enum ActiveError`
  - `impl core`
  - `fn fmt`
  - `impl std`
  - `fn argsort_orders_high_to_low`
- 📄 `uncertainty.rs`
  - `enum Strategy`
  - `fn top_two`
  - `fn row_score`
  - `fn score`
  - `fn rank_informative`
  - `fn most_informative`
  - `fn least_confidence_picks_the_flat_distribution`
  - `fn margin_uses_top_two_gap`
  - `fn entropy_ranks_diffuse_highest`
  - `fn fails_closed_on_empty_and_ragged`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
