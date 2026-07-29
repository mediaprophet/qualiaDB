use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ArtConservationQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:artconservation".to_string(),
            title: "Art Conservation Explorer".to_string()
        }
    }
}
