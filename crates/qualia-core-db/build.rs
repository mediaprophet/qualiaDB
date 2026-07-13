use std::env;
use std::path::{Path, PathBuf};

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS not set");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=QUALIA_BUILD_VERBOSE");
    println!("cargo:rerun-if-env-changed=DIRECTML_LIB_PATH");
    println!("cargo:rerun-if-env-changed=QUALIA_DXC_PATH");

    match target_os.as_str() {
        "android" => {
            println!("cargo:rustc-link-lib=dylib=neuralnetworks");
            build_info("Android: NNAPI linked");
        }
        "macos" | "ios" => {
            println!("cargo:rustc-link-lib=framework=Metal");
            println!("cargo:rustc-link-lib=framework=Accelerate");
            println!("cargo:rustc-link-lib=framework=CoreML");
            println!("cargo:rustc-link-lib=framework=Security");
            build_info("Apple: Metal + Accelerate + CoreML + Security linked");
        }
        "windows" => {
            println!("cargo:rustc-link-lib=dylib=d3d12");
            println!("cargo:rustc-link-lib=dylib=dxgi");

            let manifest = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
            let vendor_dml = PathBuf::from(&manifest)
                .join("..")
                .join("..")
                .join("vendor")
                .join("directml")
                .join("bin")
                .join("x64-win");
            let env_path = env::var("DIRECTML_LIB_PATH").ok().map(PathBuf::from);

            let lib_dir = if vendor_dml.join("DirectML.lib").exists() {
                Some(vendor_dml)
            } else {
                env_path.filter(|p| p.join("DirectML.lib").exists())
            };

            if let Some(dir) = lib_dir {
                println!("cargo:rustc-link-search=native={}", dir.display());
                println!("cargo:rustc-link-lib=dylib=DirectML");
                println!("cargo:rustc-cfg=feature=\"directml\"");
                // Runtime: Windows loads DirectML.dll from the exe/module directory first.
                // Previously only DXC was copied — missing DLL caused load failures or stale
                // system DirectML, and PowerShell treated cargo:warning success spam as errors.
                // Always stage release DLL. Debug-layer DLL only when present (dev machines).
                copy_runtime_dlls(&dir, &["DirectML.dll", "DirectML.Debug.dll"]);
                build_info(&format!("DirectML linked + DLL staged from {}", dir.display()));
            } else {
                // Real problem only — keep as cargo warning.
                println!(
                    "cargo:warning=Qualia-DB: vendor/directml not found and DIRECTML_LIB_PATH unset. \
                     GPU inference will fall back to wgpu-only path."
                );
            }

            // DXC — runtime load by wgpu for WGSL→DXIL on DX12 (FXC cannot compile attention).
            let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
            let dxc_sub = if arch == "aarch64" {
                "arm64-win"
            } else {
                "x64-win"
            };
            let dxc_dir = PathBuf::from(&manifest)
                .join("..")
                .join("..")
                .join("vendor")
                .join("dxc")
                .join("bin")
                .join(dxc_sub);
            let dxc_dll = dxc_dir.join("dxcompiler.dll");
            let dxil_dll = dxc_dir.join("dxil.dll");
            if dxc_dll.exists() && dxil_dll.exists() {
                if let Ok(out_dir) = env::var("OUT_DIR") {
                    if let Some(profile_dir) = Path::new(&out_dir).ancestors().nth(3) {
                        for dst_dir in [profile_dir.to_path_buf(), profile_dir.join("deps")] {
                            let _ = std::fs::create_dir_all(&dst_dir);
                            let _ = std::fs::copy(&dxc_dll, dst_dir.join("dxcompiler.dll"));
                            let _ = std::fs::copy(&dxil_dll, dst_dir.join("dxil.dll"));
                        }
                        println!("cargo:rerun-if-changed={}", dxc_dll.display());
                        println!("cargo:rerun-if-changed={}", dxil_dll.display());
                        build_info(&format!("DXC ({dxc_sub}) staged beside binaries"));
                    }
                }
            } else {
                println!(
                    "cargo:warning=Qualia-DB: vendor/dxc/{dxc_sub} not found — DX12 will fall back to \
                     FXC (which cannot compile the attention shader). Set QUALIA_DXC_PATH or vendor DXC."
                );
            }
        }
        "linux" => {
            if env::var("QUALIA_CUDA").is_ok() {
                println!("cargo:rustc-cfg=feature=\"cuda\"");
                build_info("Linux: QUALIA_CUDA set — cudarc path enabled");
            } else {
                build_info("Linux: Vulkan via wgpu (set QUALIA_CUDA=1 for explicit CUDA)");
            }
        }
        _ => {
            println!(
                "cargo:warning=Qualia-DB: No native NPU/GPU accelerator defined for this OS. Defaulting to CPU Triad."
            );
        }
    }
}

/// Success/info for build — only printed when `QUALIA_BUILD_VERBOSE=1`.
/// Using `cargo:warning=` for success caused PowerShell to treat builds as failed (stderr).
fn build_info(msg: &str) {
    if env::var("QUALIA_BUILD_VERBOSE").ok().as_deref() == Some("1") {
        println!("cargo:warning=Qualia-DB: {msg}");
    }
}

/// Copy vendor DLLs next to `target/<profile>/` and `deps/` so the OS loader finds them.
fn copy_runtime_dlls(src_dir: &Path, names: &[&str]) {
    let Ok(out_dir) = env::var("OUT_DIR") else {
        return;
    };
    let Some(profile_dir) = Path::new(&out_dir).ancestors().nth(3) else {
        return;
    };
    for name in names {
        let src = src_dir.join(name);
        if !src.exists() {
            continue;
        }
        println!("cargo:rerun-if-changed={}", src.display());
        for dst_dir in [profile_dir.to_path_buf(), profile_dir.join("deps")] {
            let _ = std::fs::create_dir_all(&dst_dir);
            if let Err(e) = std::fs::copy(&src, dst_dir.join(name)) {
                println!(
                    "cargo:warning=Qualia-DB: failed to stage {name} → {}: {e}",
                    dst_dir.display()
                );
            }
        }
    }
}
