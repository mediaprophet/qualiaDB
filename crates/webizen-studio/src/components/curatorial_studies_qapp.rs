use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CuratorialStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:curatorialstudies".to_string(),
            title: "Curatorial Studies Explorer".to_string()
        }
    }
}
