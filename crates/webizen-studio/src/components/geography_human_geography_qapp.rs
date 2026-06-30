use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn GeographyHumanGeographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:geographyhumangeography".to_string(),
            title: "Geography Human Geography Explorer".to_string()
        }
    }
}
