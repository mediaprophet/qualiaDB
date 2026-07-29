use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PublicHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:publichistory".to_string(),
            title: "Public History Explorer".to_string()
        }
    }
}
