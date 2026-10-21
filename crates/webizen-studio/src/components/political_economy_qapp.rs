use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `PoliticalEconomyQapp` component.
pub fn PoliticalEconomyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:politicaleconomy".to_string(),
            title: "Political Economy Explorer".to_string()
        }
    }
}
