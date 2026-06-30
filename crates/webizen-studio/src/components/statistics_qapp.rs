use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn StatisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:statistics".to_string(),
            title: "Statistics Explorer".to_string()
        }
    }
}
