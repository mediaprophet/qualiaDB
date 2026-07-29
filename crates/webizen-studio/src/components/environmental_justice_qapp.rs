use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EnvironmentalJusticeQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:environmentaljustice".to_string(),
            title: "Environmental Justice Explorer".to_string()
        }
    }
}
