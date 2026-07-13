use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn FuturesStudiesAndForesightQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:futuresstudiesandforesight".to_string(),
            title: "Futures Studies And Foresight Explorer".to_string()
        }
    }
}
