use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn EthnomusicologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ethnomusicology".to_string(),
            title: "Ethnomusicology Explorer".to_string()
        }
    }
}
