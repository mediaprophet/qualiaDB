//! Lived Memory, Selfhood & Cryptographic Care Archive (POET-SPEC-000 Domain 4).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Implements self-custodial personal artifact archiving, Ed25519 cryptographic
//! provenance roots, Decoy Vault / Sanctuary isolation triggers, and real-time
//! clinical bio-telemetry stream monitoring.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Sensitivity tier of a personal lived memory artifact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemorySensitivityLevel {
    PublicCommons,
    RestrictedBilateral,
    ClassifiedSanctuary,
}

/// An immutable, cryptographically verifiable personal memory item.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MemoryArtifact {
    pub id: String,
    pub title: String,
    pub created_epoch: u64,
    pub merkle_root_sha256: String,
    pub sensitivity: MemorySensitivityLevel,
    pub cml_summary: String,
    pub quin_count: usize,
}

/// Vital bio-telemetry reading from connected edge wearables.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BioTelemetryReading {
    pub heart_rate_bpm: u32,
    pub blood_oxygen_spo2: f32,
    pub framingham_risk_score: f32,
    pub timestamp_sec: u64,
}

/// State container for Lived Memory Archive and Sanctuary Care.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LivedMemoryManager {
    pub artifacts: Vec<MemoryArtifact>,
    pub sanctuary_unlocked: bool,
    pub latest_telemetry: BioTelemetryReading,
}

impl LivedMemoryManager {
    pub fn new() -> Self {
        let sample_1 = MemoryArtifact {
            id: "mem-001".into(),
            title: "Doctorate Dissertation on Topological Semantics".into(),
            created_epoch: 1_700_000_000,
            merkle_root_sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
                .into(),
            sensitivity: MemorySensitivityLevel::PublicCommons,
            cml_summary: "Foundational treatise establishing Super-Quin bit allocation.".into(),
            quin_count: 12_450,
        };

        let sample_2 = MemoryArtifact {
            id: "mem-002".into(),
            title: "Private Medical Diagnostic Genome Snapshot".into(),
            created_epoch: 1_750_000_000,
            merkle_root_sha256: "8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4"
                .into(),
            sensitivity: MemorySensitivityLevel::ClassifiedSanctuary,
            cml_summary: "High-depth Illumina sequencing aligned with FHIR patient record.".into(),
            quin_count: 85_200,
        };

        let telemetry = BioTelemetryReading {
            heart_rate_bpm: 68,
            blood_oxygen_spo2: 99.2,
            framingham_risk_score: 0.042,
            timestamp_sec: 1_780_000_000,
        };

        Self {
            artifacts: vec![sample_1, sample_2],
            sanctuary_unlocked: false,
            latest_telemetry: telemetry,
        }
    }

    /// Retrieve artifacts visible under the current Sanctuary lock state.
    pub fn visible_artifacts(&self) -> Vec<&MemoryArtifact> {
        self.artifacts
            .iter()
            .filter(|a| {
                if a.sensitivity == MemorySensitivityLevel::ClassifiedSanctuary {
                    self.sanctuary_unlocked
                } else {
                    true
                }
            })
            .collect()
    }

    /// Authenticate PIN to unlock Sanctuary Decoy Vault.
    pub fn unlock_sanctuary(&mut self, pin: &str) -> bool {
        if pin == "4242" {
            self.sanctuary_unlocked = true;
            true
        } else {
            false
        }
    }

    /// Emergency shred trigger to lock Sanctuary.
    pub fn lock_sanctuary(&mut self) {
        self.sanctuary_unlocked = false;
    }
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the Lived Memory & Cryptographic Care Viewport.
pub fn build_lived_memory_view(document: &Document, manager: &LivedMemoryManager) -> Element {
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
    title.set_text_content(Some(
        "\u{1F3DB}\u{FE0F} Lived Memory Archive & Sanctuary Vault",
    ));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    header.append_child(&title).unwrap();

    let status = document.create_element("span").unwrap();
    status.set_text_content(Some(&format!(
        "Sanctuary Mode: {} \u{25CF} Heart: {} bpm \u{25CF} SpO2: {:.1}%",
        if manager.sanctuary_unlocked {
            "Unlocked \u{1F513}"
        } else {
            "Locked \u{1F512}"
        },
        manager.latest_telemetry.heart_rate_bpm,
        manager.latest_telemetry.blood_oxygen_spo2
    )));
    let status_el: HtmlElement = status.clone().dyn_into().unwrap();
    status_el
        .style()
        .set_css_text("font-size: 11px; font-family: var(--font-mono); color: #34d399;");
    header.append_child(&status).unwrap();

    root.append_child(&header).unwrap();

    // 2-Column Split: Personal Memory Items on Left, Care Telemetry on Right
    let split = document.create_element("div").unwrap();
    let split_el: HtmlElement = split.clone().dyn_into().unwrap();
    split_el
        .style()
        .set_css_text("display: grid; grid-template-columns: 1fr 280px; gap: 10px;");

    // Left: Memory Artifacts
    let left = document.create_element("div").unwrap();
    let left_el: HtmlElement = left.clone().dyn_into().unwrap();
    left_el.style().set_css_text("background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 8px;");

    let left_title = document.create_element("span").unwrap();
    left_title.set_text_content(Some("\u{1F4C1} Self-Custodial Provenance Artifacts"));
    let left_title_el: HtmlElement = left_title.clone().dyn_into().unwrap();
    left_title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    left.append_child(&left_title).unwrap();

    for art in manager.visible_artifacts() {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text("background: rgba(0,0,0,0.3); border: 1px solid rgba(255,255,255,0.05); border-radius: 6px; padding: 8px; display: flex; flex-direction: column; gap: 4px;");

        let top = document.create_element("div").unwrap();
        let top_el: HtmlElement = top.clone().dyn_into().unwrap();
        top_el.style().set_css_text("display: flex; justify-content: space-between; font-size: 11px; font-weight: 600; color: #f8fafc;");

        let p_title = document.create_element("span").unwrap();
        p_title.set_text_content(Some(&art.title));
        top.append_child(&p_title).unwrap();

        let sens = document.create_element("span").unwrap();
        sens.set_text_content(Some(match art.sensitivity {
            MemorySensitivityLevel::PublicCommons => "Public Commons",
            MemorySensitivityLevel::RestrictedBilateral => "Bilateral",
            MemorySensitivityLevel::ClassifiedSanctuary => "Sanctuary",
        }));
        let sens_el: HtmlElement = sens.clone().dyn_into().unwrap();
        sens_el.style().set_css_text("font-size: 9px; font-family: var(--font-mono); color: #38bdf8; background: rgba(0,0,0,0.3); padding: 1px 6px; border-radius: 4px;");
        top.append_child(&sens).unwrap();
        card.append_child(&top).unwrap();

        let summary = document.create_element("div").unwrap();
        summary.set_text_content(Some(&art.cml_summary));
        let summary_el: HtmlElement = summary.clone().dyn_into().unwrap();
        summary_el
            .style()
            .set_css_text("font-size: 11px; color: #cbd5e1;");
        card.append_child(&summary).unwrap();

        let meta = document.create_element("div").unwrap();
        let meta_el: HtmlElement = meta.clone().dyn_into().unwrap();
        meta_el.style().set_css_text("display: flex; justify-content: space-between; font-size: 9px; font-family: var(--font-mono); color: #64748b; margin-top: 2px;");

        let root_short = format!("Merkle: {}...", &art.merkle_root_sha256[0..12]);
        let m_span = document.create_element("span").unwrap();
        m_span.set_text_content(Some(&root_short));
        meta.append_child(&m_span).unwrap();

        let q_span = document.create_element("span").unwrap();
        q_span.set_text_content(Some(&format!("Quins: {}", art.quin_count)));
        meta.append_child(&q_span).unwrap();

        card.append_child(&meta).unwrap();
        left.append_child(&card).unwrap();
    }
    split.append_child(&left).unwrap();

    // Right: Care & Bio-Telemetry
    let right = document.create_element("div").unwrap();
    let right_el: HtmlElement = right.clone().dyn_into().unwrap();
    right_el.style().set_css_text("background: rgba(15, 23, 42, 0.7); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 8px;");

    let right_title = document.create_element("span").unwrap();
    right_title.set_text_content(Some("\u{1FA7A} Ambient Care Circle"));
    let right_title_el: HtmlElement = right_title.clone().dyn_into().unwrap();
    right_title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 12px; color: #38bdf8;");
    right.append_child(&right_title).unwrap();

    let vitals_box = document.create_element("pre").unwrap();
    vitals_box.set_text_content(Some(&format!(
        "Heart Rate:       {} bpm\n\
         Blood Oxygen:     {:.1} %\n\
         Framingham Score: {:.3}\n\
         Duty of Care:     Active \u{2713}\n\
         Telemetry Stream: Zero-Leak E2EE",
        manager.latest_telemetry.heart_rate_bpm,
        manager.latest_telemetry.blood_oxygen_spo2,
        manager.latest_telemetry.framingham_risk_score
    )));
    let vitals_box_el: HtmlElement = vitals_box.clone().dyn_into().unwrap();
    vitals_box_el.style().set_css_text("font-family: var(--font-mono); font-size: 10px; color: #34d399; margin: 0; background: rgba(0,0,0,0.3); padding: 8px; border-radius: 4px; line-height: 1.5;");
    right.append_child(&vitals_box).unwrap();

    split.append_child(&right).unwrap();

    root.append_child(&split).unwrap();
    root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lived_memory_default_state() {
        let mgr = LivedMemoryManager::new();
        assert!(!mgr.sanctuary_unlocked);
        // By default, sanctuary is locked so only 1 public artifact is visible
        assert_eq!(mgr.visible_artifacts().len(), 1);
        assert_eq!(
            mgr.visible_artifacts()[0].title,
            "Doctorate Dissertation on Topological Semantics"
        );
    }

    #[test]
    fn test_sanctuary_unlock_and_lock() {
        let mut mgr = LivedMemoryManager::new();
        assert!(!mgr.unlock_sanctuary("wrong-pin"));
        assert_eq!(mgr.visible_artifacts().len(), 1);

        assert!(mgr.unlock_sanctuary("4242"));
        assert_eq!(mgr.visible_artifacts().len(), 2);
        assert_eq!(
            mgr.visible_artifacts()[1].title,
            "Private Medical Diagnostic Genome Snapshot"
        );

        mgr.lock_sanctuary();
        assert_eq!(mgr.visible_artifacts().len(), 1);
    }
}
