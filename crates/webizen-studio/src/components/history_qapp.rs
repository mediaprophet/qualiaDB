use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn HistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:history".to_string(),
            title: "History Explorer".to_string()
        }
    }
}
