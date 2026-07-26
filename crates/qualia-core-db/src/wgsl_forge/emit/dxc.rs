use crate::wgsl_forge::ForgeError;
#[cfg(feature = "dxc")]
use std::process::Command;

/// Compiles HLSL source to SPIR-V bytecodes using the DirectXShaderCompiler (DXC).
#[cfg(feature = "dxc")]
pub fn compile_hlsl_to_spirv(hlsl_source: &str, entry_point: &str) -> Result<Vec<u8>, ForgeError> {
    // Use temp path strings (not NamedTempFile) for both input and output —
    // NamedTempFile holds exclusive locks on Windows that prevent DXC from
    // accessing the files.
    let temp_dir = std::env::temp_dir();
    let in_path = temp_dir.join(format!(
        "qualia_dxc_in_{}_{}.hlsl",
        std::process::id(),
        entry_point
    ));
    let out_path = temp_dir.join(format!(
        "qualia_dxc_out_{}_{}.spv",
        std::process::id(),
        entry_point
    ));

    std::fs::write(&in_path, hlsl_source)
        .map_err(|e| ForgeError::Emission(format!("Failed to write HLSL temp file: {:?}", e)))?;

    let in_path_str = in_path.to_str().unwrap();
    let out_path_str = out_path.to_str().unwrap();

    // Resolve DXC CLI. Priority:
    //   1. QUALIA_DXC_CLI_PATH — explicit path to dxc.exe (CLI tool, not the DLL)
    //   2. Vendored dxc.exe beside the current executable (build.rs stages dxcompiler.dll
    //      there; dxc.exe lives in the same vendor/dxc/bin/<arch>/ directory)
    //   3. "dxc" on PATH
    //
    // NOTE: QUALIA_DXC_PATH is intentionally NOT used here — that env var points
    // to dxcompiler.dll (the dynamic library wgpu loads for DX12 compilation),
    // not the dxc.exe CLI. Using it would try to execute a DLL as a program.
    let dxc_path = resolve_dxc_cli_path();

    // Note: -fspv-flatten-composite-loads is omitted because the vendored DXC
    // build doesn't support it. It's an optional optimization.
    let output = Command::new(&dxc_path)
        .arg("-spirv")
        .arg("-fspv-target-env=vulkan1.1")
        .arg("-fvk-use-dx-layout")
        .arg("-O3")
        .arg("-T")
        .arg("cs_6_0")
        .arg("-E")
        .arg(entry_point)
        .arg(in_path_str)
        .arg("-Fo")
        .arg(out_path_str)
        .output()
        .map_err(|e| {
            ForgeError::Emission(format!(
                "Failed to execute DXC CLI at {}: {:?}",
                dxc_path, e
            ))
        })?;

    if !output.status.success() {
        let _ = std::fs::remove_file(&in_path);
        let _ = std::fs::remove_file(&out_path);
        let err_str = String::from_utf8_lossy(&output.stderr);
        let out_str = String::from_utf8_lossy(&output.stdout);
        return Err(ForgeError::Emission(format!(
            "DXC Compilation Failed:\nStdout: {}\nStderr: {}",
            out_str, err_str
        )));
    }

    let spirv_blob = std::fs::read(&out_path)
        .map_err(|e| ForgeError::Emission(format!("Failed to read SPIRV output: {:?}", e)))?;
    // Clean up temp files.
    let _ = std::fs::remove_file(&in_path);
    let _ = std::fs::remove_file(&out_path);
    Ok(spirv_blob)
}

/// Resolve the DXC CLI executable path.
///
/// Uses `QUALIA_DXC_CLI_PATH` if set, otherwise tries to find `dxc.exe` beside
/// the current executable (where `build.rs` stages vendored DXC DLLs), falling
/// back to `dxc` on PATH. Does NOT use `QUALIA_DXC_PATH` (that points to
/// `dxcompiler.dll` for wgpu's DX12 backend — a different file).
#[cfg(feature = "dxc")]
fn resolve_dxc_cli_path() -> String {
    if let Ok(p) = std::env::var("QUALIA_DXC_CLI_PATH") {
        if !p.trim().is_empty() {
            return p;
        }
    }
    // Try vendored dxc.exe beside the executable.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let dxc_exe = dir.join("dxc.exe");
            if dxc_exe.exists() {
                return dxc_exe.to_string_lossy().into_owned();
            }
        }
    }
    "dxc".to_string()
}

/// Fallback when DXC feature is disabled.
#[cfg(not(feature = "dxc"))]
pub fn compile_hlsl_to_spirv(
    _hlsl_source: &str,
    _entry_point: &str,
) -> Result<Vec<u8>, ForgeError> {
    Err(ForgeError::Emission(
        "HLSL compilation requires the 'dxc' feature".to_string(),
    ))
}
