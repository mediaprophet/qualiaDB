use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MeteorologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:meteorology".to_string(),
            title: "Meteorology Explorer".to_string()
        }
    }
}
