#![allow(non_snake_case)]
use dioxus::prelude::*;
use crate::components::shoelace::*;

#[component]
pub fn NeuroSymbolicChat() -> Element {
    rsx! {
        div { class: "neuro-symbolic-chat-pane flex flex-col h-full",
            SlCard {
                div { slot: "header",
                    "Neuro-Symbolic Chat (Webizen Context)"
                }
                div { class: "flex-1 overflow-y-auto min-h-[300px]",
                    "Awaiting LLM Intent Routing or Literature Ingestion..."
                }
                div { slot: "footer", class: "flex gap-2",
                    SlInput { placeholder: "Query datasets or drop a PDF..." }
                    SlButton { variant: "primary", "Execute" }
                }
            }
        }
    }
}
