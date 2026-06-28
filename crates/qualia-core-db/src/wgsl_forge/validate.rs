use serde::{Deserialize, Serialize};

use super::ForgeError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationReport {
    pub source_hash: String,
    pub entry_points: Vec<String>,
    pub binding_count: usize,
    pub naga_validated: bool,
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wgsl_forge::{generate_builtin, BuiltinKernel, Schedule};

    #[test]
    fn generated_schedules_pass_full_naga_validation() {
        for vector_width in [1, 2, 4] {
            let generated = generate_builtin(
                BuiltinKernel::AffineF32,
                Schedule {
                    vector_width,
                    ..Schedule::default()
                },
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
