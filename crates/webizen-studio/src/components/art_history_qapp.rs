use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ArtHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:arthistory".to_string(),
            title: "Art History Explorer".to_string()
        }
    }
}
