use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn CriminologyAndCriminalJusticeQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:criminologyandcriminaljustice".to_string(),
            title: "Criminology And Criminal Justice Explorer".to_string()
        }
    }
}
