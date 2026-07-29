use crate::components::QAppEngine;
use dioxus::prelude::*;

#[component]
pub fn SocialAndPoliticalPhilosophyQapp() -> Element {
    rsx! {
        QAppEngine {
            ontology_id: "urn:ontology:socialandpoliticalphilosophy".to_string(),
            title: "Social And Political Philosophy Explorer".to_string()
        }
    }
}
