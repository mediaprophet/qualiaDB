use super::{Cat, QApp, Stat};

pub(super) fn apps() -> Vec<QApp> {
    vec![
        // â”€â”€ Financial & Economics â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
        QApp {
            id: "portfolio",
            name: "Portfolio Analyzer",
            tagline: "Asset Management",
            desc: "Markowitz optimisation, Sharpe/Sortino ratios, and factor exposure analysis with \
                   ML-DSA fiduciary signatures â€” all zero-copy via the financial_modeling specialized library.",
            icon: "currency-exchange",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Financial,
        },
        QApp {
            id: "risk-engine",
            name: "Risk Engine",
            tagline: "Quantitative Risk",
            desc: "Value-at-Risk, Conditional VaR, stress testing, and Monte Carlo scenario generation. \
                   Results provenance-stamped and signed via the financial_modeling library.",
            icon: "graph-up-arrow",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Financial,
        },
        QApp {
            id: "gbm-sim",
            name: "GBM Simulator",
            tagline: "Stochastic Modelling",
            desc: "Geometric Brownian Motion and jump-diffusion price path simulation via \
                   domains::economics. Parameterise drift, volatility, and correlation matrices interactively.",
            icon: "shuffle",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Financial,
        },
        QApp {
            id: "tax-schema",
            name: "Tax Schema Editor",
            tagline: "Compliance",
            desc: "Define and evaluate tax rules via domains::tax_schema. Jurisdiction-specific rule \
                   trees, ODRL-linked obligation sets, and automated compliance reporting.",
            icon: "receipt",
            route: None,
            stat: Stat::Beta,
            cat: Cat::Financial,
        },
        QApp {
            id: "ilp-dashboard",
            name: "ILP Routing Dashboard",
            tagline: "Interledger",
            desc: "Monitor Interledger Protocol streaming micropayments via ilp_dispatcher. Track \
                   Remote inference metering and ontology seeding transactions in real time.",
            icon: "arrow-left-right",
            route: None,
            stat: Stat::Soon,
            cat: Cat::Financial,
        },
    ]
}
