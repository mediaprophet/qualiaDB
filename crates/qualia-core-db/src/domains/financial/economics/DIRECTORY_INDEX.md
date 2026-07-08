---
created: 2026-07-07
updated: 2026-07-07
update_scope: Initial
---

# financial/economics Index

Purpose-defined financial economics kernels. `mod.rs` is a compatibility
re-export barrel; implementation lives in focused submodules.

## Files

- `mod.rs`
  - Re-exports the public API previously exposed from `economics.rs`.
- `stochastic.rs`
  - `simulate_gbm_path`
  - `simulate_gbm_path_seeded`
  - `simulate_gbm_steps_into`
  - `run_monte_carlo_var`
  - `run_monte_carlo_var_seeded_into`
  - `StochasticError`
  - `DEFAULT_MONTE_CARLO_SEED`
- `input_output.rs`
  - `MAX_SECTORS`
  - `propagate_supply_shock`
- `resilience.rs`
  - `resilience_resource_pricing`
- `macro_flows.rs`
  - `simulate_macroeconomic_flow`
- `node_pricing.rs`
  - `SystemContext`
  - `get_current_system_context`
  - `calculate_bandwidth_liability`

## Changelog

- **2026-07-07**: Split from the old single `economics.rs` file and added seeded
  caller-buffered stochastic kernels.
