use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn UrbanEcologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:urbanecology".to_string(),
            title: "Urban Ecology Explorer".to_string()
        }
    }
}
