use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn IntellectualHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:intellectualhistory".to_string(),
            title: "Intellectual History Explorer".to_string()
        }
    }
}
