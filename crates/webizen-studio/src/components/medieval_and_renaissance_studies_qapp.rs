use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MedievalAndRenaissanceStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:medievalandrenaissancestudies".to_string(),
            title: "Medieval And Renaissance Studies Explorer".to_string()
        }
    }
}
