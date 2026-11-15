use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `EgyptologyQapp` component.
pub fn EgyptologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:egyptology".to_string(),
            title: "Egyptology Explorer".to_string()
        }
    }
}
