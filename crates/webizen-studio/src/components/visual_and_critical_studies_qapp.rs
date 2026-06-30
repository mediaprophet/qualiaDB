use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn VisualAndCriticalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:visualandcriticalstudies".to_string(),
            title: "Visual And Critical Studies Explorer".to_string()
        }
    }
}
