use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SocialActivismQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialactivism".to_string(),
            title: "Social Activism Explorer".to_string()
        }
    }
}
