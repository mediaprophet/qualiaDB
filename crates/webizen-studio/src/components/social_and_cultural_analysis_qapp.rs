use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SocialAndCulturalAnalysisQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialandculturalanalysis".to_string(),
            title: "Social And Cultural Analysis Explorer".to_string()
        }
    }
}
