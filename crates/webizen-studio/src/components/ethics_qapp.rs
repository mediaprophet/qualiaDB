use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EthicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ethics".to_string(),
            title: "Ethics Explorer".to_string()
        }
    }
}
