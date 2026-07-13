use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn ReligionAndTheologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:religionandtheology".to_string(),
            title: "Religion And Theology Explorer".to_string()
        }
    }
}
