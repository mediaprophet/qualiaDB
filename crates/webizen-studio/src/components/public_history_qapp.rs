use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PublicHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:publichistory".to_string(),
            title: "Public History Explorer".to_string()
        }
    }
}
