---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# control_feedback Index

## Functionality Overview
Comprehensive index of functionality for `control_feedback`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `advanced.rs`
  - `fn adaptive_gains`
  - `fn mit_rule_adapt`
  - `fn scheduled_gain`
  - `fn mpc_control`
  - `fn mimo_step`
  - `fn mimo_output`
  - `fn close`
  - `fn adaptive_tuning_schedules_and_adapts_gains`
  - `fn mpc_drives_state_toward_setpoint`
  - `fn mimo_state_space_step_and_output`
- 📄 `mod.rs`
  - `fn enforce_linguistic_degradation`
  - `fn enforce_guided_referral`
  - `struct ControlState`
  - `impl ControlState`
  - `fn new`
  - `fn update`
  - `fn reset_integral`
  - `struct PidParameters`
  - `impl PidParameters`
  - `fn conservative_power_system`
  - `fn aggressive_response`
  - `struct FeedbackController`
  - `impl FeedbackController`
  - `fn compute_output`
  - `fn set_setpoint`
  - *(...and 17 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
