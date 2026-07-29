use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn BookArtsAndPapermakingQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:bookartsandpapermaking".to_string(),
            title: "Book Arts And Papermaking Explorer".to_string()
        }
    }
}
