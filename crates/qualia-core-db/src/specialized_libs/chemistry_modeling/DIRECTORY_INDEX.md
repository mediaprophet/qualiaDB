---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# chemistry_modeling Index

## Functionality Overview
Comprehensive index of functionality for `chemistry_modeling`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
  - `struct ChemistryModelingLibrary`
  - `struct MolecularSimulator`
  - `struct SimulationEngine`
  - `struct SimulationConfig`
  - `enum SimulationType`
  - `enum Ensemble`
  - `enum BoundaryType`
  - `struct TimeStepControl`
  - `enum TimeStepControlType`
  - `struct AdaptiveParameters`
  - `struct StabilityAnalysis`
  - `enum StabilityAnalysisMethod`
  - `struct EnergyConservation`
  - `struct TemperatureFluctuation`
  - `struct EnsembleManager`
  - *(...and 296 more)*
- 📄 `molecular_dynamics.rs`
  - `fn lj_params`
  - `fn mix`
  - `fn compute_lj_forces`
  - `fn kinetic_energy`
  - `fn temperature`
  - `struct Lcg`
  - `impl Lcg`
  - `fn next_unit`
  - `fn next_gaussian`
  - `fn init_velocities`
  - `fn run_md`
  - `fn atom`
  - `fn molecule`
  - `fn config`
  - `fn force_matches_finite_difference_gradient`
  - *(...and 4 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
