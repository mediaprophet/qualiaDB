---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# qpu Index

## Functionality Overview
Comprehensive index of functionality for `qpu`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `dispatcher.rs`
  - `struct JobState`
  - `enum InternalStatus`
  - `fn now_ms`
  - `struct QueueStats`
  - `struct JobQueue`
  - `impl JobQueue`
  - `fn new`
  - `fn enqueue`
  - `fn process_queue`
  - `fn record_result`
  - `fn take_results`
  - `fn stats`
  - `impl Default`
  - `fn default`
  - `struct Dispatcher`
  - *(...and 10 more)*
- 📄 `mod.rs`
  - `enum ProblemType`
  - `struct JobParameters`
  - `impl Default`
  - `fn default`
  - `struct QpuJob`
  - `impl QpuJob`
  - `fn new`
  - `enum JobStatus`
  - `struct Measurement`
  - `struct JobResultData`
  - `struct QpuResult`
  - `impl QpuResult`
  - `fn failed`
  - `enum QpuError`
  - `impl std`
  - *(...and 2 more)*
- 📄 `pre_solver.rs`
  - `struct ProblemDescription`
  - `struct Variable`
  - `enum VariableDomain`
  - `struct Constraint`
  - `enum ConstraintType`
  - `struct Objective`
  - `enum ObjectiveType`
  - `struct QuboFormulation`
  - `impl QuboFormulation`
  - `fn new`
  - `fn add_linear_term`
  - `fn add_quadratic_term`
  - `fn to_job_parameters`
  - `struct CircuitFormulation`
  - `struct Gate`
  - *(...and 20 more)*

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
