use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `StructuralismQapp` component.
pub fn StructuralismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:structuralism".to_string(),
            title: "Structuralism Explorer".to_string()
        }
    }
}
