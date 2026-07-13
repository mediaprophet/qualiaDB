---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# physical Index

## Functionality Overview
Comprehensive index of functionality for `physical`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
- 📄 `thermodynamics.rs`
  - `struct EnsembleState`
  - `struct ThermodynamicSampler`
  - `impl ThermodynamicSampler`
  - `fn new`
  - `fn metropolis_step`
  - `fn calculate_gibbs_free_energy`
  - `struct LithiumPack`
  - `impl LithiumPack`
  - `fn cell_ocv`
  - `fn pack_ocv`
  - `fn pack_resistance`
  - `fn terminal_voltage`
  - `fn deliverable_power`
  - `fn pack_capacity_ah`
  - `struct SolarPanel`
  - *(...and 10 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
