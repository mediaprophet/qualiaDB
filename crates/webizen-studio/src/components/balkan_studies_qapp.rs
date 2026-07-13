use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn BalkanStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:balkanstudies".to_string(),
            title: "Balkan Studies Explorer".to_string()
        }
    }
}
