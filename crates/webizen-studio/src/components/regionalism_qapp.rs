use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn RegionalismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:regionalism".to_string(),
            title: "Regionalism Explorer".to_string()
        }
    }
}
