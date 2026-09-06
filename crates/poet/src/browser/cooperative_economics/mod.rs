//! Cooperative Systems, SDN & Ontological Economics Subsystem (Spec 20).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements Socially Defined Networking (SDN), P2P Swarm Coordination,
//! Ontological Pricing Rules (human commons quota, academic barter, enterprise metering),
//! True-Cost Personal Unit Economics ($C_hw + C_net + C_pwr), and Live `Econ.gini`
//! on user-supplied incomes.

mod live_welfare;
mod model;
mod view;

use web_sys::{Document, Element};

pub use model::{
    AccessVerdict, OntologicalPricingEngine, PeerOntologyClass, SocialRoutingLane, TrueCostModel,
};

/// Build the Cooperative Systems & Ontological Economics Viewport.
pub fn build_cooperative_economics_view(
    document: &Document,
    cost_model: &TrueCostModel,
) -> Element {
    view::build_cooperative_economics_view(document, cost_model)
}
