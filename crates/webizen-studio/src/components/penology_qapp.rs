use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `PenologyQapp` component.
pub fn PenologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:penology".to_string(),
            title: "Penology Explorer".to_string()
        }
    }
}
