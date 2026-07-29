use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ChicanoAndLatinoStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:chicanoandlatinostudies".to_string(),
            title: "Chicano And Latino Studies Explorer".to_string()
        }
    }
}
