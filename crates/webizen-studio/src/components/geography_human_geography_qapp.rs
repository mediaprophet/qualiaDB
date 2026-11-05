use crate::components::QAppEngine;
use aether::prelude::*;

#[component]
/// Aether `GeographyHumanGeographyQapp` component.
pub fn GeographyHumanGeographyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:geographyhumangeography".to_string(),
            title: "Geography Human Geography Explorer".to_string()
        }
    }
}
