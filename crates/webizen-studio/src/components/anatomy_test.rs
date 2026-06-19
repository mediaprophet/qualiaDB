use dioxus::prelude::*;

/// Anatomy Project stress test component
///
/// Invokes CCF asset loading tests and displays memory profiling results
/// Validates zero-heap binary IPC infrastructure with heavy biomedical assets
#[component]
pub fn AnatomyTest() -> Element {
    let mut test_result = use_signal(|| String::new());
    let mut is_running = use_signal(|| false);

    let run_ipc_handshake = move |_| {
        is_running.set(true);
        test_result.set("Running IPC handshake test...".to_string());

        // TODO: Implement Tauri command invocation
        #[cfg(not(target_arch = "wasm32"))]
        {
            // Tauri invocation will be added here
            test_result.set("IPC handshake: Tauri invocation pending".to_string());
            is_running.set(false);
        }

        #[cfg(target_arch = "wasm32")]
        {
            test_result.set("IPC handshake test: Not available in WASM environment".to_string());
            is_running.set(false);
        }
    };

    let run_larynx_smoke = move |_| {
        is_running.set(true);
        test_result.set("Running Larynx smoke test (335KB)...".to_string());

        #[cfg(not(target_arch = "wasm32"))]
        {
            test_result.set("Larynx smoke test: Tauri invocation pending".to_string());
            is_running.set(false);
        }

        #[cfg(target_arch = "wasm32")]
        {
            test_result.set("Larynx smoke test: Not available in WASM environment".to_string());
            is_running.set(false);
        }
    };

    let run_vasculature_stress = move |_| {
        is_running.set(true);
        test_result
            .set("Running Vasculature stress test (18MB) with memory profiling...".to_string());

        #[cfg(not(target_arch = "wasm32"))]
        {
            test_result.set("Vasculature stress test: Tauri invocation pending".to_string());
            is_running.set(false);
        }

        #[cfg(target_arch = "wasm32")]
        {
            test_result
                .set("Vasculature stress test: Not available in WASM environment".to_string());
            is_running.set(false);
        }
    };

    rsx! {
        div {
            style: "display: flex; flex-direction: column; gap: 16px; padding: 16px; background: rgba(0, 0, 0, 0.5); border-radius: 8px; max-width: 800px; margin: 20px;",

            h2 {
                style: "color: #ffffff; margin: 0; font-size: 18px; font-weight: 600;",
                "Anatomy Project Stress Tests"
            }

            p {
                style: "color: #aaaaaa; margin: 0; font-size: 13px;",
                "Validate zero-heap binary IPC infrastructure with CCF biomedical assets"
            }

            div {
                style: "display: flex; gap: 8px; flex-wrap: wrap;",

                button {
                    onclick: run_ipc_handshake,
                    disabled: is_running(),
                    style: "padding: 8px 16px; background: #4444ff; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 13px;",
                    "IPC Handshake"
                }

                button {
                    onclick: run_larynx_smoke,
                    disabled: is_running(),
                    style: "padding: 8px 16px; background: #44aa44; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 13px;",
                    "Larynx (335KB)"
                }

                button {
                    onclick: run_vasculature_stress,
                    disabled: is_running(),
                    style: "padding: 8px 16px; background: #ff4444; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 13px;",
                    "Vasculature (18MB)"
                }
            }

            if !test_result().is_empty() {
                div {
                    style: "padding: 12px; background: rgba(0, 0, 0, 0.3); border-radius: 4px; font-family: monospace; font-size: 12px; color: #00ff00; white-space: pre-wrap; word-break: break-all;",
                    "{test_result()}"
                }
            }

            if is_running() {
                div {
                    style: "color: #ffaa00; font-size: 13px;",
                    "Test running..."
                }
            }
        }
    }
}
