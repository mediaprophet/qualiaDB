use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn UtopianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:utopianstudies".to_string(),
            title: "Utopian Studies Explorer".to_string()
        }
    }
}
