use std::fs;

fn main() {
    let mut content = fs::read_to_string("crates/qualia-client-core/src/api.rs").unwrap();

    // 1. Remove tauri import in api.rs
    content = content.replace("use tauri::{State, Window, Manager, SystemTray, SystemTrayMenu, CustomMenuItem, SystemTrayMenuItem, SystemTrayEvent};", "");

    // 2. Remove formatting errors `println!("Opening: ", `
    content = content.replace("println!(\"Opening: \", ", "println!(\"Opening: {}\", ");

    // 3. Remove trailing commas in function params. This happens when the first param was `app:` or `state:` and it was removed, leaving `, app_did: String` or something like that.
    content = content.replace("(, ", "(");
    content = content.replace("(\n    ,", "(");
    
    // There might be some lines that just have `,` from multi-line parameters. Let's do a simple line-by-line check.
    let mut new_content = String::new();
    for line in content.lines() {
        if line.trim() == "," {
            continue; // just drop the lonely comma
        }
        new_content.push_str(line);
        new_content.push('\n');
    }
    content = new_content;

    fs::write("crates/qualia-client-core/src/api.rs", content).unwrap();

    let mut llm_content = fs::read_to_string("crates/qualia-client-core/src/engine/llm_offload.rs").unwrap();
    llm_content = llm_content.replace("use tauri::Manager;", "");
    llm_content = llm_content.replace("use rtrb::RingBuffer;", "");
    fs::write("crates/qualia-client-core/src/engine/llm_offload.rs", llm_content).unwrap();
}
