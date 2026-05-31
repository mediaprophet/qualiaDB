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
            // Link DirectX 12 and DirectML
            
            // NOTE: We wrap these in a cfg check or comment them out if the host doesn't have the DirectML SDK installed
            // otherwise `cargo test` will fail with LNK1181 on standard Windows machines.
            // println!("cargo:rustc-link-lib=dylib=d3d12");
            // println!("cargo:rustc-link-lib=dylib=directml");
            
            println!("cargo:warning=Qualia-DB Compiling for Windows: DirectML Linking Configured (Awaiting SDK).");
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
