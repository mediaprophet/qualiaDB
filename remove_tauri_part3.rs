use std::fs;

fn main() {
    let mut content = fs::read_to_string("crates/qualia-client-core/src/api.rs").unwrap();

    // Fix execute_agent_inference call
    content = content.replace("llm_offload::execute_agent_inference(app_handle, prompt, model_name, intent_layout)", "llm_offload::execute_agent_inference(prompt, model_name, intent_layout)");

    // Fix daemon_status call
    content = content.replace("daemon_status(state)", "daemon_status()");

    // Fix trait imports in api.rs
    content = content.replace("use byteorder::io::ReadBytesExt as _;\n", "");
    content = content.replace("use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};", "use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};");

    // Fix tauri import in api.rs which wasn't fully removed
    // We already removed #[tauri::command]. Are there any tauri paths?
    content = content.replace("tauri::AppHandle", "String /* Dummy */");
    content = content.replace("app: String /* Dummy */", "");
    content = content.replace("app_handle: String /* Dummy */", "");

    fs::write("crates/qualia-client-core/src/api.rs", content).unwrap();
}
