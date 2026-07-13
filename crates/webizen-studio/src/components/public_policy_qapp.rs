use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn PublicPolicyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:publicpolicy".to_string(),
            title: "Public Policy Explorer".to_string()
        }
    }
}
