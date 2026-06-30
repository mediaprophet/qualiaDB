---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# solvers Index

## Functionality Overview
Comprehensive index of functionality for `solvers`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Subdirectories
- 📁 `[calculus](calculus/DIRECTORY_INDEX.md)`
- 📁 `[exact](exact/DIRECTORY_INDEX.md)`
- 📁 `[fuzzy_query](fuzzy_query/DIRECTORY_INDEX.md)`
- 📁 `[geometric_algebra](geometric_algebra/DIRECTORY_INDEX.md)`
- 📁 `[graph_match](graph_match/DIRECTORY_INDEX.md)`
- 📁 `[graph_opt](graph_opt/DIRECTORY_INDEX.md)`
- 📁 `[grounding](grounding/DIRECTORY_INDEX.md)`
- 📁 `[interpolation](interpolation/DIRECTORY_INDEX.md)`
- 📁 `[learning](learning/DIRECTORY_INDEX.md)`
- 📁 `[linear_algebra](linear_algebra/DIRECTORY_INDEX.md)`
- 📁 `[number_theory](number_theory/DIRECTORY_INDEX.md)`
- 📁 `[ontology_align](ontology_align/DIRECTORY_INDEX.md)`
- 📁 `[optimization](optimization/DIRECTORY_INDEX.md)`
- 📁 `[qpu](qpu/DIRECTORY_INDEX.md)`
- 📁 `[quantum_optimizers](quantum_optimizers/DIRECTORY_INDEX.md)`
- 📁 `[shared](shared/DIRECTORY_INDEX.md)`
- 📁 `[special_functions](special_functions/DIRECTORY_INDEX.md)`
- 📁 `[statistics](statistics/DIRECTORY_INDEX.md)`
- 📁 `[symbolic_logic](symbolic_logic/DIRECTORY_INDEX.md)`
- 📁 `[transforms](transforms/DIRECTORY_INDEX.md)`
- 📁 `[units](units/DIRECTORY_INDEX.md)`
- 📁 `[vector_calculus](vector_calculus/DIRECTORY_INDEX.md)`

### Files & Exported Functionality
- 📄 `activation.rs`
  - `fn relu`
  - `fn sigmoid`
  - `fn tanh`
  - `fn silu`
  - `fn gelu`
  - `fn softmax`
  - `fn rms_norm`
  - `fn layer_norm`
  - `fn approx`
  - `fn relu_clamps_negatives`
  - `fn sigmoid_known_values`
  - `fn silu_equals_x_times_sigmoid`
  - `fn gelu_zero_and_monotone`
  - `fn softmax_sums_to_one_and_orders`
  - `fn softmax_is_shift_invariant_and_stable`
  - *(...and 2 more)*
- 📄 `attention.rs`
  - `fn scaled_dot_product_attention`
  - `fn approx`
  - `fn single_key_returns_that_value`
  - `fn uniform_scores_average_the_values`
  - `fn matches_hand_computed_softmax_qkt_v`
  - `fn causal_mask_blocks_future_keys`
  - `fn rejects_bad_dims`
- 📄 `feed_forward.rs`
  - `fn swiglu_ffn`
  - `fn silu_scalar`
  - `fn matches_hand_computed_swiglu`
  - `fn zero_input_gives_zero_output`
  - `fn rejects_bad_dims`
- 📄 `mod.rs`
  - `enum SolversError`
  - `struct SolverConfig`
  - `impl Default`
  - `fn default`
  - `struct SolverState`
  - `impl SolverState`
  - `fn cost_value`
  - `fn set_cost_value`
  - `fn satisfiable`
  - `fn set_satisfiable`
  - `fn quantum_calls`
  - `fn set_quantum_calls`
  - `fn add_quantum_calls`
- 📄 `polynomial.rs`
  - `struct Complex`
  - `impl Complex`
  - `fn add`
  - `fn sub`
  - `fn mul`
  - `fn div`
  - `fn abs`
  - `fn is_real`
  - `enum QuadraticRoots`
  - `fn solve_quadratic`
  - `fn poly_eval_complex`
  - `fn polynomial_roots`
  - `fn quadratic_two_real`
  - `fn quadratic_double_and_complex_and_linear`
  - `fn quadratic_rejects_degenerate`
  - *(...and 3 more)*
- 📄 `rope.rs`
  - `fn rope_interleaved`
  - `fn position_zero_is_identity`
  - `fn rotation_preserves_pair_norm`
  - `fn quarter_turn_known_rotation`
  - `fn per_head_blocks_independent`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
