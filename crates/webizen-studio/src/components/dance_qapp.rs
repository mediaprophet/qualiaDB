use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn DanceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:dance".to_string(),
            title: "Dance Explorer".to_string()
        }
    }
}
