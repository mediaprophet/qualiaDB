use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EarthScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:earthscience".to_string(),
            title: "Earth Science Explorer".to_string()
        }
    }
}
