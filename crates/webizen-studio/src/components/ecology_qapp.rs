use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EcologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ecology".to_string(),
            title: "Ecology Explorer".to_string()
        }
    }
}
