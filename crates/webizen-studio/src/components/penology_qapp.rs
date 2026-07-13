use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PenologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:penology".to_string(),
            title: "Penology Explorer".to_string()
        }
    }
}
