use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PoliticalEconomyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:politicaleconomy".to_string(),
            title: "Political Economy Explorer".to_string()
        }
    }
}
