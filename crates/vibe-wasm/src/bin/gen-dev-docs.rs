//! Dev-docs generator: scans poet-vibe source files, extracts public API
//! items with their doc comments, and emits a JSON file for the docs site.
//!
//! Usage: cargo run -p vibe-wasm --bin gen-dev-docs -- output.json

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(serde::Serialize)]
struct ApiItem {
    name: String,
    kind: String, // "struct", "enum", "fn", "trait", "type", "const", "module"
    doc: String,
    file: String,
    line: usize,
    signature: String,
}

#[derive(serde::Serialize)]
struct ApiModule {
    name: String,
    doc: String,
    items: Vec<ApiItem>,
}

fn extract_doc_comments(lines: &[String], line_idx: usize) -> String {
    // Walk backwards from line_idx to collect consecutive /// comments.
    let mut doc_lines = Vec::new();
    let mut i = line_idx;
    while i > 0 {
        i -= 1;
        let trimmed = lines[i].trim();
        if let Some(rest) = trimmed.strip_prefix("///") {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            doc_lines.insert(0, rest.to_string());
        } else if trimmed.starts_with("//!") {
            // Module doc — stop here
            break;
        } else if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        } else {
            break;
        }
    }
    doc_lines.join("\n")
}

fn extract_module_doc(lines: &[String]) -> String {
    let mut doc_lines = Vec::new();
    for line in lines {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("//!") {
            let rest = rest.strip_prefix(' ').unwrap_or(rest);
            doc_lines.push(rest.to_string());
        } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
            break;
        }
    }
    doc_lines.join("\n")
}

fn process_file(path: &Path, module_name: &str) -> ApiModule {
    let content = fs::read_to_string(path).unwrap_or_default();
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let module_doc = extract_module_doc(&lines);

    let mut items = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // Check for pub items
        if !trimmed.starts_with("pub ") && !trimmed.starts_with("pub(crate)") {
            continue;
        }

        // Determine item kind and extract name
        let (kind, name, signature) = if let Some(rest) = trimmed.strip_prefix("pub ") {
            extract_item(rest, trimmed)
        } else if trimmed.starts_with("pub(crate) ") {
            // Skip crate-private items
            continue;
        } else {
            continue;
        };

        if name.is_empty() {
            continue;
        }

        let doc = extract_doc_comments(&lines, i);

        items.push(ApiItem {
            name,
            kind,
            doc,
            file: path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
            line: i + 1,
            signature,
        });
    }

    ApiModule {
        name: module_name.to_string(),
        doc: module_doc,
        items,
    }
}

fn extract_item(rest: &str, full_line: &str) -> (String, String, String) {
    // Try to match various pub item patterns
    if let Some(s) = rest.strip_prefix("struct ") {
        let name = s
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        return (
            "struct".to_string(),
            name.to_string(),
            full_line.trim_end_matches('{').trim().to_string(),
        );
    }
    if let Some(s) = rest.strip_prefix("enum ") {
        let name = s
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        return (
            "enum".to_string(),
            name.to_string(),
            full_line.trim_end_matches('{').trim().to_string(),
        );
    }
    if let Some(s) = rest.strip_prefix("trait ") {
        let name = s
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        return (
            "trait".to_string(),
            name.to_string(),
            full_line.trim_end_matches('{').trim().to_string(),
        );
    }
    if let Some(s) = rest.strip_prefix("fn ") {
        let name = extract_fn_name(s);
        return ("fn".to_string(), name, full_line.trim().to_string());
    }
    if let Some(s) = rest.strip_prefix("const ") {
        let name = s
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        return (
            "const".to_string(),
            name.to_string(),
            full_line.trim_end_matches(';').trim().to_string(),
        );
    }
    if let Some(s) = rest.strip_prefix("static ") {
        let name = s
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        return (
            "static".to_string(),
            name.to_string(),
            full_line.trim_end_matches(';').trim().to_string(),
        );
    }
    if let Some(s) = rest.strip_prefix("type ") {
        let name = s
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        return (
            "type".to_string(),
            name.to_string(),
            full_line.trim_end_matches(';').trim().to_string(),
        );
    }
    if rest.starts_with("use ") {
        // Re-export — extract the last segment
        let name = rest
            .trim_start_matches("use ")
            .rsplit(|c: char| c == ':' || c == ',')
            .next()
            .unwrap_or("")
            .trim()
            .trim_end_matches(';')
            .to_string();
        return ("use".to_string(), name, full_line.trim().to_string());
    }
    if rest.starts_with("mod ") {
        let name = rest
            .trim_start_matches("mod ")
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .unwrap_or("");
        return (
            "module".to_string(),
            name.to_string(),
            full_line.trim().to_string(),
        );
    }

    (String::new(), String::new(), String::new())
}

fn extract_fn_name(s: &str) -> String {
    // Find the function name after "fn "
    let after_fn = s.strip_prefix("fn ").unwrap_or(s);
    // Take chars until we hit ( or < or whitespace
    let mut name = String::new();
    for c in after_fn.chars() {
        if c == '(' || c == '<' || c == ' ' {
            break;
        }
        name.push(c);
    }
    name
}

fn scan_directory(dir: &Path, prefix: &str) -> Vec<ApiModule> {
    let mut modules = Vec::new();

    // Process mod.rs or lib.rs first
    let mod_rs = dir.join("mod.rs");
    let lib_rs = dir.join("lib.rs");
    let mod_file: Option<PathBuf> = if mod_rs.exists() {
        Some(mod_rs)
    } else if lib_rs.exists() {
        Some(lib_rs)
    } else {
        None
    };
    if let Some(ref mod_path) = mod_file {
        if mod_path.exists() {
            let module_name = if prefix.is_empty() {
                "root".to_string()
            } else {
                prefix.to_string()
            };
            let module = process_file(mod_path, &module_name);

            // Find submodules
            let content = fs::read_to_string(mod_path).unwrap_or_default();
            for line in content.lines() {
                let trimmed = line.trim();
                if let Some(rest) = trimmed.strip_prefix("pub mod ") {
                    let submod = rest
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .next()
                        .unwrap_or("");
                    if !submod.is_empty() {
                        let submod_path = dir.join(format!("{}.rs", submod));
                        let submod_dir = dir.join(submod);
                        let full_name = if prefix.is_empty() {
                            submod.to_string()
                        } else {
                            format!("{}::{}", prefix, submod)
                        };
                        if submod_path.exists() {
                            modules.push(process_file(&submod_path, &full_name));
                        } else if submod_dir.exists() {
                            let mut sub = scan_directory(&submod_dir, &full_name);
                            modules.append(&mut sub);
                        }
                    }
                } else if let Some(rest) = trimmed.strip_prefix("mod ") {
                    let submod = rest
                        .split(|c: char| !c.is_alphanumeric() && c != '_')
                        .next()
                        .unwrap_or("");
                    if !submod.is_empty() {
                        let submod_path = dir.join(format!("{}.rs", submod));
                        let submod_dir = dir.join(submod);
                        let full_name = if prefix.is_empty() {
                            submod.to_string()
                        } else {
                            format!("{}::{}", prefix, submod)
                        };
                        if submod_path.exists() {
                            modules.push(process_file(&submod_path, &full_name));
                        } else if submod_dir.exists() {
                            let mut sub = scan_directory(&submod_dir, &full_name);
                            modules.append(&mut sub);
                        }
                    }
                }
            }

            modules.push(module);
        }
    }

    modules
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let output_path = if args.len() >= 2 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("docs/vibe/dev-docs.json")
    };

    // Find poet-vibe source directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
    let poet_vibe_src = PathBuf::from(&manifest_dir)
        .join("..")
        .join("poet-vibe")
        .join("src");

    if !poet_vibe_src.exists() {
        eprintln!("poet-vibe source not found at {:?}", poet_vibe_src);
        std::process::exit(1);
    }

    eprintln!("Scanning {:?} ...", poet_vibe_src);
    let modules = scan_directory(&poet_vibe_src, "");

    // Sort modules by name
    let mut module_map: BTreeMap<String, ApiModule> = BTreeMap::new();
    for m in modules {
        module_map.entry(m.name.clone()).or_insert(m);
    }

    let json = serde_json::to_string_pretty(&module_map.values().collect::<Vec<_>>()).unwrap();

    // Ensure parent directory exists
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).ok();
    }

    fs::write(&output_path, &json).expect("write output");
    eprintln!("Wrote {} modules to {:?}", module_map.len(), output_path);

    // Also print a summary
    let total_items: usize = module_map.values().map(|m| m.items.len()).sum();
    eprintln!(
        "Total: {} modules, {} public items",
        module_map.len(),
        total_items
    );
}
