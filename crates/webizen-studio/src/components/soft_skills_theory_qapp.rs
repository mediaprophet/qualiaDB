use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SoftSkillsTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:softskillstheory".to_string(),
            title: "Soft Skills Theory Explorer".to_string()
        }
    }
}
