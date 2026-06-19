use dioxus::prelude::*;

#[component]
pub fn StorageDriverConfig() -> Element {
    let mut driver = use_signal(|| "io_uring".to_string());

    rsx! {
        div { style: "padding: 32px; max-width: 800px; margin: 0 auto; font-family: system-ui, sans-serif;",
            h1 { style: "color: #1f2937;", "Storage Driver Configuration" }
            p { style: "color: #6b7280; margin-bottom: 32px;", "Configure low-level block I/O engines for the 42MB SLG Arena ring buffer." }

            div { style: "display: flex; gap: 16px; margin-bottom: 32px;",
                label { style: "flex: 1; border: 2px solid #3b82f6; border-radius: 8px; padding: 16px; cursor: pointer; transition: all 0.2s; background: transparent;",
                    input { type: "radio", name: "driver", value: "io_uring", checked: driver() == "io_uring", onchange: move |_| driver.set("io_uring".to_string()), style: "display: none;" }
                    div { style: "font-weight: bold; font-size: 18px; color: #1f2937;", "io_uring" }
                    div { style: "font-size: 14px; color: #6b7280; margin-top: 8px;", "Linux modern async I/O. Zero-copy ring buffer integration." }
                }
                label { style: "flex: 1; border: 2px solid #e5e7eb; border-radius: 8px; padding: 16px; cursor: pointer; transition: all 0.2s; background: transparent;",
                    input { type: "radio", name: "driver", value: "direct_io", checked: driver() == "direct_io", onchange: move |_| driver.set("direct_io".to_string()), style: "display: none;" }
                    div { style: "font-weight: bold; font-size: 18px; color: #1f2937;", "O_DIRECT" }
                    div { style: "font-size: 14px; color: #6b7280; margin-top: 8px;", "Legacy fallback. Bypasses kernel page cache." }
                }
            }

            if driver() == "io_uring" {
                div { style: "background: #f8fafc; padding: 24px; border-radius: 8px; border: 1px solid #e2e8f0;",
                    h3 { style: "margin-top: 0; color: #334155;", "io_uring Advanced Settings" }
                    div { style: "display: flex; flex-direction: column; gap: 16px;",
                        div {
                            label { style: "display: block; font-weight: 500; margin-bottom: 8px;", "Queue Depth (SQ/CQ)" }
                            input { type: "number", value: "4096", style: "width: 100%; padding: 8px; border: 1px solid #cbd5e1; border-radius: 4px;" }
                        }
                        div {
                            label { style: "display: flex; align-items: center; gap: 8px; font-weight: 500;",
                                input { type: "checkbox", checked: true }
                                "SQPOLL (Kernel Thread Polling)"
                            }
                            p { style: "font-size: 12px; color: #64748b; margin-top: 4px; margin-left: 24px;", "Warning: Requires elevated privileges / CAP_SYS_NICE." }
                        }
                    }
                }
            }

            button { style: "margin-top: 32px; background: #111827; color: white; border: none; padding: 12px 24px; border-radius: 6px; font-size: 16px; cursor: pointer; transition: background 0.2s;",
                "Save and Restart Engine"
            }
        }
    }
}
