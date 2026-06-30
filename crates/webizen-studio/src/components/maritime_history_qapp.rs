use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MaritimeHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:maritimehistory".to_string(),
            title: "Maritime History Explorer".to_string()
        }
    }
}
