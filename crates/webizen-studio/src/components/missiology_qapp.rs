use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MissiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:missiology".to_string(),
            title: "Missiology Explorer".to_string()
        }
    }
}
