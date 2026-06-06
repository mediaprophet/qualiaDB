use std::fs;

fn main() {
    let mut content = fs::read_to_string("crates/qualia-client-core/src/api.rs").unwrap();

    // Very simple replacements based on string operations
    content = content.replace("use tauri::{State, Window, Manager, SystemTray, SystemTrayMenu, CustomMenuItem, SystemTrayMenuItem, SystemTrayEvent};\n", "");
    content = content.replace("#[tauri::command]\n", "");
    content = content.replace("state: State<AppState>", "");
    content = content.replace("state: tauri::State<'_, AppState>", "");
    content = content.replace("state: tauri::State<AppState>", "");
    content = content.replace("app: tauri::AppHandle", "");
    content = content.replace("app: &tauri::AppHandle", "");
    content = content.replace("window: Window", "");
    content = content.replace("app_handle: tauri::AppHandle", "");
    content = content.replace("tauri::async_runtime::spawn", "tokio::spawn");

    // Fix commas and empty args
    content = content.replace("(, ", "(");
    content = content.replace(", )", ")");
    content = content.replace("(,)", "()");
    content = content.replace("( )", "()");
    content = content.replace(", ,", ",");
    content = content.replace(",  ,", ",");

    // Insert let state = APP_STATE.get().unwrap();
    let funcs_using_state = vec![
        "list_installed_apps", "verify_and_install_app", "get_config", "save_config", 
        "download_and_vectorize", "download_model", "cancel_download", "daemon_status", 
        "get_tax_suite", "save_tax_suite", "dispatch_tax_payment", "ingest_literature", 
        "ingest_ontology", "get_tokens", "add_token", "remove_token", "get_directory_actors", 
        "add_directory_actor", "get_front_doors", "add_front_door", "get_delegation_rules", 
        "add_delegation_rule", "get_engine_telemetry", "save_directory_state"
    ];

    for fn_name in funcs_using_state {
        let pattern1 = format!("pub fn {}(", fn_name);
        let pattern2 = format!("pub async fn {}(", fn_name);
        
        let mut new_content = String::new();
        let mut lines = content.lines().peekable();
        while let Some(line) = lines.next() {
            new_content.push_str(line);
            new_content.push('\n');
            if line.contains(&pattern1) || line.contains(&pattern2) {
                // If it doesn't end with {, keep reading until {
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

    content = content.replace("let _ = app.emit_all(\"download-progress\", &payload);", "let _ = state.download_events.send(payload.clone());");
    content = content.replace("let _ = app.emit_all(\"download-progress\", &processing_payload);", "let _ = state.download_events.send(processing_payload.clone());");
    content = content.replace("let _ = app.emit_all(\"download-progress\", &done_payload);", "let _ = state.download_events.send(done_payload.clone());");
    content = content.replace("let _ = app.emit_all(\"ingestion-complete\", payload);", "/* TODO: remove ingestion-complete */");
    content = content.replace("let _ = app.emit_all(\"active-model-changed\", &model_name);", "/* TODO: remove active-model-changed */");

    fs::write("crates/qualia-client-core/src/api.rs", content).unwrap();
}
