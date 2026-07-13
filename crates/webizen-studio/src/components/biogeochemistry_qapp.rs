use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn BiogeochemistryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:biogeochemistry".to_string(),
            title: "Biogeochemistry Explorer".to_string()
        }
    }
}
