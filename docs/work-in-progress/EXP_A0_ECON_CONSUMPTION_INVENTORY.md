# EXP-A0 — Econ.* Poet consumption inventory

**Date:** 2026-09-06  
**Packet:** EXP-A0 (baseline) → **EXP-A1 Complete** (5 Live tools landed)

- `Econ.*` in host ids.rs string literals: **106**
- Cited anywhere under `crates/poet/**/*.rs`: **5** (post-A1 curated slice)
- Uncited in Poet (UI gap): **101**
- Poet cites `FinancialModeling.*`: `FinancialModeling.gbm_var`
- Poet cites `Finance.*`: _none_

## Poet citations (finance-ish)

- `Econ.black_scholes` — 3 file(s)
  - `crates/poet/src/browser/econ_chain_actions.rs`
  - `crates/poet/src/browser/registration/mod.rs`
  - `crates/poet/src/browser/registration/register_econ_toolbox.rs`
- `Econ.capm_expected_return` — 3 file(s)
  - `crates/poet/src/browser/econ_chain_actions.rs`
  - `crates/poet/src/browser/registration/mod.rs`
  - `crates/poet/src/browser/registration/register_econ_toolbox.rs`
- `Econ.gini` — 3 file(s)
  - `crates/poet/src/browser/econ_chain_actions.rs`
  - `crates/poet/src/browser/registration/mod.rs`
  - `crates/poet/src/browser/registration/register_econ_toolbox.rs`
- `Econ.mixed_nash_2x2` — 3 file(s)
  - `crates/poet/src/browser/econ_chain_actions.rs`
  - `crates/poet/src/browser/registration/mod.rs`
  - `crates/poet/src/browser/registration/register_econ_toolbox.rs`
- `Econ.solow_steady_state` — 3 file(s)
  - `crates/poet/src/browser/econ_chain_actions.rs`
  - `crates/poet/src/browser/registration/mod.rs`
  - `crates/poet/src/browser/registration/register_econ_toolbox.rs`
- `FinancialModeling.gbm_var` — 3 file(s)
  - `crates/poet/src/browser/logic_workbench/request_capabilities.rs`
  - `crates/poet/src/browser/logic_workbench/requests.rs`
  - `crates/poet/src/browser/specialist_persist/sessions.rs`

## Curated EXP-A1 slice (consume next)

| Capability | Why |
|---|---|
| `Econ.capm_expected_return` | Asset pricing entry |
| `Econ.gini` | Welfare / cooperative economics |
| `Econ.mixed_nash_2x2` | Game theory entry |
| `Econ.black_scholes` | Derivatives; pairs with FinancialModeling.black_scholes |
| `Econ.solow_steady_state` | Macro entry |

## Uncited `Econ.*` (first 40)

- `Econ.abatement_net_benefit`
- `Econ.agent_based_aggregate_wealth`
- `Econ.aggregate_paper_fills`
- `Econ.aggregate_wealth`
- `Econ.atkinson`
- `Econ.autocorrelation`
- `Econ.bellman_update`
- `Econ.bertrand_duopoly`
- `Econ.bertrand_with_demand`
- `Econ.binomial_option`
- `Econ.block_bootstrap`
- `Econ.capm_beta`
- `Econ.ccapm_equity_premium`
- `Econ.ccapm_sdf`
- `Econ.check_budget_balance`
- `Econ.check_ir`
- `Econ.cournot_duopoly`
- `Econ.covariance_matrix`
- `Econ.cross_correlation`
- `Econ.cumulative_wealth`
- `Econ.degree_centrality`
- `Econ.discount_factor`
- `Econ.distributional_npv`
- `Econ.drawdown`
- `Econ.efficiency_units`
- `Econ.eigenvector_centrality`
- `Econ.endowment_effect`
- `Econ.expected_holding_time`
- `Econ.fiscal_multiplier`
- `Econ.forward_rate`
- `Econ.gbm_simulate`
- `Econ.gordon_growth`
- `Econ.gravity_flow`
- `Econ.headcount_poverty`
- `Econ.historical_cvar`
- `Econ.historical_var`
- `Econ.household_production_ces`
- `Econ.hyperbolic_discount`
- `Econ.interbank_clearing`
- `Econ.interpolate_zero_rate`
- … +61 more
