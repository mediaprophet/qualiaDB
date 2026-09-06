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
                description: "Curated Econ.* binds — CAPM, Gini, Nash, Black–Scholes, Solow, Cournot, Bertrand, VaR, Atkinson, Gordon."
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
            ],
        )],
    ));
}
