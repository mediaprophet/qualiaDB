use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `PaleontologyQapp` component.
pub fn PaleontologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:paleontology".to_string(),
            title: "Paleontology Explorer".to_string()
        }
    }
}
