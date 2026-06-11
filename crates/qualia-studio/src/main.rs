#![allow(non_snake_case)]

mod pane_registry;
mod studio_canvas;

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use studio_canvas::DynamicPage;

fn main() {
    // Launch the Dioxus app on the web
    dioxus::launch(App);
}

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/:path..")]
    DynamicPage { path: Vec<String> },
}

#[component]
fn App() -> Element {
    rsx! {
        div {
            style: "width: 100vw; height: 100vh; display: flex; flex-direction: column; background-color: var(--qualia-bg, #000); color: var(--qualia-text, #fff);",
            // Header
            div {
                style: "padding: 1rem; border-bottom: 1px solid var(--qualia-border, #333);",
                h1 { "Webizen Studio (Fiduciary HUD)" }
            }
            // Main Routing Canvas
            Router::<Route> {}
        }
    }
}
