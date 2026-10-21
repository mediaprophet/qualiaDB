use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `EcoCriticalTheoryQapp` component.
pub fn EcoCriticalTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ecocriticaltheory".to_string(),
            title: "Eco Critical Theory Explorer".to_string()
        }
    }
}
