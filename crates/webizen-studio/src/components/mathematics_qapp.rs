use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn MathematicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:mathematics".to_string(),
            title: "Mathematics Explorer".to_string()
        }
    }
}
