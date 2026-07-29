use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ChemistryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:chemistry".to_string(),
            title: "Chemistry Explorer".to_string()
        }
    }
}
