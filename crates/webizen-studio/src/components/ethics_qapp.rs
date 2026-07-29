use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EthicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ethics".to_string(),
            title: "Ethics Explorer".to_string()
        }
    }
}
