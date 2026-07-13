use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CriticalFilmStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticalfilmstudies".to_string(),
            title: "Critical Film Studies Explorer".to_string()
        }
    }
}
