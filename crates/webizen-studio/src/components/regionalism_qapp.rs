use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn RegionalismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:regionalism".to_string(),
            title: "Regionalism Explorer".to_string()
        }
    }
}
