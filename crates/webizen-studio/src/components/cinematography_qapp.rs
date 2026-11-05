use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `CinematographyQapp` component.
pub fn CinematographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:cinematography".to_string(),
            title: "Cinematography Explorer".to_string()
        }
    }
}
