---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# sequential Index

## Functionality Overview
Comprehensive index of functionality for `sequential`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `hmm.rs`
  - `struct Hmm`
  - `impl Hmm`
  - `fn new`
  - `fn forward_scaled`
  - `fn log_likelihood`
  - `fn viterbi`
  - `struct Lcg`
  - `impl Lcg`
  - `fn unit`
  - `fn normalize`
  - `fn baum_welch`
  - `fn sticky_hmm`
  - `fn viterbi_recovers_obvious_path`
  - `fn log_likelihood_is_finite_and_orders_sequences`
  - `fn baum_welch_increases_likelihood_and_learns_structure`
  - *(...and 1 more)*
- 📄 `kalman.rs`
  - `struct KalmanFilter`
  - `fn no`
  - `fn no_t`
  - `impl KalmanFilter`
  - `fn new`
  - `fn state`
  - `fn covariance`
  - `fn predict`
  - `fn update`
  - `fn filter`
  - `fn tracks_a_constant_with_noisy_measurements`
  - `fn smooths_better_than_raw_measurements`
  - `fn guards`
- 📄 `mod.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
