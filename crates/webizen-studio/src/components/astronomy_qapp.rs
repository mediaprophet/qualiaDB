use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AstronomyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:astronomy".to_string(),
            title: "Astronomy Explorer".to_string()
        }
    }
}
