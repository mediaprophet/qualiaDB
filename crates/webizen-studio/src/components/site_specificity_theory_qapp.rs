use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SiteSpecificityTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sitespecificitytheory".to_string(),
            title: "Site Specificity Theory Explorer".to_string()
        }
    }
}
