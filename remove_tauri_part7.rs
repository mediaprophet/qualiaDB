use std::fs;

fn main() {
    let mut content = fs::read_to_string("crates/qualia-client-core/src/api.rs").unwrap();

    // 1. Remove build_tray_menu and handle_tray_event
    let tray_menu_start = content.find("pub fn build_tray_menu()").unwrap_or(0);
    if tray_menu_start > 0 {
        let mut end = tray_menu_start;
        while let Some(idx) = content[end..].find("pub fn ") {
            if idx > 0 {
                // Not the start
                let next_fn = end + idx;
                if content[next_fn..].starts_with("pub fn handle_tray_event") {
                    end = next_fn + 24;
                } else if content[next_fn..].starts_with("pub fn toggle_window") {
                    // Stop here
                    content.replace_range(tray_menu_start..next_fn, "");
                    break;
                } else {
                    end = next_fn + 7;
                }
            } else {
                end += 7;
            }
        }
    }

    // 2. Remove lingering `use tauri...`
    let mut lines: Vec<&str> = content.lines().collect();
    lines.retain(|line| !line.contains("use tauri::"));
    
    // Fix comma parameters `(, app_did: String)`
    let joined = lines.join("\n");
    let joined = joined.replace("(, ", "(");
    let joined = joined.replace("(\n    ,", "(");
    let joined = joined.replace("    ,\n", "");
    
    fs::write("crates/qualia-client-core/src/api.rs", joined).unwrap();

    let mut llm_content = fs::read_to_string("crates/qualia-client-core/src/engine/llm_offload.rs").unwrap();
    let mut llm_lines: Vec<&str> = llm_content.lines().collect();
    llm_lines.retain(|line| !line.contains("use tauri::"));
    llm_lines.retain(|line| !line.contains("use rtrb::"));
    fs::write("crates/qualia-client-core/src/engine/llm_offload.rs", llm_lines.join("\n")).unwrap();
}
