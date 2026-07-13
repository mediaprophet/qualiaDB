use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PovertyAndInequalityStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:povertyandinequalitystudies".to_string(),
            title: "Poverty And Inequality Studies Explorer".to_string()
        }
    }
}
