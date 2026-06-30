use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ArchitecturalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:architecturalstudies".to_string(),
            title: "Architectural Studies Explorer".to_string()
        }
    }
}
