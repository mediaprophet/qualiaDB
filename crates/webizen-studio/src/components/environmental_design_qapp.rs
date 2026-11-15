use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `EnvironmentalDesignQapp` component.
pub fn EnvironmentalDesignQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:environmentaldesign".to_string(),
            title: "Environmental Design Explorer".to_string()
        }
    }
}
