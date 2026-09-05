//! Health surfaces: COP records, Semantic Library share, NLP ingest.
//!
//! Conditions are possessions of a Principal (`rdfs:Class`), not owl:Thing.
//! No fabricated lab/vital/score values.

use wasm_bindgen::JsCast;
use web_sys::{Document, Element, HtmlElement, HtmlInputElement};

use super::super::cop_records::{build_count_panel, build_family_panel, CopField};
use super::super::live_invoke;

pub const HEALTH_COUNT_FAMILIES: &[(&str, &str)] = &[
    ("health_condition", "Conditions"),
    ("health_medication", "Medications"),
    ("health_lab", "Lab results"),
    ("health_vital", "Vitals"),
    ("health_document", "Documents"),
    ("health_share", "Disclosures"),
    ("health_safeguard", "Safeguards"),
    ("health_report", "Clinical reports"),
    ("health_note", "Notes"),
];

fn wrap(document: &Document, child: Element) -> Element {
    let wrapper = document.create_element("div").unwrap();
    let wrapper_el: HtmlElement = wrapper.clone().dyn_into().unwrap();
    wrapper_el.style().set_css_text(
        "display: flex; flex-direction: column; flex: 1; overflow: auto; padding: 8px; gap: 8px;",
    );
    wrapper.append_child(&child).unwrap();
    wrapper
}

fn banner(document: &Document, text: &str) -> Element {
    let note = document.create_element("div").unwrap();
    note.set_text_content(Some(text));
    let el: HtmlElement = note.clone().dyn_into().unwrap();
    el.style().set_css_text(
        "font-size: 10px; color: var(--text-muted); font-family: var(--font-mono); \
         border: 1px solid var(--border-subtle); border-radius: 4px; padding: 6px 8px;",
    );
    note
}

fn input_value(root: &Element, selector: &str) -> String {
    root.query_selector(selector)
        .ok()
        .flatten()
        .and_then(|el| el.dyn_into::<HtmlInputElement>().ok())
        .map(|el| el.value())
        .unwrap_or_default()
}

mod clinical;
mod records;

pub use clinical::*;
pub use records::*;

pub fn build_health_overview_view(document: &Document) -> Element {
    let wrapper = wrap(
        document,
        banner(
            document,
            "Health overview is live COP counts. Conditions belong to a Principal (rdfs:Class); they are not owl:Thing. No fabricated scores.",
        ),
    );
    wrapper
        .append_child(&build_count_panel(
            document,
            "Live health family counts. Empty stays 0 until you save a record.",
            HEALTH_COUNT_FAMILIES,
        ))
        .unwrap();
    let vitals = build_family_panel(
        document,
        "health_vital",
        "Enter vitals yourself. ClinicalRisk.* uses these fields; it does not invent them.",
        &[
            CopField {
                key: "age",
                placeholder: "Age (years)",
            },
            CopField {
                key: "sex",
                placeholder: "Sex (male|female)",
            },
            CopField {
                key: "sys_bp",
                placeholder: "Systolic BP (mmHg)",
            },
            CopField {
                key: "chf",
                placeholder: "CHF (true|false)",
            },
            CopField {
                key: "sensitivity",
                placeholder: "Sensitivity (classified|restricted)",
            },
        ],
    );
    vitals
        .append_child(&live_invoke::action_bar(
            document,
            &[
                (
                    "ClinicalRisk.cha2ds2_vasc",
                    "ClinicalRisk.cha2ds2_vasc",
                    serde_json::json!({}),
                ),
                (
                    "ClinicalRisk.framingham",
                    "ClinicalRisk.framingham",
                    serde_json::json!({}),
                ),
            ],
        ))
        .unwrap();
    wrapper.append_child(&vitals).unwrap();
    wrapper
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_count_families_are_unique() {
        let mut names: Vec<_> = HEALTH_COUNT_FAMILIES.iter().map(|(f, _)| *f).collect();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), HEALTH_COUNT_FAMILIES.len());
    }
}
