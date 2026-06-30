use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn MathematicalEconomicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:mathematicaleconomics".to_string(),
            title: "Mathematical Economics Explorer".to_string()
        }
    }
}
