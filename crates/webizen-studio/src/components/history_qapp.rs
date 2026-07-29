use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn HistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:history".to_string(),
            title: "History Explorer".to_string()
        }
    }
}
