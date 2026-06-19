use crate::Route;
use dioxus::prelude::*;

#[component]
pub fn BrowserUnavailable() -> Element {
    rsx! {
        div {
            style: "flex: 1; width: 100%; height: 100%; overflow-y: auto; padding: 2rem; display: flex; align-items: flex-start; justify-content: center;",

            div {
                class: "panel-card",
                style: "width: min(100%, 920px); background: linear-gradient(180deg, rgba(255,255,255,0.20) 0%, rgba(255,255,255,0.12) 100%), var(--qualia-surface); border: 1px solid var(--qualia-border); border-radius: 22px; padding: 1.5rem; backdrop-filter: blur(24px); box-shadow: 0 18px 48px rgba(0,0,0,0.08); display: flex; flex-direction: column; gap: 1.25rem;",

                div {
                    style: "display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; flex-wrap: wrap;",

                    div {
                        style: "display: inline-flex; align-items: center; gap: 0.5rem; background: var(--qualia-accent-glow); border: 1px solid var(--qualia-border); border-radius: 999px; padding: 0.38rem 0.75rem;",
                        sl-icon { "name": "display", style: "font-size: 0.82rem; color: var(--qualia-accent);" }
                        span {
                            style: "font-size: 0.72rem; font-weight: 700; letter-spacing: 0.04em; text-transform: uppercase; color: var(--qualia-accent);",
                            "Native Desktop Host"
                        }
                    }

                    span {
                        style: "font-size: 0.72rem; color: var(--qualia-text-muted); background: rgba(255,255,255,0.22); border: 1px solid var(--qualia-border); border-radius: 999px; padding: 0.32rem 0.7rem;",
                        "Public wasm/demo limitation"
                    }
                }

                div {
                    style: "display: flex; flex-direction: column; gap: 0.7rem;",

                    h1 {
                        style: "margin: 0; font-size: clamp(1.5rem, 2vw, 2rem); font-weight: 700; letter-spacing: -0.03em; color: var(--qualia-text);",
                        "Browser pane unavailable on this surface"
                    }

                    p {
                        style: "margin: 0; font-size: 0.95rem; line-height: 1.7; color: var(--qualia-text-muted);",
                        "The in-app browser is available only inside the native desktop host, where Webizen can attach the embedded webview and local runtime surfaces together."
                    }

                    p {
                        style: "margin: 0; font-size: 0.95rem; line-height: 1.7; color: var(--qualia-text-muted);",
                        "If you are viewing the public wasm/demo build, that hosted surface does not expose the native browser bridge, so this panel stays in fallback mode instead of trying to open the full browser experience."
                    }
                }

                div {
                    style: "display: grid; grid-template-columns: repeat(auto-fit, minmax(220px, 1fr)); gap: 0.9rem;",

                    Link {
                        to: Route::QAppsRoute {},
                        class: "panel-card",
                        style: "text-decoration: none; background: rgba(255,255,255,0.18); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1rem; display: flex; flex-direction: column; gap: 0.45rem; color: inherit;",
                        div {
                            style: "width: 2.25rem; height: 2.25rem; border-radius: 0.8rem; background: var(--qualia-accent-glow); display: flex; align-items: center; justify-content: center;",
                            sl-icon { "name": "grid", style: "font-size: 0.95rem; color: var(--qualia-accent);" }
                        }
                        div { style: "font-size: 0.92rem; font-weight: 700; color: var(--qualia-text);", "QApps" }
                        p {
                            style: "margin: 0; font-size: 0.78rem; line-height: 1.55; color: var(--qualia-text-muted);",
                            "Browse the wider app catalog and jump into surfaces that are fully available in the hosted demo."
                        }
                    }

                    Link {
                        to: Route::ContextStudioRoute {},
                        class: "panel-card",
                        style: "text-decoration: none; background: rgba(255,255,255,0.18); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1rem; display: flex; flex-direction: column; gap: 0.45rem; color: inherit;",
                        div {
                            style: "width: 2.25rem; height: 2.25rem; border-radius: 0.8rem; background: var(--qualia-accent-glow); display: flex; align-items: center; justify-content: center;",
                            sl-icon { "name": "diagram-3", style: "font-size: 0.95rem; color: var(--qualia-accent);" }
                        }
                        div { style: "font-size: 0.92rem; font-weight: 700; color: var(--qualia-text);", "Context Studio" }
                        p {
                            style: "margin: 0; font-size: 0.78rem; line-height: 1.55; color: var(--qualia-text-muted);",
                            "Shift into the semantic workspace to explore graph-native context instead of the embedded browser pane."
                        }
                    }

                    Link {
                        to: Route::StudioRoute {},
                        class: "panel-card",
                        style: "text-decoration: none; background: rgba(255,255,255,0.18); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1rem; display: flex; flex-direction: column; gap: 0.45rem; color: inherit;",
                        div {
                            style: "width: 2.25rem; height: 2.25rem; border-radius: 0.8rem; background: var(--qualia-accent-glow); display: flex; align-items: center; justify-content: center;",
                            sl-icon { "name": "layers", style: "font-size: 0.95rem; color: var(--qualia-accent);" }
                        }
                        div { style: "font-size: 0.92rem; font-weight: 700; color: var(--qualia-text);", "QApp Studio" }
                        p {
                            style: "margin: 0; font-size: 0.78rem; line-height: 1.55; color: var(--qualia-text-muted);",
                            "Open the studio canvas to compose layouts, wire panes, and keep working without the native browser host."
                        }
                    }
                }

                div {
                    style: "display: flex; align-items: flex-start; gap: 0.7rem; background: rgba(255,255,255,0.16); border: 1px solid var(--qualia-border); border-radius: 16px; padding: 1rem;",
                    sl-icon { "name": "info-circle", style: "font-size: 0.95rem; color: var(--qualia-accent); margin-top: 0.1rem; flex-shrink: 0;" }
                    p {
                        style: "margin: 0; font-size: 0.8rem; line-height: 1.6; color: var(--qualia-text-muted);",
                        "For the full in-app browser workflow, launch Webizen inside the native desktop shell. For the public web surface, QApps, Context Studio, and QApp Studio remain the best-supported paths forward."
                    }
                }
            }
        }
    }
}
