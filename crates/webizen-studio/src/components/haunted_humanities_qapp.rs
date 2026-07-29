use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn HauntedHumanitiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:hauntedhumanities".to_string(),
            title: "Haunted Humanities Explorer".to_string()
        }
    }
}
