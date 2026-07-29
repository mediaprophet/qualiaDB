use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EcoFeminismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ecofeminism".to_string(),
            title: "Eco Feminism Explorer".to_string()
        }
    }
}
