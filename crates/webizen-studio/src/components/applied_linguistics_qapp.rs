use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `AppliedLinguisticsQapp` component.
pub fn AppliedLinguisticsQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:appliedlinguistics".to_string(),
            title: "Applied Linguistics Explorer".to_string()
        }
    }
}
