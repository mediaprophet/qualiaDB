use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CryptographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:cryptography".to_string(),
            title: "Cryptography Explorer".to_string()
        }
    }
}
