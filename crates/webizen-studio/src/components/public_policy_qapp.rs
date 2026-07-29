use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn PublicPolicyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:publicpolicy".to_string(),
            title: "Public Policy Explorer".to_string()
        }
    }
}
