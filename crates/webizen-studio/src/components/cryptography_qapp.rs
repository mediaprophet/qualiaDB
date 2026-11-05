use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `CryptographyQapp` component.
pub fn CryptographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:cryptography".to_string(),
            title: "Cryptography Explorer".to_string()
        }
    }
}
