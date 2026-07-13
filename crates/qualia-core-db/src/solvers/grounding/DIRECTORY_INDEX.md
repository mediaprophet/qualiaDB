---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# grounding Index

## Functionality Overview
Comprehensive index of functionality for `grounding`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `claim_support.rs`
  - `struct GroundingReport`
  - `fn role_overlap`
  - `fn component_support`
  - `fn entity_grounding`
  - `fn report`
  - `fn quin`
  - `fn exact_match_scores_full`
  - `fn two_role_match_clears_two_thirds`
  - `fn both_endpoints_cited_elsewhere_is_review_band`
  - `fn unrelated_evidence_scores_zero`
  - `fn empty_evidence_scores_zero`
  - `fn zero_hash_endpoints_do_not_ground`
- 📄 `evaluate.rs`
  - `fn evaluate_grounding`
  - `fn grounding_verdict`
  - `fn resolve_citations`
  - `fn evaluate_output_grounding`
  - `fn quin`
  - `struct MapResolver`
  - `impl GroundingResolver`
  - `fn resolve`
  - `fn exact_attestation_is_grounded`
  - `fn endpoints_only_is_weak_review_band`
  - `fn unrelated_is_ungrounded`
  - `fn resolver_path_grounds_a_cited_claim`
  - `fn unresolvable_citations_fail_closed`
- 📄 `mod.rs`
  - `struct GroundingThresholds`
  - `impl Default`
  - `fn default`
  - `enum GroundingVerdict`
  - `impl GroundingVerdict`
  - `fn score`
  - `fn is_grounded`
  - `trait GroundingResolver`
  - `fn resolve`
  - `fn verdict_accessors`
  - `fn default_thresholds_ordered`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
