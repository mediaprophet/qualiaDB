use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AstrophysicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:astrophysics".to_string(),
            title: "Astrophysics Explorer".to_string()
        }
    }
}
