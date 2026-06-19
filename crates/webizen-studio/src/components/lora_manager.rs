use dioxus::prelude::*;

#[component]
pub fn LoraManager() -> Element {
    let mut selected_adapter = use_signal(|| None::<usize>);
    let adapters = vec![
        ("Deontic Logic Core", "Active", "128MB", "100%"),
        ("Epistemic Resonator", "Idle", "64MB", "98%"),
        ("Medical Q-Graph", "Training", "256MB", "45%"),
    ];

    rsx! {
        div { style: "display: flex; flex-direction: column; height: 100%; padding: 20px; background: #0f172a; color: #e2e8f0; font-family: sans-serif; gap: 20px;",
            div { style: "display: flex; justify-content: space-between; align-items: center;",
                h2 { style: "margin: 0; font-size: 24px; color: #38bdf8;", "LoRA Topology Manager" }
                button { style: "background: #0ea5e9; color: white; border: none; padding: 8px 16px; border-radius: 6px; cursor: pointer;", "Import Adapter" }
            }
            div { style: "display: grid; grid-template-columns: repeat(3, 1fr); gap: 16px;",
                div { style: "background: #1e293b; padding: 16px; border-radius: 8px; border: 1px solid #334155;",
                    h4 { style: "margin: 0 0 8px 0; color: #94a3b8;", "Total VRAM Allocated" }
                    div { style: "font-size: 28px; font-weight: bold; color: #10b981;", "448 MB" }
                }
                div { style: "background: #1e293b; padding: 16px; border-radius: 8px; border: 1px solid #334155;",
                    h4 { style: "margin: 0 0 8px 0; color: #94a3b8;", "Active Adapters" }
                    div { style: "font-size: 28px; font-weight: bold; color: #8b5cf6;", "1 / 8" }
                }
                div { style: "background: #1e293b; padding: 16px; border-radius: 8px; border: 1px solid #334155;",
                    h4 { style: "margin: 0 0 8px 0; color: #94a3b8;", "Global Training Loss" }
                    div { style: "font-size: 28px; font-weight: bold; color: #f59e0b;", "0.0314" }
                }
            }
            div { style: "flex: 1; background: #1e293b; border-radius: 8px; border: 1px solid #334155; overflow: hidden;",
                table { style: "width: 100%; border-collapse: collapse; text-align: left;",
                    thead { style: "background: #0f172a;",
                        tr {
                            th { style: "padding: 12px 16px; border-bottom: 1px solid #334155;", "Adapter Target" }
                            th { style: "padding: 12px 16px; border-bottom: 1px solid #334155;", "Status" }
                            th { style: "padding: 12px 16px; border-bottom: 1px solid #334155;", "Size" }
                            th { style: "padding: 12px 16px; border-bottom: 1px solid #334155;", "Fidelity" }
                            th { style: "padding: 12px 16px; border-bottom: 1px solid #334155;", "Action" }
                        }
                    }
                    tbody {
                        for (i, (name, status, size, acc)) in adapters.into_iter().enumerate() {
                            tr { style: "border-bottom: 1px solid #334155; transition: background 0.2s;",
                                td { style: "padding: 12px 16px; font-weight: 500;", "{name}" }
                                td { style: "padding: 12px 16px;",
                                    span { style: "padding: 4px 8px; border-radius: 12px; font-size: 12px; background: #1e293b; color: #94a3b8;", "{status}" }
                                }
                                td { style: "padding: 12px 16px; color: #cbd5e1;", "{size}" }
                                td { style: "padding: 12px 16px; color: #cbd5e1;", "{acc}" }
                                td { style: "padding: 12px 16px;",
                                    button { style: "background: transparent; border: 1px solid #475569; color: #e2e8f0; padding: 4px 12px; border-radius: 4px; cursor: pointer;", onclick: move |_| { selected_adapter.set(Some(i)); }, "Configure" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
