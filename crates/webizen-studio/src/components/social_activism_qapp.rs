use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SocialActivismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialactivism".to_string(),
            title: "Social Activism Explorer".to_string()
        }
    }
}
