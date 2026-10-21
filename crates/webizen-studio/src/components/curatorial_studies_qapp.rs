use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `CuratorialStudiesQapp` component.
pub fn CuratorialStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:curatorialstudies".to_string(),
            title: "Curatorial Studies Explorer".to_string()
        }
    }
}
