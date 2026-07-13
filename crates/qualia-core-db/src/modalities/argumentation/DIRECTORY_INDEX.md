---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# argumentation Index

## Functionality Overview
Comprehensive index of functionality for `argumentation`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `bipolar.rs`
  - `struct BipolarFramework`
  - `impl BipolarFramework`
  - `fn new`
  - `fn add_support`
  - `fn support_reaches`
  - `fn to_dung`
  - `fn grounded_extension`
  - `fn arg`
  - `fn support_induces_a_supported_attack`
  - `fn support_paths_are_transitive`
- 📄 `generation.rs`
  - `fn framework_from_trace`
  - `fn complementary_verdicts_become_a_debate`
  - `fn agreeing_verdicts_do_not_attack`
- 📄 `mod.rs`
  - `struct Argument`
  - `impl Argument`
  - `fn new`
  - `fn with_strength`
  - `struct Attack`
  - `enum AttackType`
  - `struct ArgumentationFramework`
  - `impl ArgumentationFramework`
  - `fn add_argument`
  - `fn add_attack`
  - `fn get_attackers`
  - `fn get_attacked`
  - `fn grounded_extension`
  - `fn preferred_extensions`
  - `fn is_conflict_free`
  - *(...and 22 more)*
- 📄 `vaf.rs`
  - `struct ValueArgumentationFramework`
  - `impl ValueArgumentationFramework`
  - `fn new`
  - `fn set_value`
  - `fn set_rank`
  - `fn rank_of_arg`
  - `fn defeats`
  - `fn defeat_framework`
  - `fn grounded_extension`
  - `fn preferred_extensions`
  - `fn arg`
  - `fn atk`
  - `fn higher_value_argument_survives_a_mutual_attack`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
