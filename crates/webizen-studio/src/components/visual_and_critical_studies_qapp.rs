use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn VisualAndCriticalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:visualandcriticalstudies".to_string(),
            title: "Visual And Critical Studies Explorer".to_string()
        }
    }
}
