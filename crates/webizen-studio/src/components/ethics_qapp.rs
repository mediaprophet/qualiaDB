use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `EthicsQapp` component.
pub fn EthicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ethics".to_string(),
            title: "Ethics Explorer".to_string()
        }
    }
}
