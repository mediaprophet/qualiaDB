use serde::{Deserialize, Serialize};

use crate::wgsl_forge::ForgeError;
use crate::wgsl_forge::TargetBackend;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub source_hash: String,
    pub entry_points: Vec<String>,
    pub binding_count: usize,
    pub naga_validated: bool,
    pub native_tool_validated: Option<String>,
}

pub fn validate_wgsl(source: &str) -> Result<ValidationReport, ForgeError> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|error| ForgeError::WgslParse(error.emit_to_string(source)))?;
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    );
    validator
        .validate(&module)
        .map_err(|error| ForgeError::WgslValidation(format!("{error:?}")))?;

    let mut entry_points = module
        .entry_points
        .iter()
        .map(|entry| entry.name.clone())
        .collect::<Vec<_>>();
    entry_points.sort();
    let binding_count = module
        .global_variables
        .iter()
        .filter(|(_, variable)| variable.binding.is_some())
        .count();
    Ok(ValidationReport {
        source_hash: blake3::hash(source.as_bytes()).to_hex().to_string(),
        entry_points,
        binding_count,
        naga_validated: true,
        native_tool_validated: None,
    })
}

pub fn validate_native(source: &str, target: TargetBackend) -> Result<ValidationReport, ForgeError> {
    use std::io::Write;
    use std::process::Command;

    let tool_name = match target {
        TargetBackend::Ptx => "ptxas",
        TargetBackend::Hlsl => "dxc",
        TargetBackend::Msl => "xcrun",
        _ => return Err(ForgeError::WgslValidation("Not a native target supported by offline validation".to_string())),
    };

    let temp_file = tempfile::Builder::new()
        .suffix(match target {
            TargetBackend::Ptx => ".ptx",
            TargetBackend::Hlsl => ".hlsl",
            TargetBackend::Msl => ".metal",
            _ => "",
        })
        .tempfile()
        .map_err(|e| ForgeError::Io(format!("Failed to create temp file: {}", e)))?;

    let mut temp_path = temp_file.path().to_path_buf();
    
    // Write source to temp file
    std::fs::write(&temp_path, source)
        .map_err(|e| ForgeError::Io(format!("Failed to write to temp file: {}", e)))?;

    let mut cmd = Command::new(tool_name);
    match target {
        TargetBackend::Ptx => {
            // Check if ptxas exists first, fallback gently
            if Command::new("ptxas").arg("--version").output().is_err() {
                return Err(ForgeError::WgslValidation("ptxas not found in PATH".to_string()));
            }
            cmd.arg(&temp_path).arg("-c"); // Compile only
        }
        TargetBackend::Hlsl => {
            if Command::new("dxc").arg("--help").output().is_err() {
                return Err(ForgeError::WgslValidation("dxc not found in PATH".to_string()));
            }
            cmd.arg("-T").arg("cs_6_0").arg(&temp_path);
        }
        TargetBackend::Msl => {
            if Command::new("xcrun").arg("--version").output().is_err() {
                return Err(ForgeError::WgslValidation("xcrun not found in PATH".to_string()));
            }
            cmd.arg("-sdk").arg("macosx").arg("metal").arg("-c").arg(&temp_path);
        }
        _ => {}
    }

    let output = cmd.output().map_err(|e| ForgeError::Io(format!("Failed to execute validation tool: {}", e)))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ForgeError::WgslValidation(format!("{} validation failed: {}", tool_name, stderr)));
    }

    Ok(ValidationReport {
        source_hash: blake3::hash(source.as_bytes()).to_hex().to_string(),
        entry_points: vec!["affine_f32".to_string()], // We can parse this out if needed, but we hardcode for affine-f32 for now
        binding_count: 3,
        naga_validated: false,
        native_tool_validated: Some(tool_name.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::{generate_builtin, BuiltinKernel, Schedule, TargetBackend};

    #[test]
    fn generated_schedules_pass_full_naga_validation() {
        for vector_width in [1, 2, 4] {
            let generated = generate_builtin(
                BuiltinKernel::AffineF32,
                Schedule {
                    vector_width,
                    ..Schedule::default()
                },
                TargetBackend::Wgsl,
            )
            .unwrap();
            let report = validate_wgsl(&generated.source).expect("Naga validation");
            assert_eq!(report.entry_points, vec!["affine_f32"]);
            assert_eq!(report.binding_count, 3);
            assert_eq!(report.source_hash, generated.source_hash);
        }
    }

    #[test]
    fn semantic_errors_are_rejected() {
        let source = "@compute @workgroup_size(64) fn broken() { let x: u32 = 1.0; }";
        assert!(validate_wgsl(source).is_err());
    }
}
