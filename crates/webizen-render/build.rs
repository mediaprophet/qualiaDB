fn main() {
    // Configure platform-specific feature flags
    cfg_aliases::cfg_aliases! {
        wasm: { all(target_arch = "wasm32", target_os = "unknown") },
        native: { not(target_arch = "wasm32") },
        // Use built-in cfg(target_os = "windows") instead of custom alias
        macos: { all(target_os = "macos", not(target_arch = "wasm32")) },
        linux: { all(target_os = "linux", not(target_arch = "wasm32")) },
    }
}
