use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn AsianStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:asianstudies".to_string(),
            title: "Asian Studies Explorer".to_string()
        }
    }
}
