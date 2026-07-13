use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EnvironmentalDesignQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:environmentaldesign".to_string(),
            title: "Environmental Design Explorer".to_string()
        }
    }
}
