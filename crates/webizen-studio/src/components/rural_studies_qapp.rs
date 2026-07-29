use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn RuralStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ruralstudies".to_string(),
            title: "Rural Studies Explorer".to_string()
        }
    }
}
