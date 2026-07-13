---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# experiment Index

## Functionality Overview
Comprehensive index of functionality for `experiment`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `ab_test.rs`
  - `struct AbResult`
  - `fn ab_test`
  - `fn detects_a_real_lift`
  - `fn no_real_difference_is_not_significant`
  - `fn small_sample_is_underpowered`
  - `fn guards`
- 📄 `bandit.rs`
  - `enum Policy`
  - `struct Bandit`
  - `struct Lcg`
  - `impl Lcg`
  - `fn unit`
  - `fn below`
  - `fn gaussian`
  - `fn gamma`
  - `fn beta`
  - `impl Bandit`
  - `fn new`
  - `fn select`
  - `fn update`
  - `fn counts`
  - `fn values`
  - *(...and 10 more)*
- 📄 `mod.rs`
- 📄 `power.rs`
  - `fn required_sample_size_two_sample`
  - `fn power_two_sample`
  - `fn required_sample_size_two_proportion`
  - `fn sample_size_grows_as_effect_shrinks`
  - `fn power_and_sample_size_are_consistent`
  - `fn power_rises_with_sample_size`
  - `fn proportion_sample_size`
  - `fn guards`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
