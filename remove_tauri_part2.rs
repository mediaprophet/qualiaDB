use std::fs;

fn main() {
    let mut content = fs::read_to_string("crates/qualia-client-core/src/api.rs").unwrap();

    // Fix ReadBytesExt
    content = content.replace("use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};", "use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};\nuse byteorder::io::ReadBytesExt as _;");

    // Fix toggle_window
    content = content.replace("toggle_window(app)", "toggle_window()");

    // Fix open crate
    content = content.replace("open::that(", "println!(\"Opening: \", ");

    // Remove get_invoke_handler
    if let Some(pos) = content.find("pub fn get_invoke_handler() -> impl Fn(tauri::Invoke) {") {
        content.truncate(pos);
    }

    fs::write("crates/qualia-client-core/src/api.rs", content).unwrap();

    // Fix state.rs librqbit and KeyVault
    let mut state_content = fs::read_to_string("crates/qualia-client-core/src/state.rs").unwrap();
    state_content = state_content.replace("pub rqbit_session: Arc<tokio::sync::Mutex<Option<std::sync::Arc<librqbit::Session>>>>,", "// pub rqbit_session: Arc<tokio::sync::Mutex<Option<std::sync::Arc<librqbit::Session>>>>,");
    state_content = state_content.replace("rqbit_session: Arc::new(tokio::sync::Mutex::new(None)),", "// rqbit_session: Arc::new(tokio::sync::Mutex::new(None)),");
    state_content = state_content.replace("qualia_core_db::key_vault::KeyVault::new()", "qualia_core_db::key_vault::KeyVault::load_or_generate(\".qualia\").unwrap()");
    fs::write("crates/qualia-client-core/src/state.rs", state_content).unwrap();

    // Fix llm_offload.rs
    let llm_path = "crates/qualia-client-core/src/engine/llm_offload.rs";
    if let Ok(mut llm) = fs::read_to_string(llm_path) {
        llm = llm.replace("app: tauri::AppHandle,", "");
        llm = llm.replace("app.clone(),", "");
        llm = llm.replace("app,", "");
        llm = llm.replace("let _ = app.emit_all(\"inference-token\", &token_payload);", "");
        llm = llm.replace("let _ = app.emit_all(\"inference-complete\", &complete_payload);", "");
        fs::write(llm_path, llm).unwrap();
    }
}
