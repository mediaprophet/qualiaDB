use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EcoFeminismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ecofeminism".to_string(),
            title: "Eco Feminism Explorer".to_string()
        }
    }
}
