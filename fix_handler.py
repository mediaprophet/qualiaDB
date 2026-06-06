import re
import os

main_rs_path = r"crates\qualia-desktop\src\main.rs"
commands_mod_path = r"crates\qualia-desktop\src\commands\mod.rs"

with open(main_rs_path, "r", encoding="utf-8") as f:
    main_content = f.read()

# Extract tauri::generate_handler block from main.rs
match = re.search(r'(tauri::generate_handler!\[(.*?)\])', main_content, re.DOTALL)
if match:
    handler_block = match.group(1)
    # Replace in main.rs
    main_content = main_content.replace(handler_block, "commands::get_invoke_handler()")
    with open(main_rs_path, "w", encoding="utf-8") as f:
        f.write(main_content)
    
    # Append to commands/mod.rs
    with open(commands_mod_path, "a", encoding="utf-8") as f:
        f.write("\n\npub fn get_invoke_handler() -> impl Fn(tauri::Invoke) {\n")
        f.write("    " + handler_block + "\n")
        f.write("}\n")

# Now ensure all functions in commands/mod.rs are pub
with open(commands_mod_path, "r", encoding="utf-8") as f:
    cmd_lines = f.readlines()

for i, line in enumerate(cmd_lines):
    if line.startswith("fn ") or line.startswith("async fn "):
        cmd_lines[i] = "pub " + line

with open(commands_mod_path, "w", encoding="utf-8") as f:
    f.writelines(cmd_lines)

print("Handler moved and functions made pub!")
