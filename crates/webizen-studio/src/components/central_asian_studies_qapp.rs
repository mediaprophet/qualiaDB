use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CentralAsianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:centralasianstudies".to_string(),
            title: "Central Asian Studies Explorer".to_string()
        }
    }
}
