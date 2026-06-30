use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn GlobalCriticalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:globalcriticalstudies".to_string(),
            title: "Global Critical Studies Explorer".to_string()
        }
    }
}
