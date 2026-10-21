use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `SocialWorkQapp` component.
pub fn SocialWorkQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialwork".to_string(),
            title: "Social Work Explorer".to_string()
        }
    }
}
