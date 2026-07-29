use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn JournalismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:journalism".to_string(),
            title: "Journalism Explorer".to_string()
        }
    }
}
