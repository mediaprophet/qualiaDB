use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn HistoryOfArtAndArchitectureQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:historyofartandarchitecture".to_string(),
            title: "History Of Art And Architecture Explorer".to_string()
        }
    }
}
