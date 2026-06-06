use std::env;

fn main() {
    // Retrieve the target operating system from Cargo's build environment
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS not set");

    println!("cargo:rerun-if-changed=build.rs");

    match target_os.as_str() {
        "android" => {
            // Target: Google Tensor (Edge TPU) & Qualcomm Hexagon NPUs
            // Link the Android Neural Networks API (NNAPI)
            println!("cargo:rustc-link-lib=dylib=neuralnetworks");
            println!("cargo:warning=Qualia-DB Compiling for Android: NNAPI Linked.");
        }
        "macos" | "ios" => {
            // Target: Apple Silicon (M-Series / A-Series)
            // Link Metal for zero-copy UMA compute and Accelerate for the AMX coprocessor
            println!("cargo:rustc-link-lib=framework=Metal");
            println!("cargo:rustc-link-lib=framework=Accelerate");
            println!("cargo:rustc-link-lib=framework=CoreML");
            println!("cargo:rustc-link-lib=framework=Security");
            println!("cargo:warning=Qualia-DB Compiling for Apple Silicon: Metal, Accelerate, CoreML & Security Linked.");
        }
        "windows" => {
            // Target: ARM-based Surface devices, Intel NPUs, AMD Ryzen AI
            // D3D12 is always present on Windows 10+.
            println!("cargo:rustc-link-lib=dylib=d3d12");
            println!("cargo:rustc-link-lib=dylib=dxgi");

            // DirectML 1.15 — shipped in vendor/directml/ (checked into repo).
            // Falls back to DIRECTML_LIB_PATH env var for CI environments that
            // supply their own SDK copy.
            let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
            let vendor   = std::path::PathBuf::from(&manifest)
                .join("..").join("..").join("vendor").join("directml").join("bin").join("x64-win");
            let env_path = std::env::var("DIRECTML_LIB_PATH").ok()
                .map(std::path::PathBuf::from);

            let lib_dir = if vendor.join("DirectML.lib").exists() {
                Some(vendor)
            } else {
                env_path.filter(|p| p.join("DirectML.lib").exists())
            };

            if let Some(dir) = lib_dir {
                println!("cargo:rustc-link-search=native={}", dir.display());
                println!("cargo:rustc-link-lib=dylib=DirectML");
                println!("cargo:rustc-cfg=feature=\"directml\"");
                println!("cargo:warning=Qualia-DB: DirectML 1.15 linked from {}.", dir.display());
            } else {
                println!("cargo:warning=Qualia-DB: vendor/directml not found and DIRECTML_LIB_PATH unset. \
                          GPU inference will fall back to wgpu-only path.");
            }
        }
        "linux" => {
            // Target: Raw Linux Environments / Bare-metal Servers
            // Link Vulkan for massive parallel grid compute shaders
            // println!("cargo:rustc-link-lib=dylib=vulkan");
            println!("cargo:warning=Qualia-DB Compiling for Linux: Vulkan Compute Linking Configured.");
        }
        _ => {
            // Fallback for unsupported OS (Standard CPU Triad only)
            println!("cargo:warning=Qualia-DB: No native NPU/GPU accelerator defined for this OS. Defaulting to CPU Triad.");
        }
    }
}
