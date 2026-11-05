use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `ZoologyQapp` component.
pub fn ZoologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:zoology".to_string(),
            title: "Zoology Explorer".to_string()
        }
    }
}
