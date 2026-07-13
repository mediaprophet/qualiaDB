use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn OralHistoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:oralhistory".to_string(),
            title: "Oral History Explorer".to_string()
        }
    }
}
