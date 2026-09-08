# Q1/Q2 incorporation wave 2 swarm — 2026-09-06

**Branch:** `0.0.36-dev`  
**Prior wave:** `POET_Q1Q2_WAVE_SWARM_2026-09-06.md` (Complete)  
**Methodology:** `docs/work-in-progress/VIBE_INCORPORATION_METHODOLOGY_2026-09-06.md`  
**Constraint:** `docs/work-in-progress/VIBE_HOST_CONSTRAINT_CORRECTION_2026-09-06.md`  
**Pipeline:** crate → **Host `Family.method`** → **Poet Live**  
**Workspace:** `C:\github\qualiaDB` · no worktrees · not committed unless owner asks  
**Backlog baseline:** `ALL_BOUND=904` · `Q2=818` · `Q1=12308` · `PoetLive=86`

## Lanes (disjoint)

| Lane | Packet | Owns (write) | Must not touch |
|------|--------|--------------|----------------|
| A | `Q2-ECON` | `econ_chain_actions.rs`, `register_econ_toolbox.rs`; **append-only** econ sections in `tool_actions` / `tool_copy` / `registration/mod` asserts | sheet, ai, scientific, Host ids |
| B | `Q2-STATS` | `stats_chain_actions.rs`, `register_sheet_toolbox.rs`; append-only sheet/stats sections in shared wiring | econ, ai, Host ids |
| C | `Q2-ML` | `ml_chain_actions.rs`, `register_ai_toolbox.rs`; append-only ai/ml sections in shared wiring | econ, sheet, Host ids |
| D | `Q1-HOST` | New Host binds (4–8) for real specialized_libs `pub fn`s still Host-missing — prefer remaining `LinearAlgebra` vector/solver helpers, `symbolic_trig` / `polynomial_algebra`, or unbound special-fn style pure math. Pair `poet_host/invoke/ids.rs` + `vibe/catalog/ids.rs` + invoke submodule + tests. **No Poet Live** this wave (avoid A–C collisions). | poet toolbox / chain_actions files |

## Targets (≥8 Live each for A–C)

Prefer **pure numeric** Host ids that admit honest local sketches.

- **Econ examples (verify still Q2):** `capm_beta`, `autocorrelation`, `cross_correlation`, `bertrand_with_demand`, `check_budget_balance`, `ccapm_equity_premium`, `cumulative_wealth`, `aggregate_wealth`, …
- **Stats examples:** `argmax`, `binomial_pmf`/`binomial_cdf`, `beta_pdf`, `chi_squared_*`, `autocorrelation`, `bootstrap_means`, …
- **ML examples (prefer non-`al_*`):** `ab_test`, `holm`, `benjamini_hochberg`, `bootstrap_ci`, `bootstrap_estimate`, …

## Acceptance

- Dual-path honesty; `capability_scope` = exact Host id
- Lane tests green + `product_integrity` if Poet registration changed
- Lane D: `vibe_catalog_contains_every_bound_invoke_id` + module tests green
- Curated slice only — do not clear backlog

## Parent integrate

Verify all lanes → refresh `vibe_incorporation_backlog.py` → update register + ledger.

## Lane reports (parent)

| Lane | Status | Agent | Delivered |
|------|--------|-------|-----------|
| A `Q2-ECON` | **done** | [Wave2 Q2-Econ Live](e9200a4b-1ba5-4816-99e6-69b231cbf96a) | +8 Live: capm_beta, autocorrelation, cross_correlation, bertrand_with_demand, check_budget_balance, ccapm_equity_premium, mean_return, poverty_gap |
| B `Q2-STATS` | **done** | [Wave2 Q2-Stats Live](606c3ee3-994e-4697-84cf-09c4fcc93ccf) | +9 Live: argmax, binomial_pmf/cdf, beta_pdf, chi_squared_pdf/cdf/quantile, autocorrelation, bootstrap_means |
| C `Q2-ML` | **done** | [Wave2 Q2-ML Live](1e8f04cb-0735-4102-820e-82c8bca4ed82) | +8 Live: holm, BH, ab_test, bootstrap_estimate/ci, permutation_test, n_rejected, power_two_sample |
| D `Q1-HOST` | **done** | [Wave2 Q1-Host binds](13472316-d299-4bd0-b767-ef8a3d0bb661) | +8 Host: LinAlg add_assign/hadamard_assign/scale; SymbolicAlgebra.simplify_trig; PolynomialAlgebra ×4 |

**Wave status:** integrated (parent-verified). Not committed.

**Parent verify:**
- poet econ **34**; stats **21**; ai_ml **2**; product_integrity **11**
- core-db catalog **1**; poly_algebra **5**; add_assign_hadamard_assign_scale **1**; simplify_trig **2**

**Backlog refresh:** `ALL_BOUND=912` · `pub_fns=19392` · `Q1=12292` · `Q2=801` · `PoetLive=111` (was 904 / 86 after wave 1).
