use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EarthScienceQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:earthscience".to_string(),
            title: "Earth Science Explorer".to_string()
        }
    }
}
