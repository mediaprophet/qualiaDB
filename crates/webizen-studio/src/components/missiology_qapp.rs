use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MissiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:missiology".to_string(),
            title: "Missiology Explorer".to_string()
        }
    }
}
