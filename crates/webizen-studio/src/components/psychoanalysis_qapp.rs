use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PsychoanalysisQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:psychoanalysis".to_string(),
            title: "Psychoanalysis Explorer".to_string()
        }
    }
}
