use dioxus::prelude::*;

#[component]
pub fn EbpfFilterManager() -> Element {
    let filters = vec![
        ("TCP RST Dropper", "XDP", "Loaded", 14502),
        ("HTTP Rate Limiter", "TC BPF", "Loaded", 894301),
        ("DNS Query Inspector", "Kprobe", "Unloaded", 0),
    ];

    rsx! {
        div { style: "padding: 24px; background: #0f172a; color: #e2e8f0; height: 100vh;",
            h1 { style: "font-size: 24px; color: #f8fafc; display: flex; align-items: center; gap: 8px;",
                span { style: "color: #38bdf8;", "⚡" } "eBPF Filter Manager"
            }
            p { style: "color: #94a3b8;", "Manage high-performance kernel-space network filters directly mapped to Qualia networking lanes." }

            div { style: "margin-top: 24px; display: grid; gap: 16px;",
                for (name, hook, status, hits) in filters {
                    div { style: "background: #1e293b; border: 1px solid #334155; border-radius: 8px; padding: 16px; display: flex; justify-content: space-between; align-items: center;",
                        div {
                            div { style: "font-size: 18px; font-weight: 600; margin-bottom: 4px;", "{name}" }
                            div { style: "font-size: 14px; color: #64748b; display: flex; gap: 16px;",
                                span { style: "background: #0f172a; padding: 2px 8px; border-radius: 4px;", "{hook}" }
                                span { "Hits: {hits}" }
                            }
                        }
                        div {
                            button {
                                style: "background: #ef4444; color: white; border: none; padding: 8px 16px; border-radius: 4px; cursor: pointer; font-weight: 500;",
                                "Load BPF"
                            }
                        }
                    }
                }
            }

            button { style: "margin-top: 24px; width: 100%; background: transparent; border: 2px dashed #475569; color: #94a3b8; padding: 16px; border-radius: 8px; cursor: pointer; font-size: 16px; font-weight: 600; transition: border 0.2s;",
                "+ Compile and Inject New BPF Program"
            }
        }
    }
}
