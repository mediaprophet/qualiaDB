use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn AreaAndRegionalStudiesQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:areaandregionalstudies".to_string(),
            title: "Area And Regional Studies Explorer".to_string()
        }
    }
}
