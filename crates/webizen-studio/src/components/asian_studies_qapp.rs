use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AsianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:asianstudies".to_string(),
            title: "Asian Studies Explorer".to_string()
        }
    }
}
