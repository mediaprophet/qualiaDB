use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `StudioArtQapp` component.
pub fn StudioArtQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:studioart".to_string(),
            title: "Studio Art Explorer".to_string()
        }
    }
}
