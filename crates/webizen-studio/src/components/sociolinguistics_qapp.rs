use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SociolinguisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sociolinguistics".to_string(),
            title: "Sociolinguistics Explorer".to_string()
        }
    }
}
