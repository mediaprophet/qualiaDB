---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# optimization Index

## Functionality Overview
Comprehensive index of functionality for `optimization`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `metaheuristics.rs`
  - `struct Rng`
  - `impl Rng`
  - `fn unit`
  - `fn below`
  - `fn gaussian`
  - `fn hill_climbing`
  - `fn simulated_annealing`
  - `fn artificial_bee_colony`
  - `fn abc_minimizes_the_sphere`
  - `fn sa_escapes_a_local_minimum`
  - `fn hill_climbing_reaches_a_discrete_optimum`
  - `fn guards`
- 📄 `mod.rs`
  - `struct NelderMeadSimplex`
  - `struct BoundedNewtonRaphson`
  - `struct LevenbergMarquardtStack`
  - `struct OptimizationState`
  - `struct RootFindingState`
  - `struct CurveFitState`
  - `trait ObjectiveFunction`
  - `fn evaluate`
  - `fn in_bounds`
  - `trait RootFunction`
  - `fn derivative`
  - `trait CurveFitFunction`
  - `fn jacobian`
  - `impl NelderMeadSimplex`
  - `fn new`
  - *(...and 34 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
