//! Chora 4D Spatio-Temporal Commons & Dialectical Web Reader (POET-SPEC-000 Domain 2).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements 4D spatio-temporal commons navigation, GeoSPARQL entity pins,
//! dialectical claim decomposition, and 1-click extraction to HyperDocs.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// A GeoSPARQL entity pinned on the 4D Chora canvas.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChoraEntityPin {
    pub id: String,
    pub title: String,
    pub lat: f64,
    pub lon: f64,
    pub year_epoch: i32,
    pub cultural_attribution: String,
    pub claim_summary: String,
    pub trust_weight: f32,
}

/// A dialectical claim pair representing opposing viewpoints.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DialecticalClaimPair {
    pub thesis: String,
    pub antithesis: String,
    pub synthesis_status: String,
    pub epistemic_certainty: f32,
}

/// State container for the Chora 4D Canvas and Dialectical Reader.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChoraManager {
    pub active_year: i32,
    pub zoom_level: f64,
    pub pins: Vec<ChoraEntityPin>,
    pub active_claims: Vec<DialecticalClaimPair>,
}

impl ChoraManager {
    pub fn new() -> Self {
        let sample_pin_1 = ChoraEntityPin {
            id: "pin-001".into(),
            title: "Ancient Catchment Water Rights Treaty".into(),
            lat: 31.7683,
            lon: 35.2137,
            year_epoch: 1200,
            cultural_attribution: "Commons Water Custodianship".into(),
            claim_summary: "Upstream riparian buffer protected under customary stewardship.".into(),
            trust_weight: 0.94,
        };

        let sample_pin_2 = ChoraEntityPin {
            id: "pin-002".into(),
            title: "Modern Hydro-Geological Sensor Array".into(),
            lat: 31.7800,
            lon: 35.2200,
            year_epoch: 2026,
            cultural_attribution: "IoT Commons Mesh".into(),
            claim_summary: "Live nitrate and flow rate telemetry broadcasting every 60s.".into(),
            trust_weight: 0.99,
        };

        let sample_claim = DialecticalClaimPair {
            thesis: "Centralized aquifer allocation maximizes agricultural productivity.".into(),
            antithesis:
                "Polycentric watershed commons preserves long-term subterranean water tables."
                    .into(),
            synthesis_status: "Verified Ostrom Equivalence in Super-Quin Quad-Store".into(),
            epistemic_certainty: 0.88,
        };

        Self {
            active_year: 2026,
            zoom_level: 1.0,
            pins: vec![sample_pin_1, sample_pin_2],
            active_claims: vec![sample_claim],
        }
    }

    /// Filter pins active around the selected historical or current year.
    pub fn pins_for_epoch(&self, window_years: i32) -> Vec<&ChoraEntityPin> {
        self.pins
            .iter()
            .filter(|p| (p.year_epoch - self.active_year).abs() <= window_years)
            .collect()
    }

    /// Extract an entity pin into a structured CML HyperDoc snippet.
    pub fn extract_pin_to_hyperdoc_cml(&self, pin_id: &str) -> Option<String> {
        self.pins.iter().find(|p| p.id == pin_id).map(|p| {
            format!(
                "<q-doc title=\"{}\">\n  \
                 <q-entity id=\"qualia:{}\" lat=\"{:.4}\" lon=\"{:.4}\" epoch=\"{}\">\n    \
                   <q-claim attribution=\"{}\">{}</q-claim>\n  \
                 </q-entity>\n\
                 </q-doc>",
                p.title, p.id, p.lat, p.lon, p.year_epoch, p.cultural_attribution, p.claim_summary
            )
        })
    }
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the Chora 4D Spatio-Temporal Commons Viewport.
pub fn build_chora_view(document: &Document, manager: &ChoraManager) -> Element {
    let root = document.create_element("div").unwrap();
    let root_el: HtmlElement = root.clone().dyn_into().unwrap();
    root_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; padding: 12px; gap: 10px; \
         background: #020617; color: #f8fafc; overflow-y: auto; font-family: sans-serif;",
    );

    // Header Toolbar
    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 8px 12px;",
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some(&format!(
        "\u{1F310} Chora 4D Spatio-Temporal Commons [ Epoch: {} CE ]",
        manager.active_year
    )));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    header.append_child(&title).unwrap();

    let status = document.create_element("span").unwrap();
    status.set_text_content(Some(&format!(
        "GeoSPARQL Pins: {} \u{25CF} Dialectical Claims: {}",
        manager.pins.len(),
        manager.active_claims.len()
    )));
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el
        .style()
        .set_css_text("font-size: 11px; font-family: var(--font-mono); color: #34d399;");
    header.append_child(&status).unwrap();

    root.append_child(&header).unwrap();

    // 2-Column Split: Map & Pins on Left, Dialectical Claims on Right
    let split = document.create_element("div").unwrap();
    let split_el: HtmlElement = split.clone().dyn_into().unwrap();
    split_el
        .style()
        .set_css_text("display: grid; grid-template-columns: 1fr 1fr; gap: 10px;");

    // Left: 4D Pins
    let left = document.create_element("div").unwrap();
    let left_el: HtmlElement = left.clone().dyn_into().unwrap();
    left_el.style().set_css_text("background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 8px;");

    let left_title = document.create_element("span").unwrap();
    left_title.set_text_content(Some("\u{1F4CD} Attributed GeoSPARQL Entity Pins"));
    let left_title_el: HtmlElement = left_title.clone().dyn_into().unwrap();
    left_title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    left.append_child(&left_title).unwrap();

    for pin in &manager.pins {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text("background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.05); border-radius: 6px; padding: 8px; display: flex; flex-direction: column; gap: 4px;");

        let top = document.create_element("div").unwrap();
        let top_el: HtmlElement = top.clone().dyn_into().unwrap();
        top_el.style().set_css_text("display: flex; justify-content: space-between; font-size: 11px; font-weight: 600; color: #f8fafc;");

        let p_title = document.create_element("span").unwrap();
        p_title.set_text_content(Some(&pin.title));
        top.append_child(&p_title).unwrap();

        let epoch = document.create_element("span").unwrap();
        epoch.set_text_content(Some(&format!("{} CE", pin.year_epoch)));
        let epoch_el: HtmlElement = epoch.clone().dyn_into().unwrap();
        epoch_el
            .style()
            .set_css_text("font-size: 10px; font-family: var(--font-mono); color: #fbbf24;");
        top.append_child(&epoch).unwrap();
        card.append_child(&top).unwrap();

        let summary = document.create_element("div").unwrap();
        summary.set_text_content(Some(&pin.claim_summary));
        let summary_el: HtmlElement = summary.clone().dyn_into().unwrap();
        summary_el
            .style()
            .set_css_text("font-size: 11px; color: #cbd5e1;");
        card.append_child(&summary).unwrap();

        let meta = document.create_element("div").unwrap();
        let meta_el: HtmlElement = meta.clone().dyn_into().unwrap();
        meta_el.style().set_css_text("display: flex; justify-content: space-between; font-size: 9px; font-family: var(--font-mono); color: #94a3b8; margin-top: 2px;");

        let att = document.create_element("span").unwrap();
        att.set_text_content(Some(&format!("Attribution: {}", pin.cultural_attribution)));
        meta.append_child(&att).unwrap();

        let trust = document.create_element("span").unwrap();
        trust.set_text_content(Some(&format!("Trust: {:.0}%", pin.trust_weight * 100.0)));
        trust
            .clone()
            .dyn_into::<HtmlElement>()
            .unwrap()
            .style()
            .set_css_text("color: #34d399; font-weight: 700;");
        meta.append_child(&trust).unwrap();

        card.append_child(&meta).unwrap();
        left.append_child(&card).unwrap();
    }
    split.append_child(&left).unwrap();

    // Right: Dialectical Analysis
    let right = document.create_element("div").unwrap();
    let right_el: HtmlElement = right.clone().dyn_into().unwrap();
    right_el.style().set_css_text("background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 8px;");

    let right_title = document.create_element("span").unwrap();
    right_title.set_text_content(Some("\u{2696}\u{FE0F} Dialectical Perspective Synthesis"));
    let right_title_el: HtmlElement = right_title.clone().dyn_into().unwrap();
    right_title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    right.append_child(&right_title).unwrap();

    for claim in &manager.active_claims {
        let claim_box = document.create_element("div").unwrap();
        let claim_box_el: HtmlElement = claim_box.clone().dyn_into().unwrap();
        claim_box_el.style().set_css_text("background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.05); border-radius: 6px; padding: 8px; display: flex; flex-direction: column; gap: 6px;");

        let thesis = document.create_element("div").unwrap();
        thesis.set_text_content(Some(&format!("\u{25B6} Thesis: {}", claim.thesis)));
        let thesis_el: HtmlElement = thesis.clone().dyn_into().unwrap();
        thesis_el
            .style()
            .set_css_text("font-size: 11px; color: #38bdf8;");
        claim_box.append_child(&thesis).unwrap();

        let antithesis = document.create_element("div").unwrap();
        antithesis.set_text_content(Some(&format!("\u{25B6} Antithesis: {}", claim.antithesis)));
        let antithesis_el: HtmlElement = antithesis.clone().dyn_into().unwrap();
        antithesis_el
            .style()
            .set_css_text("font-size: 11px; color: #f472b6;");
        claim_box.append_child(&antithesis).unwrap();

        let synth = document.create_element("div").unwrap();
        synth.set_text_content(Some(&format!(
            "\u{2728} Synthesis: {}",
            claim.synthesis_status
        )));
        let synth_el: HtmlElement = synth.clone().dyn_into().unwrap();
        synth_el.style().set_css_text("font-size: 10px; font-family: var(--font-mono); color: #34d399; background: rgba(0,0,0,0.4); padding: 4px; border-radius: 4px;");
        claim_box.append_child(&synth).unwrap();

        right.append_child(&claim_box).unwrap();
    }
    split.append_child(&right).unwrap();

    root.append_child(&split).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chora_default_state() {
        let mgr = ChoraManager::new();
        assert_eq!(mgr.active_year, 2026);
        assert_eq!(mgr.pins.len(), 2);
        assert_eq!(mgr.active_claims.len(), 1);
    }

    #[test]
    fn test_pins_filtering_by_epoch() {
        let mut mgr = ChoraManager::new();
        mgr.active_year = 2026;
        let modern = mgr.pins_for_epoch(10);
        assert_eq!(modern.len(), 1);
        assert_eq!(modern[0].title, "Modern Hydro-Geological Sensor Array");

        mgr.active_year = 1200;
        let medieval = mgr.pins_for_epoch(50);
        assert_eq!(medieval.len(), 1);
        assert_eq!(medieval[0].title, "Ancient Catchment Water Rights Treaty");
    }

    #[test]
    fn test_extract_pin_to_hyperdoc_cml() {
        let mgr = ChoraManager::new();
        let cml = mgr.extract_pin_to_hyperdoc_cml("pin-001");
        assert!(cml.is_some());
        let doc = cml.unwrap();
        assert!(doc.contains("<q-doc title=\"Ancient Catchment Water Rights Treaty\">"));
        assert!(doc.contains("Commons Water Custodianship"));
    }
}
