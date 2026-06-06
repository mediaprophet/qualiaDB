import os

main_rs_path = r"crates\qualia-desktop\src\main.rs"
with open(main_rs_path, "r", encoding="utf-8") as f:
    lines = f.readlines()

commands_start_idx = -1
for i, line in enumerate(lines):
    if "── Tauri commands" in line:
        commands_start_idx = i
        break

main_fn_idx = -1
for i in range(commands_start_idx, len(lines)):
    if line.startswith("fn main() {") or "fn main() {" in lines[i]:
        main_fn_idx = i
        break

if commands_start_idx == -1 or main_fn_idx == -1:
    print("Could not find delimiters")
    exit(1)

commands_lines = lines[commands_start_idx:main_fn_idx]
main_rs_top = lines[:commands_start_idx]
main_rs_bottom = lines[main_fn_idx:]

os.makedirs(r"crates\qualia-desktop\src\commands", exist_ok=True)
with open(r"crates\qualia-desktop\src\commands\mod.rs", "w", encoding="utf-8") as f:
    f.write("use crate::state::*;\n")
    f.write("use tauri::{State, Window, Manager};\n")
    f.write("use serde::{Deserialize, Serialize};\n")
    f.write("use std::path::PathBuf;\n")
    f.write("use std::collections::HashMap;\n")
    f.write("use std::sync::{Arc, Mutex};\n")
    f.write("use std::sync::atomic::{AtomicBool, Ordering};\n")
    f.write("use qualia_core_db::rpc::TaxRecipientSuite;\n")
    f.write("use sysinfo::{System, Disks};\n")
    f.write("use crate::ingestion;\n")
    f.write("use crate::llm_offload;\n")
    f.write("use crate::app_registry;\n")
    f.write("\n")
    f.writelines(commands_lines)

# Write back to main.rs
with open(main_rs_path, "w", encoding="utf-8") as f:
    f.writelines(main_rs_top)
    f.write("pub mod commands;\n")
    f.write("pub use commands::*;\n\n")
    f.writelines(main_rs_bottom)

print("Split complete!")
