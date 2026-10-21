use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
/// Dioxus `PublicPolicyQapp` component.
pub fn PublicPolicyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:publicpolicy".to_string(),
            title: "Public Policy Explorer".to_string()
        }
    }
}
