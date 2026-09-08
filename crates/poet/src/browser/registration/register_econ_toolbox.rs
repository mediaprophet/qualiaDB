//! Part of poet browser toolbox registration — curated Econ.* Live tools.

use super::*;

pub(super) fn register_econ_toolbox(reg: &mut Registry) {
    reg.register_toolbox(Toolbox::new(
        ToolboxMetadata {
            id: "econ".into(),
            label: "Economics & Markets".into(),
            icon: "finance".into(),
            ontology_prefix: "econ".into(),
            description: "Live Econ.* kernels on numbers you enter. Offline shows local sketches only."
                .into(),
            enabled_by_default: true,
            family: "lab".into(),
        },
        vec![ToolChain::new(
            ToolChainMetadata {
                id: "econ:live".into(),
                label: "Live computational economics".into(),
                icon: "finance".into(),
                description: "Curated Econ.* binds — CAPM, Gini, Nash, Black–Scholes, binomial, Solow, Cournot, Bertrand, VaR, Atkinson, Gordon, forward, GBM, poverty, hyperbolic, fiscal, drawdown, covariance, beta, autocorrelation, cross-correlation, budget, CCAPM, mean return, poverty gap, sample variance, welfare, Stackelberg, put-call parity, parametric VaR, Laffer, CVaR, endowment, prospect, Prelec weight, SDF, gravity, transfer, efficiency units, social cost of carbon, pollution damage, marginal damage, Ramsey, simple/log returns, rolling mean/variance, labor supply, optimal abatement/pollution, OLG, Euler residual, present-biased and reference-dependent utility, NPV, multi-period DDM, portfolio max drawdown, zero-rate interpolate, discount factor, par yield, progressive tax, abatement net benefit, household CES, malfeasance delta, portfolio variance/returns, distributional NPV, stress scenario, repeated-game payoff, transport cost, Markov transition/holding, IR check, VCG payment, validate transition, stationary distribution, mean first passage, degree/eigenvector centrality, New Keynesian, nearest facility, pure Nash, Moran's I, strategy-proofness, Lorenz, OLS/WLS/2SLS/logistic, Lucas price, Bellman, block bootstrap, simulate chain, value iteration, interbank clearing, Leontief inverse/multipliers, agent-based wealth status, scalar constraint, paper fills status."
                    .into(),
            },
            vec![
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:capm".into(),
                        label: "CAPM expected return".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.capm_expected_return".into()),
                        ontology_prefix: "econ".into(),
                        description: "Expected return from rf, beta, and market premium on the selected surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:gini".into(),
                        label: "Gini coefficient".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.gini".into()),
                        ontology_prefix: "econ".into(),
                        description: "Inequality of income numbers on the selected sheet or document."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:mixed_nash".into(),
                        label: "Mixed Nash 2×2".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.mixed_nash_2x2".into()),
                        ontology_prefix: "econ".into(),
                        description: "Mixed-strategy Nash from two 2×2 payoff matrices (eight numbers)."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:black_scholes".into(),
                        label: "Black–Scholes".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.black_scholes".into()),
                        ontology_prefix: "econ".into(),
                        description: "Option price and Greeks from spot, strike, time, rate, and volatility."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:binomial_option".into(),
                        label: "Binomial option".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.binomial_option".into()),
                        ontology_prefix: "econ".into(),
                        description: "CRR binomial European option price from spot, strike, time, rate, and volatility."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:solow".into(),
                        label: "Solow steady state".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.solow_steady_state".into()),
                        ontology_prefix: "econ".into(),
                        description: "Steady-state capital and output from savings, alpha, and depreciation."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:cournot".into(),
                        label: "Cournot duopoly".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.cournot_duopoly".into()),
                        ontology_prefix: "econ".into(),
                        description: "Quantity-setting duopoly from demand intercept/slope and two costs."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:bertrand".into(),
                        label: "Bertrand duopoly".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.bertrand_duopoly".into()),
                        ontology_prefix: "econ".into(),
                        description: "Price-setting duopoly from two marginal costs on the selected surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:historical_var".into(),
                        label: "Historical VaR".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.historical_var".into()),
                        ontology_prefix: "econ".into(),
                        description: "Left-tail historical value-at-risk from return numbers on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:atkinson".into(),
                        label: "Atkinson inequality".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.atkinson".into()),
                        ontology_prefix: "econ".into(),
                        description: "Atkinson index from positive incomes and inequality-aversion epsilon."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:gordon_growth".into(),
                        label: "Gordon growth".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.gordon_growth".into()),
                        ontology_prefix: "econ".into(),
                        description: "Dividend discount price from next dividend, required return, and growth."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:forward_rate".into(),
                        label: "Forward rate".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.forward_rate".into()),
                        ontology_prefix: "econ".into(),
                        description: "Annualized forward from a zero curve between two tenors on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:gbm_simulate".into(),
                        label: "GBM simulate".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.gbm_simulate".into()),
                        ontology_prefix: "econ".into(),
                        description: "Geometric Brownian motion path from S0, drift, volatility, and step count."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:headcount_poverty".into(),
                        label: "Headcount poverty".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.headcount_poverty".into()),
                        ontology_prefix: "econ".into(),
                        description: "Share of incomes below a poverty line on the selected surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:hyperbolic_discount".into(),
                        label: "Hyperbolic discount".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.hyperbolic_discount".into()),
                        ontology_prefix: "econ".into(),
                        description: "β-δ present-bias discount factor for a chosen horizon."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:fiscal_multiplier".into(),
                        label: "Fiscal multiplier".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.fiscal_multiplier".into()),
                        ontology_prefix: "econ".into(),
                        description: "GDP impact from initial spending, MPC, and leakage rate."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:drawdown".into(),
                        label: "Drawdown series".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.drawdown".into()),
                        ontology_prefix: "econ".into(),
                        description: "Peak-to-trough drawdown path from a wealth index on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:covariance_matrix".into(),
                        label: "Covariance matrix".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.covariance_matrix".into()),
                        ontology_prefix: "econ".into(),
                        description: "Sample covariance from flat period×asset returns on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:capm_beta".into(),
                        label: "CAPM beta".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.capm_beta".into()),
                        ontology_prefix: "econ".into(),
                        description: "Asset beta from paired asset and market return series on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:autocorrelation".into(),
                        label: "Autocorrelation".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.autocorrelation".into()),
                        ontology_prefix: "econ".into(),
                        description: "Lag-k autocorrelation of a return or price series on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:cross_correlation".into(),
                        label: "Cross-correlation".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.cross_correlation".into()),
                        ontology_prefix: "econ".into(),
                        description: "Cross-correlation between two equal-length series on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:bertrand_with_demand".into(),
                        label: "Bertrand with demand".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.bertrand_with_demand".into()),
                        ontology_prefix: "econ".into(),
                        description: "Bertrand price and quantity from linear demand and two costs."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:check_budget_balance".into(),
                        label: "Budget balance".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.check_budget_balance".into()),
                        ontology_prefix: "econ".into(),
                        description: "Check whether mechanism payments sum to a non-negative surplus."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:ccapm_equity_premium".into(),
                        label: "CCAPM premium".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.ccapm_equity_premium".into()),
                        ontology_prefix: "econ".into(),
                        description: "Consumption-CAPM equity premium from risk aversion and volatilities."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:mean_return".into(),
                        label: "Mean return".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.mean_return".into()),
                        ontology_prefix: "econ".into(),
                        description: "Arithmetic mean of return numbers on the selected surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:poverty_gap".into(),
                        label: "Poverty gap".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.poverty_gap".into()),
                        ontology_prefix: "econ".into(),
                        description: "Average shortfall below a poverty line as a share of the line."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:sample_variance".into(),
                        label: "Sample variance".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.sample_variance".into()),
                        ontology_prefix: "econ".into(),
                        description: "Bessel-corrected sample variance of return numbers on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:utilitarian_welfare".into(),
                        label: "Utilitarian welfare".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.utilitarian_welfare".into()),
                        ontology_prefix: "econ".into(),
                        description: "Sum of utility numbers on the selected sheet or document."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:rawlsian_welfare".into(),
                        label: "Rawlsian welfare".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.rawlsian_welfare".into()),
                        ontology_prefix: "econ".into(),
                        description: "Maximin welfare — the minimum utility on the selected surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:nash_welfare".into(),
                        label: "Nash welfare".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.nash_welfare".into()),
                        ontology_prefix: "econ".into(),
                        description: "Nash product of strictly positive utilities on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:stackelberg".into(),
                        label: "Stackelberg duopoly".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.stackelberg_duopoly".into()),
                        ontology_prefix: "econ".into(),
                        description: "Leader–follower quantities and price from demand and two costs."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:put_call_parity".into(),
                        label: "Put-call parity".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.put_call_parity".into()),
                        ontology_prefix: "econ".into(),
                        description: "Put-call parity residual from call, put, spot, strike, and rates."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:parametric_var".into(),
                        label: "Parametric VaR".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.parametric_var".into()),
                        ontology_prefix: "econ".into(),
                        description: "Gaussian value-at-risk from mean, volatility, and confidence."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:laffer_curve".into(),
                        label: "Laffer curve".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.laffer_curve".into()),
                        ontology_prefix: "econ".into(),
                        description: "Tax revenue from rate, base, and behavioral elasticity."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:historical_cvar".into(),
                        label: "Historical CVaR".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.historical_cvar".into()),
                        ontology_prefix: "econ".into(),
                        description: "Expected shortfall from return numbers on the selected surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:endowment_effect".into(),
                        label: "Endowment effect".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.endowment_effect".into()),
                        ontology_prefix: "econ".into(),
                        description: "Willingness-to-accept from WTP and loss-aversion lambda."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:prospect_value".into(),
                        label: "Prospect value".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.prospect_value".into()),
                        ontology_prefix: "econ".into(),
                        description: "Kahneman–Tversky prospect value from outcome and preference params."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:probability_weight".into(),
                        label: "Probability weight".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.probability_weight".into()),
                        ontology_prefix: "econ".into(),
                        description: "Prelec probability weighting from probability and gamma."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:ccapm_sdf".into(),
                        label: "CCAPM SDF".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.ccapm_sdf".into()),
                        ontology_prefix: "econ".into(),
                        description: "Consumption-CAPM stochastic discount factor from growth and risk aversion."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:gravity_flow".into(),
                        label: "Gravity flow".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.gravity_flow".into()),
                        ontology_prefix: "econ".into(),
                        description: "Spatial gravity trade/flow between two masses and a distance."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:transfer_payment".into(),
                        label: "Transfer payment".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.transfer_payment".into()),
                        ontology_prefix: "econ".into(),
                        description: "Means-tested transfer from base, income, threshold, and phaseout."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:efficiency_units".into(),
                        label: "Efficiency units".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.efficiency_units".into()),
                        ontology_prefix: "econ".into(),
                        description: "Effective labor as raw hours times human capital."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:social_cost_of_carbon".into(),
                        label: "Social cost of carbon".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.social_cost_of_carbon".into()),
                        ontology_prefix: "econ".into(),
                        description: "Social cost from emissions tons and damage per ton."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:pollution_damage".into(),
                        label: "Pollution damage".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.pollution_damage".into()),
                        ontology_prefix: "econ".into(),
                        description: "Quadratic pollution damage from emissions and damage coefficient."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:marginal_damage".into(),
                        label: "Marginal damage".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.marginal_damage".into()),
                        ontology_prefix: "econ".into(),
                        description: "Marginal pollution damage from emissions and damage coefficient."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:ramsey_steady_state".into(),
                        label: "Ramsey steady state".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.ramsey_steady_state".into()),
                        ontology_prefix: "econ".into(),
                        description: "Ramsey–Cass–Koopmans steady-state capital from α, β, and δ."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:simple_returns".into(),
                        label: "Simple returns".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.simple_returns".into()),
                        ontology_prefix: "econ".into(),
                        description: "Period simple returns from a strictly positive price series."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:log_returns".into(),
                        label: "Log returns".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.log_returns".into()),
                        ontology_prefix: "econ".into(),
                        description: "Log returns from a strictly positive price series on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:rolling_mean".into(),
                        label: "Rolling mean".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.rolling_mean".into()),
                        ontology_prefix: "econ".into(),
                        description: "Rolling mean of a numeric series for a chosen window length."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:rolling_variance".into(),
                        label: "Rolling variance".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.rolling_variance".into()),
                        ontology_prefix: "econ".into(),
                        description: "Rolling population variance of a numeric series for a window."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:labor_supply".into(),
                        label: "Labor supply".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.labor_supply".into()),
                        ontology_prefix: "econ".into(),
                        description: "Cobb–Douglas labor hours and consumption from wage and endowment."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:optimal_abatement".into(),
                        label: "Optimal abatement".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.optimal_abatement".into()),
                        ontology_prefix: "econ".into(),
                        description: "Optimal abatement from baseline emissions and MAC/MD coefficients."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:optimal_pollution".into(),
                        label: "Optimal pollution".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.optimal_pollution".into()),
                        ontology_prefix: "econ".into(),
                        description: "Optimal emissions from baseline and abatement/damage coefficients."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:olg_steady_state".into(),
                        label: "OLG steady state".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.olg_steady_state".into()),
                        ontology_prefix: "econ".into(),
                        description: "Overlapping-generations steady-state capital and output."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:ramsey_euler_residual".into(),
                        label: "Ramsey Euler residual".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.ramsey_euler_residual".into()),
                        ontology_prefix: "econ".into(),
                        description: "Ramsey Euler equation residual from capital path and prefs."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:present_biased_utility".into(),
                        label: "Present-biased utility".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.present_biased_utility".into()),
                        ontology_prefix: "econ".into(),
                        description: "β-δ present-biased discounted utility from a utility path."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:reference_dependent_utility".into(),
                        label: "Reference-dependent utility".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.reference_dependent_utility".into()),
                        ontology_prefix: "econ".into(),
                        description: "Prospect utility of an outcome relative to a reference point."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:npv".into(),
                        label: "Net present value".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.npv".into()),
                        ontology_prefix: "econ".into(),
                        description: "Discounted net present value from paired benefit and cost streams."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:multi_period_ddm".into(),
                        label: "Multi-period DDM".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.multi_period_ddm".into()),
                        ontology_prefix: "econ".into(),
                        description: "Dividend discount price with a Gordon terminal from a dividend path."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:portfolio_max_drawdown".into(),
                        label: "Portfolio max drawdown".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.portfolio_max_drawdown".into()),
                        ontology_prefix: "econ".into(),
                        description: "Peak-to-trough max drawdown from a portfolio return series."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:interpolate_zero_rate".into(),
                        label: "Interpolate zero rate".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.interpolate_zero_rate".into()),
                        ontology_prefix: "econ".into(),
                        description: "Linearly interpolate a zero rate at a target maturity on the curve."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:discount_factor".into(),
                        label: "Discount factor".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.discount_factor".into()),
                        ontology_prefix: "econ".into(),
                        description: "Discount factor from an interpolated zero curve at a maturity."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:par_yield".into(),
                        label: "Par yield".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.par_yield".into()),
                        ontology_prefix: "econ".into(),
                        description: "Par coupon yield implied by a zero curve at a target maturity."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:progressive_tax".into(),
                        label: "Progressive tax".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.progressive_tax".into()),
                        ontology_prefix: "econ".into(),
                        description: "Total tax and effective rate from brackets and taxable income."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:abatement_net_benefit".into(),
                        label: "Abatement net benefit".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.abatement_net_benefit".into()),
                        ontology_prefix: "econ".into(),
                        description: "Net social benefit of abatement between baseline and actual emissions."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:household_production_ces".into(),
                        label: "Household CES production".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.household_production_ces".into()),
                        ontology_prefix: "econ".into(),
                        description: "CES household production from time, goods, alpha, and rho."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:malfeasance_delta".into(),
                        label: "Malfeasance delta".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.malfeasance_delta".into()),
                        ontology_prefix: "econ".into(),
                        description: "Governance gap between capital allocated and utility delivered."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:portfolio_variance".into(),
                        label: "Portfolio variance".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.portfolio_variance".into()),
                        ontology_prefix: "econ".into(),
                        description: "Quadratic-form portfolio variance from weights and covariance."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:portfolio_returns".into(),
                        label: "Portfolio returns".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.portfolio_returns".into()),
                        ontology_prefix: "econ".into(),
                        description: "Weighted portfolio return series from row-major asset returns."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:distributional_npv".into(),
                        label: "Distributional NPV".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.distributional_npv".into()),
                        ontology_prefix: "econ".into(),
                        description: "Weighted and unweighted NPV across distributional period weights."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:stress_scenario".into(),
                        label: "Stress scenario".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.stress_scenario".into()),
                        ontology_prefix: "econ".into(),
                        description: "Scale a return or price series by a uniform shock factor."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:repeated_game_payoff".into(),
                        label: "Repeated-game payoff".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.repeated_game_payoff".into()),
                        ontology_prefix: "econ".into(),
                        description: "Discounted sum of stage payoffs over a finite repeated game."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:total_transport_cost".into(),
                        label: "Total transport cost".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.total_transport_cost".into()),
                        ontology_prefix: "econ".into(),
                        description: "Sum of flow × distance over an origin–destination matrix."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:transition_probability".into(),
                        label: "Transition probability".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.transition_probability".into()),
                        ontology_prefix: "econ".into(),
                        description: "Look up P[from,to] in a row-major Markov transition matrix."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:expected_holding_time".into(),
                        label: "Expected holding time".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.expected_holding_time".into()),
                        ontology_prefix: "econ".into(),
                        description: "Expected sojourn 1/(1−P_ii) for a Markov state."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:check_ir".into(),
                        label: "Individual rationality".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.check_ir".into()),
                        ontology_prefix: "econ".into(),
                        description: "Check that each agent's payment does not exceed valuation."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:vcg_payment".into(),
                        label: "VCG payment".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.vcg_payment".into()),
                        ontology_prefix: "econ".into(),
                        description: "Vickrey–Clarke–Groves second-price payment from valuations."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:validate_transition_matrix".into(),
                        label: "Validate transition matrix".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.validate_transition_matrix".into()),
                        ontology_prefix: "econ".into(),
                        description: "Check that a row-major Markov matrix is row-stochastic."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:stationary_distribution".into(),
                        label: "Stationary distribution".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.stationary_distribution".into()),
                        ontology_prefix: "econ".into(),
                        description: "Stationary distribution of a row-stochastic Markov chain."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:mean_first_passage".into(),
                        label: "Mean first passage".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.mean_first_passage".into()),
                        ontology_prefix: "econ".into(),
                        description: "Mean first-passage times to a target Markov state."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:degree_centrality".into(),
                        label: "Degree centrality".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.degree_centrality".into()),
                        ontology_prefix: "econ".into(),
                        description: "Out-degree centrality from a row-major adjacency matrix."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:eigenvector_centrality".into(),
                        label: "Eigenvector centrality".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.eigenvector_centrality".into()),
                        ontology_prefix: "econ".into(),
                        description: "Eigenvector centrality from a row-major adjacency matrix."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:new_keynesian_solve".into(),
                        label: "New Keynesian solve".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.new_keynesian_solve".into()),
                        ontology_prefix: "econ".into(),
                        description: "One-period New Keynesian output gap, inflation, and rate."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:nearest_facility".into(),
                        label: "Nearest facility".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.nearest_facility".into()),
                        ontology_prefix: "econ".into(),
                        description: "Assign each demand point to its nearest facility coordinates."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:pure_nash_equilibria".into(),
                        label: "Pure Nash equilibria".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.pure_nash_equilibria".into()),
                        ontology_prefix: "econ".into(),
                        description: "Pure-strategy Nash equilibria of a two-player normal-form game."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:morans_i".into(),
                        label: "Moran's I".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.morans_i".into()),
                        ontology_prefix: "econ".into(),
                        description: "Moran's I spatial autocorrelation from values and weights."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:strategy_proofness".into(),
                        label: "Strategy-proofness".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.strategy_proofness".into()),
                        ontology_prefix: "econ".into(),
                        description: "Check 2×2 mechanism strategy-proofness for agent 0."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:lorenz_curve".into(),
                        label: "Lorenz curve".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.lorenz_curve".into()),
                        ontology_prefix: "econ".into(),
                        description: "Lorenz population/income shares from income numbers."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:ols".into(),
                        label: "OLS regression".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.ols".into()),
                        ontology_prefix: "econ".into(),
                        description: "Ordinary least squares on paired x/y surface numbers."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:wls".into(),
                        label: "WLS regression".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.wls".into()),
                        ontology_prefix: "econ".into(),
                        description: "Weighted least squares from x, y, and weight columns."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:lucas_asset_price".into(),
                        label: "Lucas asset price".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.lucas_asset_price".into()),
                        ontology_prefix: "econ".into(),
                        description: "Lucas tree price from dividend and consumption paths."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:bellman_update".into(),
                        label: "Bellman update".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.bellman_update".into()),
                        ontology_prefix: "econ".into(),
                        description: "One-state Bellman operator update for a small MDP."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:block_bootstrap".into(),
                        label: "Block bootstrap".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.block_bootstrap".into()),
                        ontology_prefix: "econ".into(),
                        description: "Block-bootstrap means from return numbers on the surface."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:simulate_chain".into(),
                        label: "Simulate Markov chain".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.simulate_chain".into()),
                        ontology_prefix: "econ".into(),
                        description: "Simulate a path on a row-stochastic transition matrix."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:value_iteration".into(),
                        label: "Value iteration".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.value_iteration".into()),
                        ontology_prefix: "econ".into(),
                        description: "Value-function iteration for a small MDP."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:iv_2sls".into(),
                        label: "IV / 2SLS".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.iv_2sls".into()),
                        ontology_prefix: "econ".into(),
                        description: "Two-stage least squares with endogenous x and instruments z."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:logistic_mle".into(),
                        label: "Logistic MLE".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.logistic_mle".into()),
                        ontology_prefix: "econ".into(),
                        description: "Binary logistic regression via Newton–Raphson MLE."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:interbank_clearing".into(),
                        label: "Interbank clearing".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.interbank_clearing".into()),
                        ontology_prefix: "econ".into(),
                        description: "Eisenberg–Noe clearing payments from exposures and capital."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:leontief_inverse".into(),
                        label: "Leontief inverse".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.leontief_inverse".into()),
                        ontology_prefix: "econ".into(),
                        description: "Neumann-series Leontief inverse of a technical-coefficient matrix."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:output_multipliers".into(),
                        label: "Output multipliers".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.output_multipliers".into()),
                        ontology_prefix: "econ".into(),
                        description: "Column-sum output multipliers from a Leontief inverse."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:agent_based_aggregate_wealth".into(),
                        label: "Agent wealth status".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.agent_based_aggregate_wealth".into()),
                        ontology_prefix: "econ".into(),
                        description: "Probe Host availability of agent-based aggregate wealth."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:validate_scalar_constraint".into(),
                        label: "Scalar constraint".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.validate_scalar_constraint".into()),
                        ontology_prefix: "econ".into(),
                        description: "Check that a scalar econ result lies within [min, max]."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
                Box::new(SimpleTool::new(
                    ToolMetadata {
                        id: "econ:aggregate_paper_fills".into(),
                        label: "Paper fills status".into(),
                        icon: "finance".into(),
                        kind: ToolKind::RunAction,
                        capability_scope: Some("Econ.aggregate_paper_fills".into()),
                        ontology_prefix: "econ".into(),
                        description: "Probe Host availability of paper-trading fill aggregation."
                            .into(),
                    },
                    ActionType::Invoke,
                )),
            ],
        ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "social:live".into(),
                    label: "Live social & forensic".into(),
                    icon: "finance".into(),
                    description: "Host-bound Social.* and Forensic.* leftovers.".into(),
                },
                super::register_wave36_live::social_tools(),
            ),
            ToolChain::new(
                ToolChainMetadata {
                    id: "finance:live".into(),
                    label: "Live finance remainder".into(),
                    icon: "finance".into(),
                    description: "Host-bound Finance.convert_currency / multisig / ledger binds.".into(),
                },
                super::register_wave36_live::finance_tools(),
            ),
        ],
    ));
}
