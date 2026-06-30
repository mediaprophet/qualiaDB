use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PatristicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:patristics".to_string(),
            title: "Patristics Explorer".to_string()
        }
    }
}
