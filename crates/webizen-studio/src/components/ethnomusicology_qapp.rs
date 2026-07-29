use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn EthnomusicologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:ethnomusicology".to_string(),
            title: "Ethnomusicology Explorer".to_string()
        }
    }
}
