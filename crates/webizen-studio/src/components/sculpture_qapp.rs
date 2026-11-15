use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `SculptureQapp` component.
pub fn SculptureQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sculpture".to_string(),
            title: "Sculpture Explorer".to_string()
        }
    }
}
