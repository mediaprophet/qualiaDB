use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `MarineBiologyQapp` component.
pub fn MarineBiologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:marinebiology".to_string(),
            title: "Marine Biology Explorer".to_string()
        }
    }
}
