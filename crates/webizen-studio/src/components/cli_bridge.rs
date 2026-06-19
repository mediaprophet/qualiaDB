use dioxus::prelude::*;

#[component]
pub fn CliBridge() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; height: 100vh; background: #1e1e1e; font-family: 'Fira Code', monospace;",
            div { style: "background: #333; padding: 10px; display: flex; gap: 8px; align-items: center;",
                div { style: "width: 12px; height: 12px; border-radius: 50%; background: #ff5f56;" }
                div { style: "width: 12px; height: 12px; border-radius: 50%; background: #ffbd2e;" }
                div { style: "width: 12px; height: 12px; border-radius: 50%; background: #27c93f;" }
                span { style: "color: #aaa; font-size: 14px; margin-left: 12px;", "qualia-cli (Bridge Mode)" }
            }
            div { style: "flex: 1; padding: 20px; overflow-y: auto; color: #d4d4d4;",
                div { style: "margin-bottom: 8px;", span { style: "color: #569cd6;", "qualia" } " node start --mode=bridge --arena-size=42MB" }
                div { style: "color: #6a9955; margin-bottom: 8px;", "[INFO] Initializing SLG Arena at /dev/q42/vol0..." }
                div { style: "color: #6a9955; margin-bottom: 8px;", "[INFO] Mounting LTL Temporal Engine..." }
                div { style: "margin-bottom: 8px;", span { style: "color: #ce9178;", "Connection established to Webizen UI." } }
                div { style: "margin-bottom: 8px;", span { style: "color: #569cd6;", "qualia" } " p2p swarm status" }
                div { style: "margin-bottom: 8px;", "Connected to 42 peers." }

                div { style: "display: flex; align-items: center; margin-top: 16px;",
                    span { style: "color: #dcdcaa; margin-right: 8px;", "C:\\Projects\\qualia>" }
                    input { style: "flex: 1; background: transparent; border: none; color: #d4d4d4; font-family: 'Fira Code', monospace; outline: none; font-size: 16px;", autofocus: true }
                }
            }
        }
    }
}
