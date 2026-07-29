use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EgyptologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:egyptology".to_string(),
            title: "Egyptology Explorer".to_string()
        }
    }
}
