use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SystemsBiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:systemsbiology".to_string(),
            title: "Systems Biology Explorer".to_string()
        }
    }
}
