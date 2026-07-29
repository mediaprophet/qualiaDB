use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SoftSkillsTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:softskillstheory".to_string(),
            title: "Soft Skills Theory Explorer".to_string()
        }
    }
}
