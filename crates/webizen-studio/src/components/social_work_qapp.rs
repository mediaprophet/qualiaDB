use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `SocialWorkQapp` component.
pub fn SocialWorkQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialwork".to_string(),
            title: "Social Work Explorer".to_string()
        }
    }
}
