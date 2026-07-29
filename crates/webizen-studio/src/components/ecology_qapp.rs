use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EcologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ecology".to_string(),
            title: "Ecology Explorer".to_string()
        }
    }
}
