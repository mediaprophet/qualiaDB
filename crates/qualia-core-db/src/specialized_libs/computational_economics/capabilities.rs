//! Capability/status matrix for computational economics and finance.
//!
//! The matrix is intentionally blunt: it prevents scaffolding from being
//! counted as implemented math.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityDomain {
    Statistics,
    Economics,
    Finance,
    Accounting,
    Compliance,
    Interface,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityStatus {
    ImplementedKernel,
    PartialKernel,
    RegistryScaffold,
    RefusingSafetyStub,
    Planned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationClass {
    HotZeroHeap,
    ColdBounded,
    OwnedConvenience,
    MetadataOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SafetyClass {
    PureComputation,
    RequiresProvenance,
    RequiresHumanReview,
    SimulationOnly,
    RefusesExternalAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilityRecord {
    pub id: &'static str,
    pub domain: CapabilityDomain,
    pub status: CapabilityStatus,
    pub allocation: AllocationClass,
    pub safety: SafetyClass,
    pub module_path: &'static str,
    pub notes: &'static str,
}

pub const COMPUTATIONAL_ECONOMICS_CAPABILITIES: &[CapabilityRecord] = &[
    CapabilityRecord {
        id: "statistics.descriptive",
        domain: CapabilityDomain::Statistics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "solvers::statistics::descriptive",
        notes: "Slice-only mean/variance/covariance/quantile foundations.",
    },
    CapabilityRecord {
        id: "statistics.robust_owned",
        domain: CapabilityDomain::Statistics,
        status: CapabilityStatus::PartialKernel,
        allocation: AllocationClass::OwnedConvenience,
        safety: SafetyClass::PureComputation,
        module_path: "solvers::statistics::robust",
        notes: "Useful robust estimators, but several allocate internal Vec scratch.",
    },
    CapabilityRecord {
        id: "statistics.distributions_extra",
        domain: CapabilityDomain::Statistics,
        status: CapabilityStatus::PartialKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "solvers::statistics::distributions",
        notes: "Binomial, Poisson, lognormal, exponential, uniform, laplace, gamma, beta, weibull, empirical + more. Full set for 5.1-A.",
    },
    CapabilityRecord {
        id: "statistics.hypothesis_nonparametric",
        domain: CapabilityDomain::Statistics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "solvers::statistics::hypothesis::nonparametric",
        notes: "Mann-Whitney U, KS 1-sample, McNemar, Friedman implemented with p-values.",
    },
    CapabilityRecord {
        id: "statistics.time_series_stats",
        domain: CapabilityDomain::Statistics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "solvers::statistics",
        notes: "ljung_box, adf_proxy added.",
    },
    CapabilityRecord {
        id: "statistics.resampling_starter",
        domain: CapabilityDomain::Statistics,
        status: CapabilityStatus::PartialKernel,
        allocation: AllocationClass::ColdBounded,
        safety: SafetyClass::PureComputation,
        module_path: "solvers::statistics",
        notes: "Basic bootstrap_means (seeded, caller buffer). Full block/jackknife + CI still needed.",
    },
    CapabilityRecord {
        id: "economics.gbm_seeded",
        domain: CapabilityDomain::Economics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "domains::financial::economics::stochastic",
        notes: "Deterministic caller-buffered GBM and Monte Carlo VaR variants.",
    },
    CapabilityRecord {
        id: "economics.input_output",
        domain: CapabilityDomain::Economics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "domains::financial::economics::input_output",
        notes: "Bounded Leontief shock propagation over caller buffers.",
    },
    CapabilityRecord {
        id: "finance.portfolio_risk",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::OwnedConvenience,
        safety: SafetyClass::RequiresProvenance,
        module_path: "specialized_libs::financial_modeling::portfolio_risk",
        notes: "Real return-based risk metrics; refuses missing/misaligned histories.",
    },
    CapabilityRecord {
        id: "finance.black_scholes",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::ColdBounded,
        safety: SafetyClass::PureComputation,
        module_path: "specialized_libs::financial_modeling::PricingEngine",
        notes: "European call/put Black-Scholes price and Greeks.",
    },
    CapabilityRecord {
        id: "finance.fixed_income_basic",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "specialized_libs::computational_economics::fixed_income",
        notes: "Day-count, accrued interest, clean/dirty price, coupon bonds, duration, convexity, DV01.",
    },
    CapabilityRecord {
        id: "finance.yield_curve_basic",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "specialized_libs::computational_economics::yield_curve",
        notes: "Zero-rate interpolation, discount factors, forwards, par yields, and par-yield bootstrapping.",
    },
    CapabilityRecord {
        id: "finance.market_data_adjustment",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::RequiresProvenance,
        module_path: "specialized_libs::computational_economics::market_data",
        notes: "Supplied-bar adjustment factors, adjusted closes, simple/log returns, and VWAP with provenance checks.",
    },
    CapabilityRecord {
        id: "finance.portfolio_analytics_basic",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::RequiresProvenance,
        module_path: "specialized_libs::computational_economics::portfolio",
        notes: "Flat-matrix portfolio returns, sample covariance, variance, volatility risk contributions, drawdown.",
    },
    CapabilityRecord {
        id: "finance.risk_metrics_basic",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::RequiresProvenance,
        module_path: "specialized_libs::computational_economics::risk",
        notes: "Historical VaR/CVaR, Gaussian VaR, and supplied-scenario losses over caller-provided data.",
    },
    CapabilityRecord {
        id: "finance.derivatives_basic",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "specialized_libs::computational_economics::derivatives",
        notes: "Black-Scholes-Merton price/Greeks, put-call parity, and CRR binomial European/American pricing.",
    },
    CapabilityRecord {
        id: "accounting.double_entry_basic",
        domain: CapabilityDomain::Accounting,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::RequiresProvenance,
        module_path: "specialized_libs::computational_economics::accounting",
        notes: "Minor-unit double-entry posting validation, account balances, journal-entry checks, trial balance.",
    },
    CapabilityRecord {
        id: "finance.trade_execution",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::RefusingSafetyStub,
        allocation: AllocationClass::MetadataOnly,
        safety: SafetyClass::RefusesExternalAction,
        module_path: "specialized_libs::financial_modeling::TradingEngine",
        notes: "Correctly refuses real/fabricated order execution.",
    },
    CapabilityRecord {
        id: "finance.order_settlement_registries",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::RegistryScaffold,
        allocation: AllocationClass::OwnedConvenience,
        safety: SafetyClass::SimulationOnly,
        module_path: "specialized_libs::financial_modeling",
        notes: "Order/routing/settlement/reporting types exist, but kernels are not comprehensive.",
    },
    CapabilityRecord {
        id: "accounting.personal_ledger",
        domain: CapabilityDomain::Accounting,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::OwnedConvenience,
        safety: SafetyClass::RequiresProvenance,
        module_path: "wellfare_core::finance",
        notes: "Replay-safe signed minor-unit ledger and derived balances.",
    },
    CapabilityRecord {
        id: "tax.illustrative_clearing",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::PartialKernel,
        allocation: AllocationClass::OwnedConvenience,
        safety: SafetyClass::RequiresHumanReview,
        module_path: "domains::financial::tax_schema",
        notes: "Simple AU/EU/US/zero-rated examples; not jurisdiction-complete tax law.",
    },
    CapabilityRecord {
        id: "economics.markov",
        domain: CapabilityDomain::Economics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "specialized_libs::computational_economics::markov",
        notes: "Stationary distribution, simulation, mean first passage into caller buffers.",
    },
    CapabilityRecord {
        id: "economics.dynamic_programming",
        domain: CapabilityDomain::Economics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "specialized_libs::computational_economics::dynamic_programming",
        notes: "VFI, policy iteration, Bellman, optimal stopping for finite MDPs.",
    },
    CapabilityRecord {
        id: "economics.welfare",
        domain: CapabilityDomain::Economics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::RequiresHumanReview,
        module_path: "specialized_libs::computational_economics::welfare",
        notes: "Gini, Atkinson, Rawlsian/utilitarian, survival floor allocation.",
    },
    CapabilityRecord {
        id: "economics.agent_based",
        domain: CapabilityDomain::Economics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "specialized_libs::computational_economics::agent_based",
        notes: "Fixed-capacity ZI traders, order book, deterministic replayable simulation.",
    },
    CapabilityRecord {
        id: "economics.forensic_nquin",
        domain: CapabilityDomain::Economics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::RequiresHumanReview,
        module_path: "specialized_libs::computational_economics::forensic_economics",
        notes: "Nquin trajectories, harm accumulation, malfeasance, epistemic negligence, narrative divergence, counterfactuals. Rights-affecting.",
    },
    CapabilityRecord {
        id: "economics.ontology_bridge",
        domain: CapabilityDomain::Economics,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::HotZeroHeap,
        safety: SafetyClass::PureComputation,
        module_path: "specialized_libs::computational_economics::ontology_bridge",
        notes: "NQuin encoders + basic FIBO/SHACL-style validation for econ scalars/vectors.",
    },
    CapabilityRecord {
        id: "finance.paper_trading",
        domain: CapabilityDomain::Finance,
        status: CapabilityStatus::ImplementedKernel,
        allocation: AllocationClass::ColdBounded,
        safety: SafetyClass::SimulationOnly,
        module_path: "specialized_libs::computational_economics::paper_trading",
        notes: "Deterministic paper fills from supplied snapshots only. Explicit no-real-execution guard.",
    },
];

pub fn capabilities_by_domain(domain: CapabilityDomain, out: &mut [CapabilityRecord]) -> usize {
    let mut written = 0usize;
    for record in COMPUTATIONAL_ECONOMICS_CAPABILITIES {
        if record.domain == domain {
            if written >= out.len() {
                break;
            }
            out[written] = *record;
            written += 1;
        }
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_has_no_empty_ids() {
        assert!(COMPUTATIONAL_ECONOMICS_CAPABILITIES
            .iter()
            .all(|c| !c.id.is_empty() && !c.module_path.is_empty()));
    }

    #[test]
    fn filters_capabilities_by_domain_into_caller_buffer() {
        let blank = CapabilityRecord {
            id: "",
            domain: CapabilityDomain::Finance,
            status: CapabilityStatus::Planned,
            allocation: AllocationClass::MetadataOnly,
            safety: SafetyClass::PureComputation,
            module_path: "",
            notes: "",
        };
        let mut out = [blank; 4];
        let n = capabilities_by_domain(CapabilityDomain::Finance, &mut out);
        assert!(n >= 3);
        assert!(out[..n]
            .iter()
            .all(|r| r.domain == CapabilityDomain::Finance));
    }
}
