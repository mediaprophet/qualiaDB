use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Dioxus `PhilosophyOfReligionQapp` component.
pub fn PhilosophyOfReligionQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:philosophyofreligion".to_string(),
            title: "Philosophy Of Religion Explorer".to_string()
        }
    }
}
