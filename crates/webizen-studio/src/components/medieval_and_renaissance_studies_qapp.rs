use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MedievalAndRenaissanceStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:medievalandrenaissancestudies".to_string(),
            title: "Medieval And Renaissance Studies Explorer".to_string()
        }
    }
}
