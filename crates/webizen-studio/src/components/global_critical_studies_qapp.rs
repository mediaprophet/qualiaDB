use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn GlobalCriticalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:globalcriticalstudies".to_string(),
            title: "Global Critical Studies Explorer".to_string()
        }
    }
}
