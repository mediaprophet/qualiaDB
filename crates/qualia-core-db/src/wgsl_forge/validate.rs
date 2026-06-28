use serde::{Deserialize, Serialize};

use crate::wgsl_forge::ForgeError;
use crate::wgsl_forge::KernelSpec;
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
    // Enabling capabilities only widens what is accepted, so kernels that do not
    // use ray-query or cooperative-matrix features are unaffected.
    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::RAY_QUERY
            | naga::valid::Capabilities::COOPERATIVE_MATRIX
            | naga::valid::Capabilities::SHADER_FLOAT16,
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

/// Validate a non-WGSL shader by spawning the target's offline compiler. When the
/// source was generated from a known [`KernelSpec`], pass it so the report carries
/// that kernel's real entry point and binding count; for an opaque external source
/// (`kernel = None`) those fields are left empty/zero rather than guessed.
pub fn validate_native(
    source: &str,
    target: TargetBackend,
    kernel: Option<&KernelSpec>,
) -> Result<ValidationReport, ForgeError> {
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

    // The native compiler validated the source; report the kernel's real entry
    // point + binding count when known, else leave them empty for an opaque file.
    let (entry_points, binding_count) = match kernel {
        Some(k) => (vec![k.entry_point.clone()], k.buffers.len()),
        None => (Vec::new(), 0),
    };
    Ok(ValidationReport {
        source_hash: blake3::hash(source.as_bytes()).to_hex().to_string(),
        entry_points,
        binding_count,
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
    fn generated_topk_passes_full_naga_validation() {
        // Exercises workgroup-shared memory + barrier uniformity in the IR path.
        for workgroup_size in [32u32, 64, 128, 256] {
            let generated = generate_builtin(
                BuiltinKernel::TopK,
                Schedule {
                    workgroup_size,
                    items_per_invocation: 1,
                    vector_width: 1,
                    ..Schedule::default()
                },
                TargetBackend::Wgsl,
            )
            .unwrap();
            assert!(
                generated.source.contains("workgroupBarrier()"),
                "top-k must emit barriers"
            );
            assert!(
                generated
                    .source
                    .contains(&format!("array<f32, {workgroup_size}>")),
                "shared arrays sized to workgroup size"
            );
            let report = validate_wgsl(&generated.source).expect("Naga validation of top-k");
            assert_eq!(report.entry_points, vec!["topk"]);
            assert_eq!(report.binding_count, 3);
        }
    }

    #[test]
    fn generated_p64_passes_naga_validation() {
        let generated = generate_builtin(
            BuiltinKernel::P64Project,
            Schedule {
                workgroup_size: 64,
                items_per_invocation: 1,
                vector_width: 1,
                ..Schedule::default()
            },
            TargetBackend::Wgsl,
        )
        .unwrap();
        assert!(generated.source.contains("struct P64Words64"));
        assert!(generated.source.contains("arrayLength(&output)"));
        let report = validate_wgsl(&generated.source).expect("Naga validation of p64-project");
        assert_eq!(report.entry_points, vec!["p64_project"]);
        assert_eq!(report.binding_count, 3);
    }

    #[test]
    fn generated_ffn_passes_naga_validation() {
        let generated = generate_builtin(
            BuiltinKernel::FusedFfn,
            Schedule {
                workgroup_size: 64,
                items_per_invocation: 1,
                vector_width: 1,
                ..Schedule::default()
            },
            TargetBackend::Wgsl,
        )
        .unwrap();
        assert!(generated.source.contains("struct FfnParams"));
        assert!(generated.source.contains("tanh("));
        let report = validate_wgsl(&generated.source).expect("Naga validation of fused-ffn");
        assert_eq!(report.entry_points, vec!["fused_ffn"]);
        assert_eq!(report.binding_count, 5);
    }

    #[test]
    fn generated_ray_probe_passes_naga_validation() {
        // Exercises the acceleration_structure binding and the ray_query lowering.
        let generated = generate_builtin(
            BuiltinKernel::RayProbe,
            Schedule {
                workgroup_size: 64,
                items_per_invocation: 1,
                vector_width: 1,
                ..Schedule::default()
            },
            TargetBackend::Wgsl,
        )
        .unwrap();
        assert!(generated.source.contains("var scene: acceleration_structure;"));
        assert!(generated.source.contains("rayQueryInitialize"));
        assert!(generated.source.contains("rayQueryGetCommittedIntersection"));
        let report = validate_wgsl(&generated.source).expect("Naga validation of ray-probe");
        assert_eq!(report.entry_points, vec!["ray_probe"]);
        assert_eq!(report.binding_count, 3);
    }

    #[test]
    fn cooperative_matrix_tile_validates() {
        // Single 8x8x8 tensor-core tile: C = A * B (one subgroup cooperative).
        let source = crate::wgsl_forge::matmul_tc_wgsl();
        let report = validate_wgsl(&source).expect("coopmat tile should validate");
        assert_eq!(report.entry_points, vec!["matmul_tc"]);
    }

    #[test]
    fn semantic_errors_are_rejected() {
        let source = "@compute @workgroup_size(64) fn broken() { let x: u32 = 1.0; }";
        assert!(validate_wgsl(source).is_err());
    }
}
