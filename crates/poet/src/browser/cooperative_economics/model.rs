//! Cooperative economics models — SDN lanes, ontological pricing, true-cost.

use serde::{Deserialize, Serialize};

/// Permissive routing lanes under Socially Defined Networking.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SocialRoutingLane {
    Commons,
    Bilateral,
    Federated,
    Commercial,
}

impl SocialRoutingLane {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Commons => "Lane::Commons (Public Knowledge)",
            Self::Bilateral => "Lane::Bilateral (1-on-1 Projects)",
            Self::Federated => "Lane::Federated (Cooperative Swarms)",
            Self::Commercial => "Lane::Commercial (Metered Transit)",
        }
    }
}

/// Ontological classification of a requesting network peer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerOntologyClass {
    NaturalPerson { is_human_verified: bool },
    ResearchCollective { lab_name: String },
    Corporation {
        company_name: String,
        tax_id: Option<String>,
    },
    AnonymousOrUnverified,
}

/// Verdict returned by the Ontological Pricing Engine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AccessVerdict {
    PermitFree {
        free_bandwidth_gb: u32,
        reason: String,
    },
    ReciprocalBarter {
        allowed_storage_gb: u32,
        required_return: String,
    },
    MeteredPayment {
        rate_per_gb_cents: u32,
        rate_per_gpu_sec_cents: u32,
    },
    Deny { reason: String },
}

/// Evaluator for Ontological Economic Access Policies.
pub struct OntologicalPricingEngine;

impl OntologicalPricingEngine {
    pub fn evaluate_peer(peer: &PeerOntologyClass) -> AccessVerdict {
        match peer {
            PeerOntologyClass::NaturalPerson {
                is_human_verified: true,
            } => AccessVerdict::PermitFree {
                free_bandwidth_gb: 25,
                reason: "Universal human commons quota (inquiry & basic research)".into(),
            },
            PeerOntologyClass::NaturalPerson {
                is_human_verified: false,
            } => AccessVerdict::PermitFree {
                free_bandwidth_gb: 5,
                reason: "Unverified human trial quota".into(),
            },
            PeerOntologyClass::ResearchCollective { lab_name } => AccessVerdict::ReciprocalBarter {
                allowed_storage_gb: 50,
                required_return: format!("qualia:FederatedGradientAccess with {lab_name}"),
            },
            PeerOntologyClass::Corporation { .. } => AccessVerdict::MeteredPayment {
                rate_per_gb_cents: 15,
                rate_per_gpu_sec_cents: 5,
            },
            PeerOntologyClass::AnonymousOrUnverified => AccessVerdict::Deny {
                reason: "Anonymous requests require DID credential or PoW token".into(),
            },
        }
    }
}

/// True-Cost Personal Unit Economics Parameters (local; not a live market quote).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrueCostModel {
    pub hardware_purchase_price_aud: f64,
    pub lifespan_months: f64,
    pub monthly_isp_bill_aud: f64,
    pub monthly_data_cap_gb: f64,
    pub system_power_draw_kw: f64,
    pub electricity_tariff_per_kwh_aud: f64,
}

impl Default for TrueCostModel {
    fn default() -> Self {
        Self {
            hardware_purchase_price_aud: 3600.0,
            lifespan_months: 36.0,
            monthly_isp_bill_aud: 85.0,
            monthly_data_cap_gb: 1000.0,
            system_power_draw_kw: 0.350,
            electricity_tariff_per_kwh_aud: 0.32,
        }
    }
}

impl TrueCostModel {
    pub fn hardware_cost_per_hour(&self) -> f64 {
        let total_hours = self.lifespan_months * 730.0;
        if total_hours > 0.0 {
            self.hardware_purchase_price_aud / total_hours
        } else {
            0.0
        }
    }

    pub fn network_cost_per_gb(&self) -> f64 {
        if self.monthly_data_cap_gb > 0.0 {
            self.monthly_isp_bill_aud / self.monthly_data_cap_gb
        } else {
            0.0
        }
    }

    pub fn power_cost_per_hour(&self) -> f64 {
        self.system_power_draw_kw * self.electricity_tariff_per_kwh_aud
    }

    pub fn total_job_cost(&self, duration_hours: f64, transfer_gb: f64) -> f64 {
        (self.hardware_cost_per_hour() + self.power_cost_per_hour()) * duration_hours
            + self.network_cost_per_gb() * transfer_gb
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ontological_pricing_human_quota() {
        let human = PeerOntologyClass::NaturalPerson {
            is_human_verified: true,
        };
        let verdict = OntologicalPricingEngine::evaluate_peer(&human);
        assert_eq!(
            verdict,
            AccessVerdict::PermitFree {
                free_bandwidth_gb: 25,
                reason: "Universal human commons quota (inquiry & basic research)".into(),
            }
        );
    }

    #[test]
    fn test_ontological_pricing_research_barter() {
        let lab = PeerOntologyClass::ResearchCollective {
            lab_name: "OpenAnatomyLab".into(),
        };
        let verdict = OntologicalPricingEngine::evaluate_peer(&lab);
        match verdict {
            AccessVerdict::ReciprocalBarter {
                allowed_storage_gb,
                required_return,
            } => {
                assert_eq!(allowed_storage_gb, 50);
                assert!(required_return.contains("OpenAnatomyLab"));
            }
            _ => panic!("Expected reciprocal barter"),
        }
    }

    #[test]
    fn test_true_cost_calculations() {
        let model = TrueCostModel::default();
        let hw_cost = model.hardware_cost_per_hour();
        let net_cost = model.network_cost_per_gb();
        let pwr_cost = model.power_cost_per_hour();

        assert!(hw_cost > 0.13 && hw_cost < 0.14);
        assert!((net_cost - 0.085).abs() < 1e-6);
        assert!((pwr_cost - 0.112).abs() < 1e-6);

        let job_cost = model.total_job_cost(2.0, 10.0);
        assert!(job_cost > 1.30 && job_cost < 1.40);
    }
}
