use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn BehavioralEconomicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:behavioraleconomics".to_string(),
            title: "Behavioral Economics Explorer".to_string()
        }
    }
}
