use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SocialWorkQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialwork".to_string(),
            title: "Social Work Explorer".to_string()
        }
    }
}
