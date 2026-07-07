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
            let vendor = std::path::PathBuf::from(&manifest)
                .join("..")
                .join("..")
                .join("vendor")
                .join("directml")
                .join("bin")
                .join("x64-win");
            let env_path = std::env::var("DIRECTML_LIB_PATH")
                .ok()
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
                println!(
                    "cargo:warning=Qualia-DB: DirectML 1.15 linked from {}.",
                    dir.display()
                );
            } else {
                println!("cargo:warning=Qualia-DB: vendor/directml not found and DIRECTML_LIB_PATH unset. \
                          GPU inference will fall back to wgpu-only path.");
            }

            // DXC (DirectX Shader Compiler) — shipped in vendor/dxc/ (checked into repo). Unlike
            // DirectML this is NOT link-time: `dxcompiler.dll` is loaded at RUNTIME by wgpu to compile
            // WGSL→DXIL for the DX12 backend (DX12's legacy FXC cannot compile `fused_attention.wgsl`).
            // We copy the two DLLs next to the built binaries so the OS loader finds them from the
            // exe's own directory → wgpu's default `Auto` compiler uses DXC with no env var (turnkey
            // DX12). `QUALIA_DXC_PATH` still overrides for a bespoke DXC location. `dxil.dll` must sit
            // beside `dxcompiler.dll` (DXIL signing).
            let arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
            let dxc_sub = if arch == "aarch64" {
                "arm64-win"
            } else {
                "x64-win"
            };
            let dxc_dir = std::path::PathBuf::from(&manifest)
                .join("..")
                .join("..")
                .join("vendor")
                .join("dxc")
                .join("bin")
                .join(dxc_sub);
            let dxc_dll = dxc_dir.join("dxcompiler.dll");
            let dxil_dll = dxc_dir.join("dxil.dll");
            if dxc_dll.exists() && dxil_dll.exists() {
                // OUT_DIR = target/<profile>/build/<crate>-<hash>/out → the profile dir is 3 ancestors up.
                if let Ok(out_dir) = std::env::var("OUT_DIR") {
                    if let Some(profile_dir) = std::path::Path::new(&out_dir).ancestors().nth(3) {
                        // binaries live in <profile>/ (qualia-cli) and <profile>/deps/ (test exes);
                        // Windows loads a DLL from the loading module's own directory first.
                        for dst_dir in [profile_dir.to_path_buf(), profile_dir.join("deps")] {
                            let _ = std::fs::create_dir_all(&dst_dir);
                            let _ = std::fs::copy(&dxc_dll, dst_dir.join("dxcompiler.dll"));
                            let _ = std::fs::copy(&dxil_dll, dst_dir.join("dxil.dll"));
                        }
                        println!("cargo:rerun-if-changed={}", dxc_dll.display());
                        println!("cargo:rerun-if-changed={}", dxil_dll.display());
                        println!(
                            "cargo:warning=Qualia-DB: DXC ({dxc_sub}) copied beside binaries — DX12 uses DXC (turnkey)."
                        );
                    }
                }
            } else {
                println!("cargo:warning=Qualia-DB: vendor/dxc/{dxc_sub} not found — DX12 will fall back to \
                          FXC (which cannot compile the attention shader). Set QUALIA_DXC_PATH or vendor DXC.");
            }
        }
        "linux" => {
            // Target: Raw Linux Environments / Bare-metal Servers
            //
            // wgpu selects Vulkan automatically on Linux — it picks up the
            // system Vulkan ICD (NVIDIA, AMD RADV, Intel ANV) without any
            // explicit link directive here.  All WGSL shaders in
            // `src/shaders/` execute via Vulkan on Linux without changes.
            //
            // NVIDIA CUDA (cuBLAS) path — optional, ~10 % faster than Vulkan
            // for Q4_K GEMM on large tensors.  Enable by building with:
            //   QUALIA_CUDA=1 cargo build --release
            // and add `cudarc = "0.11"` to Cargo.toml.
            if std::env::var("QUALIA_CUDA").is_ok() {
                println!("cargo:rustc-cfg=feature=\"cuda\"");
                println!(
                    "cargo:warning=Qualia-DB Linux: QUALIA_CUDA set — stub ready for cudarc GEMM."
                );
            } else {
                println!(
                    "cargo:warning=Qualia-DB Linux: Vulkan via wgpu (covers NVIDIA/AMD/Intel). \
                          Set QUALIA_CUDA=1 for explicit cuBLAS path."
                );
            }
        }
        _ => {
            // Fallback for unsupported OS (Standard CPU Triad only)
            println!("cargo:warning=Qualia-DB: No native NPU/GPU accelerator defined for this OS. Defaulting to CPU Triad.");
        }
    }
}
