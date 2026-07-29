use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PoliticalScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:politicalscience".to_string(),
            title: "Political Science Explorer".to_string()
        }
    }
}
