use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn UrbanEcologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:urbanecology".to_string(),
            title: "Urban Ecology Explorer".to_string()
        }
    }
}
