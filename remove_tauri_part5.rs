use std::fs;

fn main() {
    let mut content = fs::read_to_string("crates/qualia-client-core/src/api.rs").unwrap();

    // 1. Completely remove all #[tauri::command] lines regardless of indentation
    let mut new_content = String::new();
    for line in content.lines() {
        if line.trim() == "#[tauri::command]" {
            continue;
        }
        new_content.push_str(line);
        new_content.push('\n');
    }
    content = new_content;

    // 2. Fix toggle_window logic
    // The previous replace removed `window: Window` but left `match window.is_visible() {`
    // We will just replace the body of `toggle_window()` entirely.
    let old_toggle_window = "pub fn toggle_window() {
    if false {
        match window.is_visible() {
            Ok(true)  => { let _ = window.hide(); }
            Ok(false) => { let _ = window.show(); let _ = window.set_focus(); }
            Err(_)    => {}
        }
    }
}";
    let new_toggle_window = "pub fn toggle_window() {\n    // No-op without Tauri\n}";
    content = content.replace(old_toggle_window, new_toggle_window);

    // If it was already rewritten and has some other shape, let's just do an aggressive replace:
    content = content.replace("match window.is_visible() {", "match Ok::<bool, ()>(false) {");
    content = content.replace("Ok(true)  => { let _ = window.hide(); }", "Ok(true) => {}");
    content = content.replace("Ok(false) => { let _ = window.show(); let _ = window.set_focus(); }", "Ok(false) => {}");

    fs::write("crates/qualia-client-core/src/api.rs", content).unwrap();
}
