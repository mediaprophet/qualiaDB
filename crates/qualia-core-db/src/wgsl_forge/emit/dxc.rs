#[cfg(feature = "dxc")]
use std::process::Command;
#[cfg(feature = "dxc")]
use std::io::Write;
#[cfg(feature = "dxc")]
use tempfile::NamedTempFile;
use crate::wgsl_forge::ForgeError;

/// Compiles HLSL source to SPIR-V bytecodes using the DirectXShaderCompiler (DXC).
#[cfg(feature = "dxc")]
pub fn compile_hlsl_to_spirv(hlsl_source: &str, entry_point: &str) -> Result<Vec<u8>, ForgeError> {
    let mut temp_in = NamedTempFile::new().map_err(|e| ForgeError::Emission(format!("Failed to create temp input file: {:?}", e)))?;
    temp_in.write_all(hlsl_source.as_bytes()).map_err(|e| ForgeError::Emission(format!("Failed to write HLSL: {:?}", e)))?;
    
    let temp_out = NamedTempFile::new().map_err(|e| ForgeError::Emission(format!("Failed to create temp output file: {:?}", e)))?;
    let out_path = temp_out.path().to_str().unwrap();

    // Resolve DXC from `QUALIA_DXC_PATH` if set, otherwise rely on `dxc` being
    // on PATH. Avoids baking a machine-specific absolute path into the binary.
    let dxc_path = std::env::var("QUALIA_DXC_PATH").unwrap_or_else(|_| "dxc".to_string());

    let output = Command::new(&dxc_path)
        .arg("-spirv")
        .arg("-fspv-target-env=vulkan1.1")
        .arg("-fvk-use-dx-layout")
        .arg("-T").arg("cs_6_0")
        .arg("-E").arg(entry_point)
        .arg(temp_in.path())
        .arg("-Fo").arg(out_path)
        .output()
        .map_err(|e| ForgeError::Emission(format!("Failed to execute DXC CLI at {}: {:?}", dxc_path, e)))?;

    if !output.status.success() {
        let err_str = String::from_utf8_lossy(&output.stderr);
        let out_str = String::from_utf8_lossy(&output.stdout);
        return Err(ForgeError::Emission(format!("DXC Compilation Failed:\nStdout: {}\nStderr: {}", out_str, err_str)));
    }

    let spirv_blob = std::fs::read(out_path).map_err(|e| ForgeError::Emission(format!("Failed to read SPIRV output: {:?}", e)))?;
    Ok(spirv_blob)
}

/// Fallback when DXC feature is disabled.
#[cfg(not(feature = "dxc"))]
pub fn compile_hlsl_to_spirv(_hlsl_source: &str, _entry_point: &str) -> Result<Vec<u8>, ForgeError> {
    Err(ForgeError::Emission("HLSL compilation requires the 'dxc' feature".to_string()))
}
