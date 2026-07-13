---
created: 2026-07-07
updated: 2026-07-07
update_scope: Initial
---

# computational_economics Index

Coordination layer for comprehensive computational economics, finance,
statistics, accounting, and safe interface work.

## Files

- `mod.rs`
  - Short re-export barrel.
- `accounting.rs`
  - `AccountType`
  - `Account`
  - `Posting`
  - `JournalEntry`
  - `AccountBalance`
  - `TrialBalance`
  - `AccountingError`
  - `validate_balanced_entry`
  - `validate_journal_entry`
  - `validate_journal_entries`
  - `account_balances_into`
  - `trial_balance`
- `capabilities.rs`
  - `CapabilityDomain`
  - `CapabilityStatus`
  - `AllocationClass`
  - `SafetyClass`
  - `CapabilityRecord`
  - `COMPUTATIONAL_ECONOMICS_CAPABILITIES`
  - `capabilities_by_domain`
- `categorical.rs`
  - `Morphism`
  - `Identity`
  - `Compose`
- `agent_based.rs`
  - Agent state, order matching, trade clearing, and deterministic agent-step helpers.
- `asset_pricing.rs`
  - CAPM/CCAPM, Gordon growth, dividend discount, and Lucas-style asset pricing helpers.
- `behavioral.rs`
  - Prospect-theory, probability weighting, hyperbolic discounting, and reference-dependent utility helpers.
- `derivatives.rs`
  - `OptionKind`
  - `OptionStyle`
  - `DerivativesError`
  - `BlackScholesResult`
  - `MAX_BINOMIAL_STEPS`
  - `normal_cdf`
  - `black_scholes_price_and_greeks`
  - `put_call_parity`
  - `parity_implied_call_price`
  - `parity_implied_put_price`
  - `binomial_option_price`
- `dynamic_programming.rs`
  - Value iteration, policy iteration, Bellman update, and optimal stopping helpers.
  - **Line-size debt:** 1357 lines; split into purpose files before substantial extension.
- `econometrics.rs`
  - OLS, WLS, IV/2SLS, logistic MLE starter, GMM moment evaluation, and calibration records.
- `environmental_resource.rs`
  - Pollution damage, abatement, optimal pollution, social cost of carbon, and resource helpers.
- `error.rs`
  - Shared computational-economics error/status/view types.
- `fixed_income.rs`
  - `DayCountConvention`
  - `FixedIncomeError`
  - `CashFlow`
  - `BondMetrics`
  - `AccruedInterest`
  - `year_fraction`
  - `discount_factor`
  - `discount_factor_continuous`
  - `present_value`
  - `coupon_bond_cash_flows_into`
  - `coupon_bond_price`
  - `coupon_bond_price_from_cash_flows`
  - `coupon_bond_metrics`
  - `coupon_bond_yield_to_maturity`
  - `accrued_interest`
  - `dirty_price_from_clean`
  - `clean_price_from_dirty`
  - `coupon_bond_dv01`
  - `key_rate_duration`
- `game_theory.rs`
  - Pure/mixed Nash, dominance checks, Cournot/Bertrand/Stackelberg, and repeated-game payoff helpers.
- `input_output.rs`
  - Leontief/Ghosh inverse, multipliers, shock propagation, capacity constraints, and sector ranking.
- `labor_household.rs`
  - Labor supply, human capital, household production, and efficiency-unit helpers.
- `macro_models.rs`
  - Solow, Ramsey, OLG, RBC, and New Keynesian starter helpers.
- `market_data.rs`
  - `MarketBar`
  - `CorporateActionKind`
  - `CorporateAction`
  - `MarketDataError`
  - `adjustment_factors_into`
  - `adjusted_close_into`
  - `simple_returns_into`
  - `log_returns_into`
  - `close_vwap`
- `market_design.rs`
  - Utility forms, matching, auctions, market clearing, and double-auction helpers.
- `mechanism.rs`
  - VCG payments, budget balance, individual rationality, and strategy-proofness checks.
- `network_economics.rs`
  - Centrality, interbank clearing, and default cascade helpers.
- `portfolio.rs`
  - `PortfolioError`
  - `portfolio_returns_into`
  - `mean_return`
  - `sample_variance`
  - `covariance_matrix_into`
  - `portfolio_variance_from_covariance`
  - `volatility_risk_contributions_into`
  - `max_drawdown`
- `public_finance.rs`
  - Progressive tax, transfers, fiscal multiplier, Laffer revenue, and survival-floor allocation.
- `risk.rs`
  - `RiskError`
  - `sorted_returns_into`
  - `historical_var`
  - `historical_cvar`
  - `gaussian_var`
  - `scenario_loss`
  - `scenario_losses_into`
- `spatial_economics.rs`
  - Gravity flow, transport costs, Moran's I, nearest facility, and Hotelling extraction helpers.
- `time_series.rs`
  - Returns, rolling statistics, autocorrelation/cross-correlation, AR(1), GBM, OU, VaR/CVaR, and bootstrap helpers.
  - **Line-size debt:** 925 lines; split before substantial extension.
- `welfare.rs`
  - Welfare functions, inequality/poverty metrics, NPV, distributional NPV, and survival-floor allocation.
  - **Line-size debt:** 991 lines; split before substantial extension.
- `yield_curve.rs`
  - `CurvePoint`
  - `YieldCurveError`
  - `interpolate_zero_rate`
  - `discount_factor_from_curve`
  - `annualized_forward_rate`
  - `par_yield_from_zero_curve`
  - `bootstrap_zero_curve_from_par_yields`

## Changelog

- **2026-07-08**: Updated index to reflect the expanded economics modules and
  recorded file-size debt for modules now over the preferred 800-line target.
- **2026-07-07**: Integrated sub-agent accounting and derivatives lanes:
  double-entry posting validation, account balances, journal-entry validation,
  trial balance, Black-Scholes-Merton price/Greeks, put-call parity, and CRR
  binomial option pricing.
- **2026-07-07**: Added basic risk metrics over supplied data: sorted return
  scratch, historical VaR/CVaR, Gaussian VaR, and scenario losses.
- **2026-07-07**: Added portfolio analytics basics over supplied return
  matrices: weighted returns, mean, sample variance, sample covariance,
  portfolio variance, volatility risk contributions, and max drawdown.
- **2026-07-07**: Added deterministic market-data basics: supplied-bar
  corporate-action adjustment factors, adjusted closes, simple/log returns,
  and close VWAP with provenance refusal for corporate actions.
- **2026-07-07**: Added yield-curve basics: zero-rate interpolation, curve
  discount factors, annualized forwards, par yields, and par-yield
  bootstrapping into caller-owned output buffers.
- **2026-07-07**: Extended fixed-income basics with caller-buffered cash-flow
  schedule generation, clean/dirty price conversion, accrued interest, DV01,
  and key-rate duration.
- **2026-07-07**: Added fixed-income basics: day-count fractions, discount
  factors, cash-flow present value, coupon-bond price/yield/duration/convexity.
- **2026-07-07**: Added first capability/status matrix and categorical
  composition helpers.
