use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CriticalGentrificationStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticalgentrificationstudies".to_string(),
            title: "Critical Gentrification Studies Explorer".to_string()
        }
    }
}
