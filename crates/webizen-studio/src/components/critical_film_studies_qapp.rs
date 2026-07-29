use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CriticalFilmStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criticalfilmstudies".to_string(),
            title: "Critical Film Studies Explorer".to_string()
        }
    }
}
