//! Cooperative Systems, SDN & Ontological Economics Subsystem (Spec 20).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements Socially Defined Networking (SDN), P2P Swarm Coordination,
//! Ontological Pricing Rules (human commons quota, academic barter, enterprise metering),
//! and the True-Cost Personal Unit Economics Modeler ($C_hw + C_net + C_pwr).

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

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
    Corporation { company_name: String, tax_id: Option<String> },
    AnonymousOrUnverified,
}

/// Verdict returned by the Ontological Pricing Engine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AccessVerdict {
    PermitFree { free_bandwidth_gb: u32, reason: String },
    ReciprocalBarter { allowed_storage_gb: u32, required_return: String },
    MeteredPayment { rate_per_gb_cents: u32, rate_per_gpu_sec_cents: u32 },
    Deny { reason: String },
}

/// Evaluator for Ontological Economic Access Policies.
pub struct OntologicalPricingEngine;

impl OntologicalPricingEngine {
    pub fn evaluate_peer(peer: &PeerOntologyClass) -> AccessVerdict {
        match peer {
            PeerOntologyClass::NaturalPerson { is_human_verified: true } => {
                AccessVerdict::PermitFree {
                    free_bandwidth_gb: 25,
                    reason: "Universal human commons quota (inquiry & basic research)".into(),
                }
            }
            PeerOntologyClass::NaturalPerson { is_human_verified: false } => {
                AccessVerdict::PermitFree {
                    free_bandwidth_gb: 5,
                    reason: "Unverified human trial quota".into(),
                }
            }
            PeerOntologyClass::ResearchCollective { lab_name } => {
                AccessVerdict::ReciprocalBarter {
                    allowed_storage_gb: 50,
                    required_return: format!("qualia:FederatedGradientAccess with {}", lab_name),
                }
            }
            PeerOntologyClass::Corporation { .. } => {
                AccessVerdict::MeteredPayment {
                    rate_per_gb_cents: 15,       // $0.15 AUD / GB
                    rate_per_gpu_sec_cents: 5,   // $0.05 AUD / GPU-sec
                }
            }
            PeerOntologyClass::AnonymousOrUnverified => {
                AccessVerdict::Deny {
                    reason: "Anonymous requests require DID credential or PoW token".into(),
                }
            }
        }
    }
}

/// True-Cost Personal Unit Economics Parameters.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TrueCostModel {
    pub hardware_purchase_price_aud: f64, // e.g. $3,600 AUD
    pub lifespan_months: f64,             // e.g. 36 months
    pub monthly_isp_bill_aud: f64,        // e.g. $85 AUD
    pub monthly_data_cap_gb: f64,         // e.g. 1,000 GB
    pub system_power_draw_kw: f64,        // e.g. 0.350 kW
    pub electricity_tariff_per_kwh_aud: f64, // e.g. $0.32 AUD / kWh
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
    /// Calculate hardware amortization per hour ($C_hw).
    pub fn hardware_cost_per_hour(&self) -> f64 {
        let total_hours = self.lifespan_months * 730.0;
        if total_hours > 0.0 { self.hardware_purchase_price_aud / total_hours } else { 0.0 }
    }

    /// Calculate internet bandwidth cost per GB ($C_net).
    pub fn network_cost_per_gb(&self) -> f64 {
        if self.monthly_data_cap_gb > 0.0 { self.monthly_isp_bill_aud / self.monthly_data_cap_gb } else { 0.0 }
    }

    /// Calculate power and thermal cost per hour ($C_pwr).
    pub fn power_cost_per_hour(&self) -> f64 {
        self.system_power_draw_kw * self.electricity_tariff_per_kwh_aud
    }

    /// Calculate total cost to serve a 1-hour job that transfers N GB of data.
    pub fn total_job_cost(&self, duration_hours: f64, transfer_gb: f64) -> f64 {
        (self.hardware_cost_per_hour() + self.power_cost_per_hour()) * duration_hours
            + self.network_cost_per_gb() * transfer_gb
    }
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the Cooperative Systems & Ontological Economics Viewport.
pub fn build_cooperative_economics_view(document: &Document, cost_model: &TrueCostModel) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 12px; gap: 10px; \
         background: #020617; color: #f8fafc; overflow-y: auto; font-family: sans-serif;"
    );

    // Header Toolbar
    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 8px 12px;"
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some("\u{1F310} Socially Defined Networking & Ontological Economics"));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el.style().set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    header.append_child(&title).unwrap();

    let cost_hud = document.create_element("span").unwrap();
    cost_hud.set_text_content(Some(&format!(
        "Base Cost: ${:.3}/hr \u{00B7} Net: ${:.3}/GB \u{00B7} Power: ${:.3}/hr",
        cost_model.hardware_cost_per_hour(),
        cost_model.network_cost_per_gb(),
        cost_model.power_cost_per_hour()
    )));
    let cost_hud_el: HtmlElement = cost_hud.clone().dyn_into().unwrap();
    cost_hud_el.style().set_css_text("font-size: 11px; font-family: var(--font-mono); color: #34d399;");
    header.append_child(&cost_hud).unwrap();

    root.append_child(&header).unwrap();

    // 3 Cards Row
    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el.style().set_css_text("display: grid; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr)); gap: 10px;");

    // Card 1: SDN Routing Lanes
    let card1 = document.create_element("div").unwrap();
    let card1_el: HtmlElement = card1.clone().dyn_into().unwrap();
    card1_el.style().set_css_text(
        "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
         border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 6px;"
    );
    let c1_title = document.create_element("span").unwrap();
    c1_title.set_text_content(Some("\u{1F500} SDN Permissive Routing Lanes"));
    let c1_title_el: HtmlElement = c1_title.clone().dyn_into().unwrap();
    c1_title_el.style().set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    card1.append_child(&c1_title).unwrap();

    let lanes_text = document.create_element("pre").unwrap();
    lanes_text.set_text_content(Some(
        "\u{2022} Lane::Commons: Public Open-Access\n\
         \u{2022} Lane::Bilateral: 1-on-1 Projects\n\
         \u{2022} Lane::Federated: Swarm Compute\n\
         \u{2022} Lane::Commercial: Metered Transit"
    ));
    let lanes_text_el: HtmlElement = lanes_text.clone().dyn_into().unwrap();
    lanes_text_el.style().set_css_text("font-family: var(--font-mono); font-size: 10px; color: #94a3b8; margin: 4px 0 0 0; background: rgba(0,0,0,0.3); padding: 6px; border-radius: 4px;");
    card1.append_child(&lanes_text).unwrap();
    grid.append_child(&card1).unwrap();

    // Card 2: Ontological Pricing Matrix
    let card2 = document.create_element("div").unwrap();
    let card2_el: HtmlElement = card2.clone().dyn_into().unwrap();
    card2_el.style().set_css_text(
        "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
         border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 6px;"
    );
    let c2_title = document.create_element("span").unwrap();
    c2_title.set_text_content(Some("\u{1F39B}\u{FE0F} Ontological Pricing Matrix"));
    let c2_title_el: HtmlElement = c2_title.clone().dyn_into().unwrap();
    c2_title_el.style().set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    card2.append_child(&c2_title).unwrap();

    let matrix_text = document.create_element("pre").unwrap();
    matrix_text.set_text_content(Some(
        "\u{2022} Person: 25GB Free Commons Quota\n\
         \u{2022} ResearchLab: Reciprocal Barter\n\
         \u{2022} Corporation: $0.15/GB + $0.05/GPU-s\n\
         \u{2022} Anonymous: Gated Challenge"
    ));
    let matrix_text_el: HtmlElement = matrix_text.clone().dyn_into().unwrap();
    matrix_text_el.style().set_css_text("font-family: var(--font-mono); font-size: 10px; color: #94a3b8; margin: 4px 0 0 0; background: rgba(0,0,0,0.3); padding: 6px; border-radius: 4px;");
    card2.append_child(&matrix_text).unwrap();
    grid.append_child(&card2).unwrap();

    // Card 3: True-Cost Unit Economics
    let card3 = document.create_element("div").unwrap();
    let card3_el: HtmlElement = card3.clone().dyn_into().unwrap();
    card3_el.style().set_css_text(
        "background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); \
         border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 6px;"
    );
    let c3_title = document.create_element("span").unwrap();
    c3_title.set_text_content(Some("\u{1F4B0} True-Cost Personal Unit Economics"));
    let c3_title_el: HtmlElement = c3_title.clone().dyn_into().unwrap();
    c3_title_el.style().set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    card3.append_child(&c3_title).unwrap();

    let cost_breakdown = document.create_element("pre").unwrap();
    cost_breakdown.set_text_content(Some(&format!(
        "Hardware: ${:.4} / hr\n\
         Bandwidth: ${:.4} / GB\n\
         Electricity: ${:.4} / hr\n\
         Sample 1hr + 10GB Job: ${:.4} AUD",
        cost_model.hardware_cost_per_hour(),
        cost_model.network_cost_per_gb(),
        cost_model.power_cost_per_hour(),
        cost_model.total_job_cost(1.0, 10.0)
    )));
    let cost_breakdown_el: HtmlElement = cost_breakdown.clone().dyn_into().unwrap();
    cost_breakdown_el.style().set_css_text("font-family: var(--font-mono); font-size: 10px; color: #34d399; margin: 4px 0 0 0; background: rgba(0,0,0,0.3); padding: 6px; border-radius: 4px;");
    card3.append_child(&cost_breakdown).unwrap();
    grid.append_child(&card3).unwrap();

    root.append_child(&grid).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ontological_pricing_human_quota() {
        let human = PeerOntologyClass::NaturalPerson { is_human_verified: true };
        let verdict = OntologicalPricingEngine::evaluate_peer(&human);
        assert_eq!(verdict, AccessVerdict::PermitFree {
            free_bandwidth_gb: 25,
            reason: "Universal human commons quota (inquiry & basic research)".into(),
        });
    }

    #[test]
    fn test_ontological_pricing_research_barter() {
        let lab = PeerOntologyClass::ResearchCollective { lab_name: "OpenAnatomyLab".into() };
        let verdict = OntologicalPricingEngine::evaluate_peer(&lab);
        match verdict {
            AccessVerdict::ReciprocalBarter { allowed_storage_gb, required_return } => {
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

        assert!(hw_cost > 0.13 && hw_cost < 0.14); // ~$0.137/hr
        assert!((net_cost - 0.085).abs() < 1e-6);  // $0.085/GB
        assert!((pwr_cost - 0.112).abs() < 1e-6);  // $0.112/hr

        let job_cost = model.total_job_cost(2.0, 10.0);
        // (0.137 + 0.112) * 2 + 0.085 * 10 = ~0.498 + 0.850 = ~$1.348 AUD
        assert!(job_cost > 1.30 && job_cost < 1.40);
    }
}
