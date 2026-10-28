use std::env;
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS not set");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH not set");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=QUALIA_BUILD_VERBOSE");
    println!("cargo:rerun-if-env-changed=DIRECTML_LIB_PATH");
    println!("cargo:rerun-if-env-changed=QUALIA_DXC_PATH");
    println!("cargo:rerun-if-env-changed=QUALIA_DXC_CLI_PATH");

    // wasm32 uses browser WebGPU / CPU fallbacks and must never be diagnosed as
    // a host OS with a missing native accelerator.
    if target_arch == "wasm32" {
        build_info("WASM: browser WebGPU + deterministic CPU fallback");
        return;
    }

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
            // The portable native renderer profile intentionally omits DX12 and
            // DirectML. Do not force Windows SDK import libraries into its link
            // step: MinGW environments can still build/run the GLES renderer.
            // The default profile enables gpu-native-dx12 and keeps the existing
            // inference/DirectML link behavior.
            let dx12_enabled = env::var_os("CARGO_FEATURE_GPU_NATIVE_DX12").is_some();
            if dx12_enabled {
                println!("cargo:rustc-link-lib=dylib=d3d12");
                println!("cargo:rustc-link-lib=dylib=dxgi");
            }

            let manifest = env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
            let vendor_dml = PathBuf::from(&manifest)
                .join("..")
                .join("..")
                .join("vendor")
                .join("directml")
                .join("bin")
                .join("x64-win");
            let env_path = if dx12_enabled {
                env::var("DIRECTML_LIB_PATH").ok().map(PathBuf::from)
            } else {
                None
            };

            let lib_dir = if dx12_enabled && vendor_dml.join("DirectML.lib").exists() {
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
                build_info(&format!(
                    "DirectML linked + DLL staged from {}",
                    dir.display()
                ));
            } else if dx12_enabled {
                // Real problem only — keep as cargo warning.
                println!(
                    "cargo:warning=Qualia-DB: vendor/directml not found and DIRECTML_LIB_PATH unset. \
                     GPU inference will fall back to wgpu-only path."
                );
            } else {
                build_info("Windows portable native profile: DX12 and DirectML links disabled");
            }

            // DXC — runtime load by wgpu for WGSL→DXIL on DX12 (FXC cannot compile attention).
            let dxc_sub = if target_arch == "aarch64" {
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
            let dxc_exe = dxc_dir.join("dxc.exe");
            if dxc_dll.exists() && dxil_dll.exists() {
                if let Ok(out_dir) = env::var("OUT_DIR") {
                    if let Some(profile_dir) = Path::new(&out_dir).ancestors().nth(3) {
                        for dst_dir in [profile_dir.to_path_buf(), profile_dir.join("deps")] {
                            if let Err(e) = std::fs::create_dir_all(&dst_dir) {
                                println!(
                                    "cargo:warning=Qualia-DB: failed to create runtime staging directory {}: {e}",
                                    dst_dir.display()
                                );
                                continue;
                            }
                            stage_runtime_file(&dxc_dll, &dst_dir.join("dxcompiler.dll"));
                            stage_runtime_file(&dxil_dll, &dst_dir.join("dxil.dll"));
                            // Stage dxc.exe for HLSL→SPIR-V CLI compilation (forge HLSL backend).
                            if dxc_exe.exists() {
                                stage_runtime_file(&dxc_exe, &dst_dir.join("dxc.exe"));
                            }
                        }
                        println!("cargo:rerun-if-changed={}", dxc_dll.display());
                        println!("cargo:rerun-if-changed={}", dxil_dll.display());
                        if dxc_exe.exists() {
                            println!("cargo:rerun-if-changed={}", dxc_exe.display());
                        }
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
            if let Err(e) = std::fs::create_dir_all(&dst_dir) {
                println!(
                    "cargo:warning=Qualia-DB: failed to create runtime staging directory {} for {name}: {e}",
                    dst_dir.display()
                );
                continue;
            }
            stage_runtime_file(&src, &dst_dir.join(name));
        }
    }
}

/// Avoid replacing locked build outputs when the staged runtime file is already
/// byte-identical. Changed-file copy failures remain visible as Cargo warnings.
fn stage_runtime_file(src: &Path, dst: &Path) {
    if let Err(e) = copy_runtime_file_if_changed(src, dst) {
        println!(
            "cargo:warning=Qualia-DB: failed to compare or stage runtime file {} → {}: {e}",
            src.display(),
            dst.display()
        );
    }
}

/// Returns `false` when already identical and `true` after a staged copy.
fn copy_runtime_file_if_changed(src: &Path, dst: &Path) -> std::io::Result<bool> {
    match files_equal(src, dst) {
        Ok(true) => return Ok(false),
        Ok(false) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e),
    }
    std::fs::copy(src, dst)?;
    Ok(true)
}

fn files_equal(left: &Path, right: &Path) -> std::io::Result<bool> {
    let left_meta = std::fs::metadata(left)?;
    let right_meta = match std::fs::metadata(right) {
        Ok(meta) => meta,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(e),
    };
    if left_meta.len() != right_meta.len() {
        return Ok(false);
    }

    let mut left = File::open(left)?;
    let mut right = File::open(right)?;
    let mut left_buf = [0u8; 16 * 1024];
    let mut right_buf = [0u8; 16 * 1024];
    loop {
        let left_read = left.read(&mut left_buf)?;
        let right_read = right.read(&mut right_buf)?;
        if left_read != right_read || left_buf[..left_read] != right_buf[..right_read] {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
    }
}

#[cfg(test)]
mod runtime_staging_tests {
    use super::copy_runtime_file_if_changed;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct Scratch(PathBuf);

    impl Scratch {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock before epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "qualia-runtime-staging-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("create isolated test directory");
            Self(path)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            if let Ok(entries) = fs::read_dir(&self.0) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        let mut permissions = metadata.permissions();
                        permissions.set_readonly(false);
                        let _ = fs::set_permissions(entry.path(), permissions);
                    }
                }
            }
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn identical_read_only_destination_is_not_rewritten() {
        let scratch = Scratch::new();
        let src = scratch.0.join("source.dll");
        let dst = scratch.0.join("destination.dll");
        fs::write(&src, b"same runtime bytes").unwrap();
        fs::write(&dst, b"same runtime bytes").unwrap();
        let mut permissions = fs::metadata(&dst).unwrap().permissions();
        permissions.set_readonly(true);
        fs::set_permissions(&dst, permissions).unwrap();

        assert!(!copy_runtime_file_if_changed(&src, &dst).unwrap());
        assert_eq!(fs::read(&dst).unwrap(), b"same runtime bytes");
    }

    #[test]
    fn same_length_difference_after_first_chunk_is_detected_and_copied() {
        let scratch = Scratch::new();
        let src = scratch.0.join("source.dll");
        let dst = scratch.0.join("destination.dll");
        let source_bytes = vec![0x5a; 48 * 1024];
        let mut destination_bytes = source_bytes.clone();
        destination_bytes[40 * 1024] = 0xa5;
        fs::write(&src, &source_bytes).unwrap();
        fs::write(&dst, &destination_bytes).unwrap();

        assert!(copy_runtime_file_if_changed(&src, &dst).unwrap());
        assert_eq!(fs::read(&dst).unwrap(), source_bytes);
    }

    #[test]
    fn missing_destination_is_staged() {
        let scratch = Scratch::new();
        let src = scratch.0.join("source.dll");
        let dst = scratch.0.join("destination.dll");
        fs::write(&src, b"runtime bytes").unwrap();

        assert!(copy_runtime_file_if_changed(&src, &dst).unwrap());
        assert_eq!(fs::read(&dst).unwrap(), b"runtime bytes");
    }
}
