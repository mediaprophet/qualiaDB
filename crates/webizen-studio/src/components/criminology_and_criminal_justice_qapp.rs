use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn CriminologyAndCriminalJusticeQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criminologyandcriminaljustice".to_string(),
            title: "Criminology And Criminal Justice Explorer".to_string()
        }
    }
}
