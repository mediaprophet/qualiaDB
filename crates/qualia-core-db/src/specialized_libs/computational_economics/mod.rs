//! Native computational economics coordination layer.
//!
//! This module is deliberately small at the root. Concrete families live in
//! submodules so economics, finance, and statistics can grow without returning
//! to monolithic files.

pub mod accounting;
pub mod agent_based;
pub mod asset_pricing;
pub mod behavioral;
pub mod capabilities;
pub mod categorical;
pub mod derivatives;
pub mod dynamic_programming;
pub mod econometrics;
pub mod environmental_resource;
pub mod error;
pub mod fixed_income;
pub mod forensic_economics;
pub mod game_theory;
pub mod input_output;
pub mod labor_household;
pub mod macro_models;
pub mod market_data;
pub mod market_design;
pub mod markov;
pub mod mechanism;
pub mod network_economics;
pub mod ontology_bridge;
pub mod paper_trading;
pub mod portfolio;
pub mod public_finance;
pub mod risk;
pub mod spatial_economics;
pub mod time_series;
pub mod welfare;
pub mod yield_curve;

pub use accounting::{
    account_balances_into, trial_balance, validate_balanced_entry, validate_journal_entries,
    validate_journal_entry, Account, AccountBalance, AccountType, AccountingError, JournalEntry,
    Posting, TrialBalance,
};
pub use agent_based::{
    aggregate_wealth, clear_trades, match_orders_into, simulate_steps_into, zero_intelligence_step,
    Agent, AgentBasedError, AgentKind, OrderBook, Trade,
};
pub use asset_pricing::{
    capm_beta, capm_expected_return, ccapm_equity_premium, ccapm_stochastic_discount_factor,
    gordon_growth_price, lucas_asset_price, multi_period_ddm, AssetPricingError,
};
pub use behavioral::{
    endowment_effect_wta, hyperbolic_discount, present_biased_utility, probability_weight,
    prospect_value, reference_dependent_utility, BehavioralError,
};
pub use capabilities::{
    AllocationClass, CapabilityDomain, CapabilityRecord, CapabilityStatus, SafetyClass,
    COMPUTATIONAL_ECONOMICS_CAPABILITIES,
};
pub use categorical::{Compose, Identity, Morphism};
pub use derivatives::{
    binomial_option_price, black_scholes_price_and_greeks, normal_cdf, parity_implied_call_price,
    parity_implied_put_price, put_call_parity, BlackScholesResult, DerivativesError, OptionKind,
    OptionStyle, MAX_BINOMIAL_STEPS,
};
pub use dynamic_programming::{
    bellman_update, optimal_stopping_into, policy_iteration_into, value_iteration_into, DpError,
};
pub use econometrics::{
    gmm_moment_eval, iv_2sls_into, logistic_mle_into, ols_into, wls_into, CalibrationRecord,
    EconometricsError,
};
pub use environmental_resource::{
    abatement_net_benefit, marginal_damage, optimal_abatement, optimal_pollution, pollution_damage,
    social_cost_of_carbon, EnvironmentalError,
};
pub use error::{EconConvergence, EconError, EconSeriesView, EconStatus};
pub use fixed_income::{
    accrued_interest, clean_price_from_dirty, coupon_bond_cash_flows_into, coupon_bond_dv01,
    coupon_bond_metrics, coupon_bond_price, coupon_bond_price_from_cash_flows,
    coupon_bond_yield_to_maturity, dirty_price_from_clean, discount_factor,
    discount_factor_continuous, key_rate_duration, present_value, year_fraction, AccruedInterest,
    BondMetrics, CashFlow, DayCountConvention, FixedIncomeError,
};
pub use forensic_economics::{
    accumulate_harm_trace, compute_malfeasance_delta, compute_narrative_divergence,
    early_intervention_counterfactual_delta, epistemic_negligence_score,
    generate_synthetic_persona_trace, step_nquin_trajectory, AccumulatedHarm, EpistemicEdge,
    ForensicError, HealthWelfareState, MalfeasanceDelta, NarrativeDivergence, NquinVector,
    NQUIN_DIMS,
};
pub use game_theory::{
    bertrand_duopoly, cournot_duopoly, dominated_strategies_col_into,
    dominated_strategies_row_into, mixed_nash_2x2, pure_nash_equilibria_into, repeated_game_payoff,
    stackelberg_duopoly, GameTheoryError,
};
pub use input_output::{
    capacity_constrained_propagation, ghosh_inverse_into, key_sector_ranking_into,
    leontief_inverse_into, output_multipliers_from_inverse, shock_decomposition_into,
    InputOutputError,
};
pub use labor_household::{
    efficiency_units, household_production_ces, human_capital_accumulation_into,
    labor_supply_cobb_douglas, LaborHouseholdError,
};
pub use macro_models::{
    new_keynesian_solve, olg_steady_state, ramsey_euler_residual, ramsey_steady_state,
    rbc_simulate_into, solow_simulate_into, solow_steady_state, MacroError,
};
pub use market_data::{
    adjusted_close_into, adjustment_factors_into, close_vwap, log_returns_into,
    simple_returns_into, CorporateAction, CorporateActionKind, MarketBar, MarketDataError,
};
pub use market_design::{
    cara_utility, ces_utility, clear_market_linear, cobb_douglas_utility, crra_utility,
    deferred_acceptance_into, double_auction, is_stable_matching, leontief_utility,
    quasi_linear_utility, sealed_bid_first_price, uniform_price_auction, vickrey_auction,
    MarketDesignError,
};
pub use markov::{
    expected_holding_time, mean_first_passage_time_into, simulate_chain_into,
    stationary_distribution_into, transition_probability, validate_transition_matrix, MarkovError,
};
pub use mechanism::{
    check_budget_balance, check_individual_rationality, check_strategy_proofness_2x2,
    mechanism_report, vickrey_clarke_groves_payment_into, MechanismError, MechanismReport,
};
pub use network_economics::{
    default_cascade_into, degree_centrality_into, eigenvector_centrality_into,
    interbank_clearing_into, NetworkError,
};
pub use ontology_bridge::{
    encode_fibo_price, encode_scalar_result, encode_vector_result, validate_scalar_econ_constraint,
    FIBO_INSTRUMENT_PRICE,
};
pub use paper_trading::{
    aggregate_paper_fills, cancel_paper_order, simulate_fills_against_snapshots,
    submit_paper_order, Fill, MarketSnapshot, OrderType, PaperOrder, PaperTradingError, Side,
};
pub use portfolio::{
    covariance_matrix_into, max_drawdown, mean_return, portfolio_returns_into,
    portfolio_variance_from_covariance, sample_variance, volatility_risk_contributions_into,
    PortfolioError,
};
pub use public_finance::{
    fiscal_multiplier, laffer_curve_revenue, progressive_tax_into,
    survival_floor_allocation_into as pf_survival_floor, transfer_payment, PublicFinanceError,
    TaxBracket,
};
pub use risk::{
    gaussian_var, historical_cvar, historical_var, scenario_loss, scenario_losses_into,
    sorted_returns_into, RiskError,
};
pub use spatial_economics::{
    gravity_flow, gravity_flow_matrix_into, hotelling_extraction_into, morans_i,
    nearest_facility_into, total_transport_cost, transport_cost_matrix_into, SpatialError,
};
pub use time_series::{
    ar1_simulate_into, autocorrelation, block_bootstrap_mean_into, cross_correlation,
    cumulative_wealth_into, drawdown_into, gbm_simulate_into,
    historical_cvar as ts_historical_cvar, historical_var as ts_historical_var,
    log_returns_into as ts_log_returns_into, max_drawdown_from, ornstein_uhlenbeck_simulate_into,
    parametric_var, rolling_mean_into, rolling_variance_into,
    simple_returns_into as ts_simple_returns_into, TimeSeriesError,
};
pub use welfare::{
    atkinson_inequality, distributional_npv, gini_coefficient, headcount_poverty,
    lorenz_curve_into, nash_welfare, net_present_value, poverty_gap_ratio, rawlsian_welfare,
    survival_floor_allocation_into, utilitarian_welfare, WelfareError, WelfareReport,
};
pub use yield_curve::{
    annualized_forward_rate, bootstrap_zero_curve_from_par_yields, discount_factor_from_curve,
    interpolate_zero_rate, par_yield_from_zero_curve, CurvePoint, YieldCurveError,
};
