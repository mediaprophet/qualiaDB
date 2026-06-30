use dioxus::prelude::*;
use crate::components::QAppEngine;

#[component]
pub fn SocialAndPoliticalPhilosophyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialandpoliticalphilosophy".to_string(),
            title: "Social And Political Philosophy Explorer".to_string()
        }
    }
}
