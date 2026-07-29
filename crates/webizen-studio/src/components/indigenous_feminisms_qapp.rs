use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn IndigenousFeminismsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:indigenousfeminisms".to_string(),
            title: "Indigenous Feminisms Explorer".to_string()
        }
    }
}
