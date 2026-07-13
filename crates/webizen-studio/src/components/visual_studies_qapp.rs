use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn VisualStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:visualstudies".to_string(),
            title: "Visual Studies Explorer".to_string()
        }
    }
}
