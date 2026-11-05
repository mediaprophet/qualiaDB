use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `PublicHealthQapp` component.
pub fn PublicHealthQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:publichealth".to_string(),
            title: "Public Health Explorer".to_string()
        }
    }
}
