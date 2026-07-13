---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# survival Index

## Functionality Overview
Comprehensive index of functionality for `survival`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `cox.rs`
  - `struct CoxModel`
  - `fn fit`
  - `fn higher_covariate_increases_hazard`
  - `fn protective_covariate_is_negative`
  - `fn guards`
- 📄 `kaplan_meier.rs`
  - `struct KaplanMeier`
  - `impl KaplanMeier`
  - `fn fit`
  - `fn survival_at`
  - `fn median_survival`
  - `fn no_censoring_matches_empirical`
  - `fn censoring_keeps_survival_higher`
  - `fn tied_events_drop_together`
  - `fn guards`
- 📄 `mod.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
