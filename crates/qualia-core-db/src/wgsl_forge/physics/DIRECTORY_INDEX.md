---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# physics Index

## Functionality Overview
Comprehensive index of functionality for `physics`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `kinematics.rs`
  - `fn nbody_step_cpu`
  - `fn nbody_step_gpu`
  - `fn kinematics_wgsl_validates`
  - `fn nbody_oracle_like_charges_repel`
  - `fn nbody_oracle_opposite_charges_attract`
  - `fn nbody_gpu_matches_oracle`
- 📄 `mod.rs`
- 📄 `molecular_dynamics.rs`
  - `fn wrap`
  - `fn md_step_cpu`
  - `fn md_step_gpu`
  - `fn md_wgsl_validates`
  - `fn md_oracle_hand_checked`
  - `fn md_oracle_pbc_wraps`
  - `fn md_gpu_matches_oracle`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
