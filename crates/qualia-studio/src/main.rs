#![allow(non_snake_case)]

mod pane_registry;
mod studio_canvas;
pub mod components;

use dioxus::prelude::*;
use studio_canvas::DynamicPage;

fn main() {
    dioxus::launch(App);
}

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    // Fallback root path
    #[route("/")]
    Home {},
    // Dynamic canvas paths
    #[route("/:..path")]
    DynamicPage { path: Vec<String> },
}

#[component]
fn Home() -> Element {
    rsx! {
        DynamicPage { path: vec![] }
    }
}

const SHOELACE_CSS: &str = "https://cdn.jsdelivr.net/npm/@shoelace-style/shoelace@2.15.0/cdn/themes/dark.css";
const SHOELACE_JS: &str = "https://cdn.jsdelivr.net/npm/@shoelace-style/shoelace@2.15.0/cdn/shoelace-autoloader.js";
const INTER_FONT: &str = "https://fonts.googleapis.com/css2?family=Inter:wght@300;400;500;600;700&display=swap";

#[component]
fn App() -> Element {
    rsx! {
        // Inject Shoelace & Fonts
        document::Link { rel: "stylesheet", href: SHOELACE_CSS }
        document::Link { rel: "stylesheet", href: INTER_FONT }
        document::Script { type: "module", src: SHOELACE_JS }
        
        // Premium Global Styles
        document::Style {
            "
            :root {{
                --qualia-bg: #09090b;
                --qualia-surface: rgba(24, 24, 27, 0.7);
                --qualia-border: rgba(63, 63, 70, 0.5);
                --qualia-text: #f4f4f5;
                --qualia-text-muted: #a1a1aa;
                --qualia-accent: #06b6d4;
                --qualia-accent-glow: rgba(6, 182, 212, 0.2);
            }}
            body {{
                margin: 0;
                padding: 0;
                font-family: 'Inter', sans-serif;
                background-color: var(--qualia-bg);
                color: var(--qualia-text);
                overflow: hidden;
            }}
            /* Micro-animations */
            .nav-link {{ transition: all 0.2s ease; }}
            .nav-link:hover {{ background: rgba(255,255,255,0.05); transform: translateX(4px); color: var(--qualia-accent); }}
            "
        }

        div {
            style: "width: 100vw; height: 100vh; display: flex; flex-direction: column; background: radial-gradient(circle at top, #1a1a24 0%, var(--qualia-bg) 50%);",
            // Glassmorphism Header
            div {
                style: "padding: 1rem 1.5rem; border-bottom: 1px solid var(--qualia-border); backdrop-filter: blur(12px); background: rgba(9, 9, 11, 0.6); display: flex; align-items: center; justify-content: space-between; box-shadow: 0 4px 24px var(--qualia-accent-glow); z-index: 10;",
                h1 { 
                    style: "margin: 0; font-weight: 600; font-size: 1.25rem; letter-spacing: -0.02em; display: flex; align-items: center; gap: 0.5rem;",
                    span { style: "color: var(--qualia-accent);", "⬡" }
                    "Qualia Studio "
                    span { style: "font-size: 0.85rem; font-weight: 400; color: var(--qualia-text-muted);", "(Fiduciary HUD)" }
                }
                div {
                    style: "display: flex; gap: 1rem; align-items: center;",
                    span { style: "font-size: 0.85rem; color: var(--qualia-text-muted);", "Human-Centric Edge Node: Active" }
                    div { style: "width: 8px; height: 8px; border-radius: 50%; background: #10b981; box-shadow: 0 0 8px #10b981;" }
                }
            }
            // Main Routing Canvas
            Router::<Route> {}
        }
    }
}
