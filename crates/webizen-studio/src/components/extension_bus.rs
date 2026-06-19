use dioxus::prelude::*;

#[component]
pub fn ExtensionBus() -> Element {
    let mut extensions = use_signal(|| {
        vec![
            ("Python ML Bindings", "Running", "12.4 MB"),
            ("React DevTools Protocol", "Idle", "1.2 MB"),
            ("Custom Graph Visualizer", "Crashed", "0 MB"),
        ]
    });

    rsx! {
        div { style: "padding: 24px; background: #fff; color: #333; height: 100vh; font-family: sans-serif;",
            h1 { style: "margin-top: 0; color: #e11d48; border-bottom: 2px solid #ffe4e6; padding-bottom: 12px;", "Native Extension Bus" }
            p { style: "color: #64748b;", "Monitor and control dynamically loaded libraries (DLL/SO) attached to the Webizen engine." }

            div { style: "margin-top: 24px; display: grid; gap: 16px; grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));",
                for (i, (name, status, mem)) in extensions.read().iter().enumerate() {
                    div { style: "border: 1px solid #e2e8f0; border-radius: 8px; padding: 20px; box-shadow: 0 4px 6px -1px rgba(0,0,0,0.05); position: relative; overflow: hidden;",
                        div { style: "position: absolute; top: 0; left: 0; width: 4px; height: 100%; background: var(--qualia-surface);" }

                        h3 { style: "margin: 0 0 12px 0; font-size: 18px;", "{name}" }
                        div { style: "display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;",
                            span { style: "font-size: 14px; color: #64748b;", "Status: ", b { style: "color: #10b981;", "{status}" } }
                            span { style: "font-size: 14px; color: #64748b;", "Mem: {mem}" }
                        }
                        div { style: "display: flex; gap: 8px;",
                            if *status == "Running" {
                                button { style: "flex: 1; padding: 8px; border-radius: 4px; border: 1px solid #e2e8f0; background: #fff; cursor: pointer; color: #ef4444; font-weight: 500;", onclick: move |_| { extensions.write()[i].1 = "Idle"; }, "Terminate" }
                            } else {
                                button { style: "flex: 1; padding: 8px; border-radius: 4px; border: 1px solid #e2e8f0; background: #fff; cursor: pointer; color: #10b981; font-weight: 500;", onclick: move |_| { extensions.write()[i].1 = "Running"; }, "Load" }
                            }
                        }
                    }
                }
            }

            button { style: "position: fixed; bottom: 32px; right: 32px; width: 64px; height: 64px; border-radius: 32px; background: #e11d48; color: white; border: none; font-size: 32px; cursor: pointer; box-shadow: 0 10px 15px -3px rgba(225, 29, 72, 0.4); display: flex; justify-content: center; align-items: center;",
                "+"
            }
        }
    }
}
