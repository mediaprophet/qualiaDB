use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn BehavioralEconomicsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:behavioraleconomics".to_string(),
            title: "Behavioral Economics Explorer".to_string()
        }
    }
}
