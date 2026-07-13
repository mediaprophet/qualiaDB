use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PoliticalScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:politicalscience".to_string(),
            title: "Political Science Explorer".to_string()
        }
    }
}
