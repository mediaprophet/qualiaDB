use std::fs;

fn main() {
    let mut content = fs::read_to_string("crates/qualia-client-core/src/api.rs").unwrap();

    // In functions that need state, but didn't get it, we need to add the state init.
    // Also we need to replace `app.` and `window.` and `app_handle` usages.

    // 1. Add state initialization to missing functions
    let missing_state_funcs = vec![
        "ingest_image_async", "toggle_nym_relay", "toggle_stark_prover",
        "update_solar_input", "fetch_torrent_telemetry", "discover_models",
        "get_active_model", "set_active_model", "get_active_downloads",
        "launch_installed_app", "run_engine_command", "generate_front_door",
        "add_directory_actor", "add_delegation_rule"
    ];

    for fn_name in missing_state_funcs {
        let pattern1 = format!("pub fn {}(", fn_name);
        let pattern2 = format!("pub async fn {}(", fn_name);
        
        let mut new_content = String::new();
        let mut lines = content.lines().peekable();
        while let Some(line) = lines.next() {
            new_content.push_str(line);
            new_content.push('\n');
            if line.contains(&pattern1) || line.contains(&pattern2) {
                if !line.ends_with("{") {
                    while let Some(l) = lines.next() {
                        new_content.push_str(l);
                        new_content.push('\n');
                        if l.ends_with("{") { break; }
                    }
                }
                new_content.push_str("    let state = crate::state::APP_STATE.get().unwrap();\n");
            }
        }
        content = new_content;
    }

    // 2. Remove references to `app.` and `window.`
    // E.g. let _ = window_clone.emit("nym-telemetry", ...);
    // Since we don't have tauri, we can just comment these out or ignore them.
    content = content.replace("let _ = window_clone.emit(\"nym-telemetry\",", "// let _ = window_clone.emit(\"nym-telemetry\",");
    content = content.replace("let _ = window_clone.emit(\"stark-telemetry\",", "// let _ = window_clone.emit(\"stark-telemetry\",");
    
    // app.exit(0)
    content = content.replace("app.exit(0);", "std::process::exit(0);");

    // if let Some(window) = app.get_window("main") {
    content = content.replace("if let Some(window) = app.get_window(\"main\") {", "if false {");

    // 3. Remove window_clone
    content = content.replace("let window_clone = window.clone();", "");

    // 4. In launch_installed_app, it uses `tauri::WindowBuilder`
    // We'll stub this out entirely.
    let window_builder_str = "tauri::WindowBuilder::new(
        &app_handle,
        window_label,
        tauri::WindowUrl::External(final_url.parse().unwrap())
    )
    .title(&app.manifest.name)
    // CRITICAL: Disable IPC to Sandbox the App
    // We obliterate the __TAURI_IPC__ binding to prevent any raw native command execution.
    .initialization_script(\"window.__TAURI_IPC__ = undefined; window.__TAURI__ = undefined; delete window.__TAURI_IPC__; delete window.__TAURI__;\")
    .build()
    .map_err(|e| e.to_string())?;";

    content = content.replace(window_builder_str, "println!(\"Launching: {}\", final_url);");

    // 5. In save_directory_state, state is passed by reference `state: &AppState`, but we can just get it globally.
    content = content.replace("pub fn save_directory_state(state: &AppState) {", "pub fn save_directory_state() {\n    let state = crate::state::APP_STATE.get().unwrap();");
    content = content.replace("save_directory_state(&state)", "save_directory_state()");
    content = content.replace("save_directory_state(state)", "save_directory_state()");

    // 6. llm_offload call in `run_agent_inference` still has extra args
    // It's in `api.rs` lines 997.
    content = content.replace("llm_offload::execute_agent_inference(prompt, model_name, intent_layout)", "llm_offload::execute_agent_inference(prompt, model_name, intent_layout)");

    // Fix other errors:
    // `state` not found in `save_directory_state(&state);` is handled by replacement above.
    
    fs::write("crates/qualia-client-core/src/api.rs", content).unwrap();
}
