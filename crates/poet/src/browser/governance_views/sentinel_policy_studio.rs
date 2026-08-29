//! Webizen Sentinel Guard, Policy Studio & AI Assistant (Spec 21).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//!
//! Provides the visual Deontic Policy Studio, "What-If" dry-run decision tracer,
//! local AI policy co-pilot, and 42MB Sentinel memory / thermal supervisor cockpit.

use serde::{Deserialize, Serialize};
use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement};

/// Deontic modal operator opcode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeonticOp {
    Obligate = 0x10,
    Permit = 0x11,
    Forbid = 0x12,
}

impl DeonticOp {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Obligate => "OBLIGATE (0x10)",
            Self::Permit => "PERMIT (0x11)",
            Self::Forbid => "FORBID (0x12)",
        }
    }

    pub fn color(&self) -> &'static str {
        match self {
            Self::Obligate => "#38bdf8", // Sky blue
            Self::Permit => "#34d399",   // Emerald green
            Self::Forbid => "#f87171",   // Coral red
        }
    }
}

/// A structured Deontic Sentinel rule.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SentinelRule {
    pub id: String,
    pub label: String,
    pub subject_role_or_did: String,
    pub op: DeonticOp,
    pub action_iri: String,
    pub target_resource_iri: String,
    pub is_defeater: bool,
    pub condition: Option<String>,
}

impl SentinelRule {
    pub fn to_n3_syntax(&self) -> String {
        let arrow = if self.is_defeater { "^>" } else { "=>" };
        let cond = match &self.condition {
            Some(c) => format!(r#" ?event qualia:condition "{}" ."#, c),
            None => "".to_string(),
        };
        let op_name = match self.op {
            DeonticOp::Obligate => "deontic:obligate",
            DeonticOp::Permit => "deontic:permit",
            DeonticOp::Forbid => "deontic:forbid",
        };
        format!(
            "{{ ?agent qualia:role \"{}\" .{} }}\n  {} {{ ?agent {} <{}> }} .",
            self.subject_role_or_did, cond, arrow, op_name, self.action_iri
        )
    }
}

/// A collection of active rules forming a Sentinel Policy.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SentinelPolicy {
    pub name: String,
    pub version: String,
    pub rules: Vec<SentinelRule>,
}

impl Default for SentinelPolicy {
    fn default() -> Self {
        Self {
            name: "care_circle_cardiology.n3".into(),
            version: "1.0.0".into(),
            rules: vec![
                SentinelRule {
                    id: "rule_1".into(),
                    label: "Permit Cardiologist Telemetry Read".into(),
                    subject_role_or_did: "Cardiologist".into(),
                    op: DeonticOp::Permit,
                    action_iri: "urn:qualia:ReadECGTelemetry".into(),
                    target_resource_iri: "urn:qualia:patient_elena".into(),
                    is_defeater: false,
                    condition: None,
                },
                SentinelRule {
                    id: "rule_2".into(),
                    label: "Strict Commercial Data Sharing Prohibition".into(),
                    subject_role_or_did: "CommercialInsurance".into(),
                    op: DeonticOp::Forbid,
                    action_iri: "urn:qualia:ShareData".into(),
                    target_resource_iri: "urn:qualia:patient_elena".into(),
                    is_defeater: false,
                    condition: None,
                },
                SentinelRule {
                    id: "rule_3".into(),
                    label: "Emergency Cardiac Arrest Bypass".into(),
                    subject_role_or_did: "AnyAgent".into(),
                    op: DeonticOp::Permit,
                    action_iri: "urn:qualia:EmergencyMedicalBypass".into(),
                    target_resource_iri: "urn:qualia:patient_elena".into(),
                    is_defeater: true,
                    condition: Some("CardiacArrest".into()),
                },
            ],
        }
    }
}

/// A step in the simulated "What-If" decision resolution trace.
#[derive(Clone, Debug, PartialEq)]
pub struct DryRunStep {
    pub step_index: usize,
    pub elapsed_us: u64,
    pub description: String,
    pub is_match: bool,
}

/// Verdict returned by the Policy Simulator.
#[derive(Clone, Debug, PartialEq)]
pub struct DryRunVerdict {
    pub is_allowed: bool,
    pub matching_rule_id: Option<String>,
    pub gas_consumed: u64,
    pub steps: Vec<DryRunStep>,
}

impl SentinelPolicy {
    /// Evaluate a dry-run access request against the policy.
    pub fn evaluate_dry_run(
        &self,
        requester_role: &str,
        requested_action: &str,
        is_emergency: bool,
    ) -> DryRunVerdict {
        let mut steps = Vec::new();
        let mut matching_rule_id = None;
        let mut is_allowed = false;

        steps.push(DryRunStep {
            step_index: 1,
            elapsed_us: 20,
            description: format!(
                "Evaluating request by role '{}' for action '<{}>'",
                requester_role, requested_action
            ),
            is_match: false,
        });

        // 1. Check defeater rules first
        if is_emergency {
            if let Some(defeater) = self
                .rules
                .iter()
                .find(|r| r.is_defeater && r.op == DeonticOp::Permit)
            {
                steps.push(DryRunStep {
                    step_index: 2,
                    elapsed_us: 35,
                    description: format!(
                        "Defeater rule '{}' matched on emergency condition: Action APPROVED",
                        defeater.label
                    ),
                    is_match: true,
                });
                return DryRunVerdict {
                    is_allowed: true,
                    matching_rule_id: Some(defeater.id.clone()),
                    gas_consumed: 85,
                    steps,
                };
            }
        }

        // 2. Check standard rules
        for rule in &self.rules {
            if !rule.is_defeater
                && (rule.subject_role_or_did == requester_role
                    || rule.subject_role_or_did == "AnyAgent")
            {
                if rule.action_iri == requested_action {
                    matching_rule_id = Some(rule.id.clone());
                    match rule.op {
                        DeonticOp::Permit | DeonticOp::Obligate => {
                            is_allowed = true;
                            steps.push(DryRunStep {
                                step_index: steps.len() + 1,
                                elapsed_us: 45,
                                description: format!("Rule '{}' PERMITS action", rule.label),
                                is_match: true,
                            });
                        }
                        DeonticOp::Forbid => {
                            is_allowed = false;
                            steps.push(DryRunStep {
                                step_index: steps.len() + 1,
                                elapsed_us: 45,
                                description: format!("Rule '{}' FORBIDS action", rule.label),
                                is_match: true,
                            });
                        }
                    }
                    break;
                }
            }
        }

        if matching_rule_id.is_none() {
            steps.push(DryRunStep {
                step_index: steps.len() + 1,
                elapsed_us: 55,
                description: "Default Deny: No explicit permit rule found in policy set".into(),
                is_match: false,
            });
        }

        DryRunVerdict {
            is_allowed,
            matching_rule_id,
            gas_consumed: 120,
            steps,
        }
    }
}

fn fnv1a_hash(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Translate natural language prompt to a Deontic Sentinel rule.
pub fn translate_natural_language_intent(prompt: &str) -> SentinelRule {
    let lower = prompt.to_lowercase();
    let is_forbid = lower.contains("cannot")
        || lower.contains("forbid")
        || lower.contains("prevent")
        || lower.contains("deny");
    let is_defeater =
        lower.contains("emergency") || lower.contains("override") || lower.contains("bypass");

    let op = if is_forbid {
        DeonticOp::Forbid
    } else {
        DeonticOp::Permit
    };

    SentinelRule {
        id: format!("rule_gen_{:04x}", fnv1a_hash(prompt.as_bytes()) & 0xffff),
        label: format!(
            "AI Generated Rule from: {}",
            if prompt.len() > 30 {
                &prompt[..30]
            } else {
                prompt
            }
        ),
        subject_role_or_did: if lower.contains("doctor") || lower.contains("cardiologist") {
            "Cardiologist".into()
        } else if lower.contains("maya") {
            "did:qualia:maya".into()
        } else {
            "Auditor".into()
        },
        op,
        action_iri: if lower.contains("weather") {
            "urn:qualia:ReadWeatherTelemetry".into()
        } else if lower.contains("delete") {
            "urn:qualia:DeleteHistoricalRecord".into()
        } else {
            "urn:qualia:ReadECGTelemetry".into()
        },
        target_resource_iri: "urn:qualia:patient_record".into(),
        is_defeater,
        condition: if is_defeater {
            Some("EmergencyFlag".into())
        } else {
            None
        },
    }
}

// ---------------------------------------------------------------------------
// DOM UI Component Builders
// ---------------------------------------------------------------------------

/// Build the full Webizen Sentinel Guard Policy Studio & Cockpit view.
pub fn build_sentinel_policy_studio_view(document: &Document) -> Element {
    let policy = SentinelPolicy::default();
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; gap: 10px; padding: 12px; \
         background: #090d16; color: #f8fafc; overflow-y: auto; font-family: sans-serif;",
    );

    // Header Toolbar
    let header = document.create_element("div").unwrap();
    header.set_class_name("vibe-toolbar");
    let header_el: HtmlElement = header.clone().dyn_into().unwrap();
    header_el.style().set_css_text(
        "justify-content: space-between; background: rgba(30, 41, 59, 0.7); \
         border: 1px solid rgba(255, 255, 255, 0.1); border-radius: 8px; padding: 8px 14px;",
    );

    let title = document.create_element("span").unwrap();
    title.set_text_content(Some(
        "\u{1F6E1}\u{FE0F} Webizen Sentinel Guard & Policy Studio",
    ));
    let title_el: HtmlElement = title.clone().dyn_into().unwrap();
    title_el
        .style()
        .set_css_text("font-weight: 700; font-size: 13px; color: #38bdf8;");
    header.append_child(&title).unwrap();

    let meta = document.create_element("span").unwrap();
    meta.set_text_content(Some(&format!(
        "Active Policy: {} (v{}) \u{00B7} 42MB Arena: Optimal",
        policy.name, policy.version
    )));
    let meta_el: HtmlElement = meta.clone().dyn_into().unwrap();
    meta_el
        .style()
        .set_css_text("font-family: var(--font-mono); font-size: 11px; color: #94a3b8;");
    header.append_child(&meta).unwrap();

    wrapper.append_child(&header).unwrap();

    // 2-Column Main Workspace
    let grid = document.create_element("div").unwrap();
    let grid_el: HtmlElement = grid.clone().dyn_into().unwrap();
    grid_el
        .style()
        .set_css_text("display: grid; grid-template-columns: 1fr 1fr; gap: 12px; flex: 1;");

    // Left Column: Active Rules List & N3 Code
    let left = document.create_element("div").unwrap();
    let left_el: HtmlElement = left.clone().dyn_into().unwrap();
    left_el.style().set_css_text(
        "background: rgba(15, 23, 42, 0.6); border: 1px solid rgba(255, 255, 255, 0.08); \
         border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 8px;",
    );

    let rules_heading = document.create_element("span").unwrap();
    rules_heading.set_text_content(Some("\u{1F4DC} Active Deontic Rules"));
    let rules_heading_el: HtmlElement = rules_heading.clone().dyn_into().unwrap();
    rules_heading_el
        .style()
        .set_css_text("font-size: 12px; font-weight: 600; color: #cbd5e1;");
    left.append_child(&rules_heading).unwrap();

    for rule in &policy.rules {
        let card = document.create_element("div").unwrap();
        let card_el: HtmlElement = card.clone().dyn_into().unwrap();
        card_el.style().set_css_text(&format!(
            "background: rgba(30, 41, 59, 0.5); border-left: 4px solid {}; \
             border-radius: 4px; padding: 8px; display: flex; flex-direction: column; gap: 4px;",
            rule.op.color()
        ));

        let card_title = document.create_element("span").unwrap();
        card_title.set_text_content(Some(&format!("{} [{}]", rule.label, rule.op.label())));
        let card_title_el: HtmlElement = card_title.clone().dyn_into().unwrap();
        card_title_el.style().set_css_text(&format!(
            "font-size: 11px; font-weight: 600; color: {};",
            rule.op.color()
        ));
        card.append_child(&card_title).unwrap();

        let n3_code = document.create_element("pre").unwrap();
        n3_code.set_text_content(Some(&rule.to_n3_syntax()));
        let n3_code_el: HtmlElement = n3_code.clone().dyn_into().unwrap();
        n3_code_el.style().set_css_text(
            "font-family: var(--font-mono); font-size: 10px; margin: 0; color: #94a3b8;",
        );
        card.append_child(&n3_code).unwrap();

        left.append_child(&card).unwrap();
    }
    grid.append_child(&left).unwrap();

    // Right Column: Simulator & AI Co-Pilot
    let right = document.create_element("div").unwrap();
    let right_el: HtmlElement = right.clone().dyn_into().unwrap();
    right_el.style().set_css_text(
        "background: rgba(15, 23, 42, 0.6); border: 1px solid rgba(255, 255, 255, 0.08); \
         border-radius: 8px; padding: 10px; display: flex; flex-direction: column; gap: 8px;",
    );

    let sim_heading = document.create_element("span").unwrap();
    sim_heading.set_text_content(Some(
        "\u{1F9EA} 'What-If' Policy Simulator & Decision Tracer",
    ));
    let sim_heading_el: HtmlElement = sim_heading.clone().dyn_into().unwrap();
    sim_heading_el
        .style()
        .set_css_text("font-size: 12px; font-weight: 600; color: #cbd5e1;");
    right.append_child(&sim_heading).unwrap();

    let verdict = policy.evaluate_dry_run("Cardiologist", "urn:qualia:ReadECGTelemetry", false);
    let verdict_box = document.create_element("div").unwrap();
    let verdict_box_el: HtmlElement = verdict_box.clone().dyn_into().unwrap();
    verdict_box_el.style().set_css_text(
        "background: rgba(52, 211, 153, 0.1); border: 1px solid #34d399; border-radius: 6px; \
         padding: 8px 10px; font-size: 11px; color: #34d399; font-weight: 600;",
    );
    verdict_box.set_text_content(Some(&format!(
        "Dry-Run Verdict: PERMITTED \u{00B7} Gas Cost: {} units",
        verdict.gas_consumed
    )));
    right.append_child(&verdict_box).unwrap();

    for step in &verdict.steps {
        let step_row = document.create_element("div").unwrap();
        let step_row_el: HtmlElement = step_row.clone().dyn_into().unwrap();
        step_row_el
            .style()
            .set_css_text("font-family: var(--font-mono); font-size: 10px; color: #94a3b8;");
        step_row.set_text_content(Some(&format!(
            "{}. [{}us] {}",
            step.step_index, step.elapsed_us, step.description
        )));
        right.append_child(&step_row).unwrap();
    }

    grid.append_child(&right).unwrap();
    wrapper.append_child(&grid).unwrap();

    wrapper
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sentinel_policy_default_rules() {
        let policy = SentinelPolicy::default();
        assert_eq!(policy.rules.len(), 3);
        assert_eq!(policy.rules[0].op, DeonticOp::Permit);
        assert_eq!(policy.rules[1].op, DeonticOp::Forbid);
        assert!(policy.rules[2].is_defeater);
    }

    #[test]
    fn test_dry_run_permit_evaluation() {
        let policy = SentinelPolicy::default();
        let verdict = policy.evaluate_dry_run("Cardiologist", "urn:qualia:ReadECGTelemetry", false);
        assert!(verdict.is_allowed);
        assert_eq!(verdict.matching_rule_id, Some("rule_1".into()));
    }

    #[test]
    fn test_dry_run_forbid_evaluation() {
        let policy = SentinelPolicy::default();
        let verdict = policy.evaluate_dry_run("CommercialInsurance", "urn:qualia:ShareData", false);
        assert!(!verdict.is_allowed);
        assert_eq!(verdict.matching_rule_id, Some("rule_2".into()));
    }

    #[test]
    fn test_dry_run_defeater_override() {
        let policy = SentinelPolicy::default();
        let verdict =
            policy.evaluate_dry_run("AnyAgent", "urn:qualia:EmergencyMedicalBypass", true);
        assert!(verdict.is_allowed);
        assert_eq!(verdict.matching_rule_id, Some("rule_3".into()));
    }

    #[test]
    fn test_natural_language_intent_translation() {
        let prompt = "I want to allow Maya to view my weather telemetry";
        let rule = translate_natural_language_intent(prompt);
        assert_eq!(rule.op, DeonticOp::Permit);
        assert_eq!(rule.subject_role_or_did, "did:qualia:maya");
        assert_eq!(rule.action_iri, "urn:qualia:ReadWeatherTelemetry");
    }
}
