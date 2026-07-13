use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SiteSpecificityTheoryQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:sitespecificitytheory".to_string(),
            title: "Site Specificity Theory Explorer".to_string()
        }
    }
}
