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
- 📄 `aabb.rs`
  - `struct Aabb`
  - `impl Aabb`
  - `fn new`
  - `fn from_points`
  - `fn center`
  - `fn extent`
  - `fn volume`
  - `fn contains_point`
  - `fn contains`
  - `fn transformed`
  - `fn unit`
  - `fn extent_volume_center`
  - `fn identity_transform_is_noop`
  - `fn scale_shrinks_extent_about_centre`
  - `fn contains_and_from_points`
- 📄 `admission.rs`
  - `enum Refusal`
  - `struct Admission`
  - `impl Admission`
  - `fn new`
  - `fn admit`
  - `fn artefact`
  - `fn admits_a_valid_rigid_move`
  - `fn refuses_contraction_below_floor`
  - `fn rotation_is_not_contraction`
  - `fn refuses_out_of_world_bounds`
  - `fn verdict_is_deterministic`
- 📄 `joint.rs`
  - `enum JointKind`
  - `struct Joint`
  - `impl Joint`
  - `fn revolute`
  - `fn prismatic`
  - `fn motor_at`
  - `fn chain_motor_at`
  - `fn approx`
  - `fn identity_at_t_zero`
  - `fn revolute_rotates_over_t`
  - `fn prismatic_translates_over_t`
  - `fn motor_at_is_deterministic`
  - `fn chain_composes_two_joints`
- 📄 `material.rs`
  - `struct Material`
  - `impl Material`
  - `struct Body`
  - `impl Body`
  - `fn new`
  - `fn mass`
  - `fn momentum`
  - `fn kinetic_energy`
  - `fn mass_is_density_times_volume`
  - `fn momentum_and_energy`
- 📄 `mod.rs`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
