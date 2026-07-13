use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn DigitalHumanitiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:digitalhumanities".to_string(),
            title: "Digital Humanities Explorer".to_string()
        }
    }
}
