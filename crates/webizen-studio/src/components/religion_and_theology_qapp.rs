use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn ReligionAndTheologyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:religionandtheology".to_string(),
            title: "Religion And Theology Explorer".to_string()
        }
    }
}
