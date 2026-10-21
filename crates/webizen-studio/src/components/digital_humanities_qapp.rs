use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `DigitalHumanitiesQapp` component.
pub fn DigitalHumanitiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:digitalhumanities".to_string(),
            title: "Digital Humanities Explorer".to_string()
        }
    }
}
