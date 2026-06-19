use dioxus::prelude::*;

#[component]
pub fn SolidLdpBrowser() -> Element {
    let mut current_path = use_signal(|| "solid://alice.q42/profile/".to_string());

    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; padding: 2rem; background: rgba(20, 25, 30, 0.8); backdrop-filter: blur(15px); border-radius: 16px; border: 1px solid rgba(0, 200, 255, 0.2); color: var(--qualia-text);",

            div {
                style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 2rem;",
                h2 {
                    style: "margin: 0; font-family: 'Outfit', sans-serif; font-size: 2rem; color: #00C8FF;",
                    "Solid LDP Browser"
                }
            }

            div {
                style: "display: flex; gap: 0.5rem; margin-bottom: 1.5rem;",
                button { style: "padding: 0.5rem 1rem; background: rgba(255,255,255,0.1); border: none; border-radius: 8px; color: #FFF; cursor: pointer;", "↑ Up" }
                input {
                    style: "flex: 1; padding: 0.8rem; background: rgba(0,0,0,0.5); border: 1px solid rgba(0,200,255,0.3); border-radius: 8px; color: #FFF; font-family: monospace;",
                    value: "{current_path}",
                    oninput: move |e| current_path.set(e.value().clone()),
                }
            }

            div {
                style: "display: grid; grid-template-columns: repeat(auto-fill, minmax(150px, 1fr)); gap: 1rem;",
                // Mock folders
                for name in ["card.ttl", "preferences.json", "public/", "private/"] {
                    div {
                        style: "padding: 1.5rem; background: rgba(255,255,255,0.03); border: 1px solid rgba(255,255,255,0.08); border-radius: 12px; text-align: center; cursor: pointer; transition: transform 0.2s, background 0.2s;",
                        div {
                            style: "font-size: 2rem; margin-bottom: 0.5rem;",
                            if name.ends_with('/') { "📁" } else { "📄" }
                        }
                        div { style: "font-size: 0.9rem; color: #CCC; word-break: break-all;", "{name}" }
                    }
                }
            }
        }
    }
}
