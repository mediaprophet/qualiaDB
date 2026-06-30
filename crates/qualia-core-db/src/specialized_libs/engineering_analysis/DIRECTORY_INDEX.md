---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# engineering_analysis Index

## Functionality Overview
Comprehensive index of functionality for `engineering_analysis`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `mod.rs`
  - `struct EngineeringAnalysisLibrary`
  - `struct StructuralAnalyzer`
  - `struct FiniteElementSolver`
  - `struct MeshGenerator`
  - `enum MeshType`
  - `struct MeshAlgorithm`
  - `enum MeshAlgorithmType`
  - `struct MeshAlgorithmParameters`
  - `struct QualityCriterion`
  - `struct MeshQuality`
  - `struct QualityMetric`
  - `enum MetricType`
  - `struct QualityAssessment`
  - `enum QualityGrade`
  - `struct ElementLibrary`
  - *(...and 316 more)*
- 📄 `thermal_conduction.rs`
  - `enum EndBc`
  - `fn thomas`
  - `fn solve_field`
  - `fn heat_flux`
  - `fn analyze_conduction`
  - `fn material`
  - `fn model`
  - `fn temp_bc`
  - `fn flux_bc`
  - `fn dirichlet_dirichlet_is_linear`
  - `fn uniform_generation_is_parabolic`
  - `fn dirichlet_neumann_matches_imposed_flux`
  - `fn refuses_without_material`
  - `fn refuses_nonpositive_conductivity`
  - `fn refuses_too_few_bcs`
  - *(...and 1 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
