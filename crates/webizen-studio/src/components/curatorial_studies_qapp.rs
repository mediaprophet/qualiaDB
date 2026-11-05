use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `CuratorialStudiesQapp` component.
pub fn CuratorialStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:curatorialstudies".to_string(),
            title: "Curatorial Studies Explorer".to_string()
        }
    }
}
