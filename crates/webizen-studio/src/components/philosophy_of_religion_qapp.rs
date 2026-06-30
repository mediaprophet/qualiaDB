use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PhilosophyOfReligionQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philosophyofreligion".to_string(),
            title: "Philosophy Of Religion Explorer".to_string()
        }
    }
}
