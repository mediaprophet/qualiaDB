use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CinematographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:cinematography".to_string(),
            title: "Cinematography Explorer".to_string()
        }
    }
}
