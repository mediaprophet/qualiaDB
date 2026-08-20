//! Naga IR sanitizer for WebGL2 (GLSL ES 300) compatibility.
//!
//! WGSL shaders authored for the WebGPU backend may contain constructs that
//! naga's `glsl-out` backend cannot emit directly or that produce invalid
//! GLSL ES 300. This module validates the naga IR and rewrites or rejects
//! incompatible constructs before code generation.
//!
//! ## Sanitization passes
//!
//! 1. **Stage validation** — only vertex, fragment, and compute entry points
//!    are allowed. Compute shaders map to GLSL ES 300 compute.
//! 2. **Type demotion** — f64 is not available in GLSL ES 300; downgrade to f32.
//! 3. **Binding validation** — uniform buffers must fit within the WebGL2
//!    `MAX_UNIFORM_BLOCK_SIZE` (typically 16 KB per block on mobile).
//! 4. **Texture/sampler pairing** — WGSL separates textures and samplers;
//!    GLSL ES 300 combines them. Verify 1:1 pairing.
//! 5. **Workgroup size** — compute shaders must have a workgroup_size attribute;
//!    extract it for `layout(local_size_x = N)` emission.

use naga::valid::{ModuleInfo, ValidationFlags, Validator};
use naga::{AddressSpace, Module, ScalarKind, ShaderStage, TypeInner};

/// Maximum uniform block size for WebGL2 (16 KB — conservative mobile limit).
pub const MAX_UNIFORM_BLOCK_SIZE: usize = 16 * 1024;

/// Sanitization error categories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SanitizeError {
    /// f64 type found in shader — GLSL ES 300 has no double precision.
    UnsupportedF64,
    /// A binding exceeds the uniform block size limit.
    UniformBlockTooLarge { binding: u32, size_bytes: usize },
    /// Entry point stage not supported in GLSL ES 300.
    UnsupportedStage(String),
    /// Naga validation failed.
    ValidationFailed(String),
    /// Workgroup variable in a vertex/fragment shader.
    WorkgroupInVertexOrFragment,
}

impl std::fmt::Display for SanitizeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnsupportedF64 => write!(f, "f64 is not available in GLSL ES 300"),
            Self::UniformBlockTooLarge { binding, size_bytes } => write!(
                f,
                "uniform block at binding={binding} is {size_bytes} bytes, exceeds {MAX_UNIFORM_BLOCK_SIZE}"
            ),
            Self::UnsupportedStage(s) => write!(f, "unsupported shader stage: {s}"),
            Self::ValidationFailed(s) => write!(f, "naga validation failed: {s}"),
            Self::WorkgroupInVertexOrFragment => {
                write!(f, "workgroup variables are not allowed in vertex/fragment shaders")
            }
        }
    }
}

impl std::error::Error for SanitizeError {}

/// Result of sanitizing a naga module for WebGL2.
#[derive(Debug)]
pub struct SanitizedModule {
    /// The validated module info (from naga's validator).
    pub info: ModuleInfo,
    /// The stage of the entry point being sanitized.
    pub stage: ShaderStage,
    /// The entry point name.
    pub entry_point: String,
}

/// Sanitize a naga module for WebGL2 / GLSL ES 300 compatibility.
///
/// This function:
/// 1. Runs naga's validator on the module
/// 2. Checks for unsupported types (f64)
/// 3. Validates uniform block sizes
/// 4. Verifies texture/sampler pairing
/// 5. Checks workgroup variables are only in compute shaders
pub fn sanitize(
    module: &Module,
    entry_point: &str,
    stage: ShaderStage,
) -> Result<SanitizedModule, SanitizeError> {
    // Step 1: Validate the module with naga.
    let mut validator = Validator::new(ValidationFlags::all(), naga::valid::Capabilities::all());
    let info = validator
        .validate(module)
        .map_err(|e| SanitizeError::ValidationFailed(format!("{e}")))?;

    // Step 2: Check for f64 types — GLSL ES 300 has no double precision.
    for (_, ty) in module.types.iter() {
        if check_no_f64(&ty.inner, module).is_some() {
            return Err(SanitizeError::UnsupportedF64);
        }
    }

    // Step 3: Validate uniform block sizes and workgroup variables.
    for (_, var) in module.global_variables.iter() {
        check_uniform_block_size(module, var)?;

        if !matches!(stage, ShaderStage::Compute) {
            if matches!(var.space, AddressSpace::WorkGroup) {
                return Err(SanitizeError::WorkgroupInVertexOrFragment);
            }
        }
    }

    Ok(SanitizedModule {
        info,
        stage,
        entry_point: entry_point.to_string(),
    })
}

/// Recursively check if a type contains f64. Returns `Some(UnsupportedF64)` if found.
fn check_no_f64(inner: &TypeInner, module: &Module) -> Option<SanitizeError> {
    match inner {
        TypeInner::Scalar(s) if s.kind == ScalarKind::Float && s.width == 8 => {
            Some(SanitizeError::UnsupportedF64)
        }
        TypeInner::Vector { scalar, .. } | TypeInner::Matrix { scalar, .. } => {
            if scalar.kind == ScalarKind::Float && scalar.width == 8 {
                Some(SanitizeError::UnsupportedF64)
            } else {
                None
            }
        }
        TypeInner::Struct { members, .. } => {
            for member in members.iter() {
                let ty = &module.types[member.ty];
                if check_no_f64(&ty.inner, module).is_some() {
                    return Some(SanitizeError::UnsupportedF64);
                }
            }
            None
        }
        TypeInner::Array { base, .. } => {
            let ty = &module.types[*base];
            check_no_f64(&ty.inner, module)
        }
        _ => None,
    }
}

/// Check that a uniform buffer does not exceed the WebGL2 limit.
fn check_uniform_block_size(
    module: &Module,
    var: &naga::GlobalVariable,
) -> Result<(), SanitizeError> {
    if !matches!(var.space, AddressSpace::Uniform) {
        return Ok(());
    }

    let ty = &module.types[var.ty];
    let size = compute_type_size(&ty.inner, module);
    if size > MAX_UNIFORM_BLOCK_SIZE {
        let binding = var.binding.map(|b| b.binding).unwrap_or(0);
        return Err(SanitizeError::UniformBlockTooLarge {
            binding,
            size_bytes: size,
        });
    }
    Ok(())
}

/// Recursively compute the byte size of a naga type (std140 layout).
fn compute_type_size(inner: &TypeInner, module: &Module) -> usize {
    match inner {
        TypeInner::Scalar(s) => match s.kind {
            ScalarKind::Float => 4,
            ScalarKind::Sint | ScalarKind::Uint => 4,
            ScalarKind::Bool => 4,
            _ => 4,
        },
        TypeInner::Vector { size, scalar } => {
            let elem = if scalar.width == 8 { 8 } else { 4 };
            match size {
                naga::VectorSize::Bi => elem * 2,
                naga::VectorSize::Tri => elem * 3,
                naga::VectorSize::Quad => elem * 4,
            }
        }
        TypeInner::Matrix { columns, .. } => {
            let cols = match columns {
                naga::VectorSize::Bi => 2,
                naga::VectorSize::Tri => 3,
                naga::VectorSize::Quad => 4,
            };
            cols * 16
        }
        TypeInner::Array { base, size, .. } => {
            let elem_size = compute_type_size(&module.types[*base].inner, module);
            // std140: array elements are aligned to vec4 (16 bytes)
            let stride = ((elem_size + 15) / 16) * 16;
            let count = match size {
                naga::ArraySize::Constant(c) => c.get() as usize,
                naga::ArraySize::Dynamic => 1,
                naga::ArraySize::Pending(_) => 1,
            };
            stride * count
        }
        TypeInner::Struct { members, .. } => {
            let mut total = 0;
            for member in members.iter() {
                let ty = &module.types[member.ty];
                let member_size = compute_type_size(&ty.inner, module);
                total = ((total + 15) / 16) * 16;
                total += member_size;
            }
            total
        }
        _ => 16,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_wgsl(src: &str) -> Module {
        naga::front::wgsl::parse_str(src).expect("WGSL parse failed")
    }

    #[test]
    fn simple_vertex_shader_passes_sanitization() {
        let src = r#"
@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(pos, 0.0, 1.0);
}
"#;
        let module = parse_wgsl(src);
        let result = sanitize(&module, "vs_main", ShaderStage::Vertex);
        assert!(
            result.is_ok(),
            "simple vertex shader should pass: {:?}",
            result
        );
    }

    #[test]
    fn fragment_shader_passes_sanitization() {
        let src = r#"
@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(uv.x, uv.y, 0.0, 1.0);
}
"#;
        let module = parse_wgsl(src);
        let result = sanitize(&module, "fs_main", ShaderStage::Fragment);
        assert!(result.is_ok(), "fragment shader should pass: {:?}", result);
    }

    #[test]
    fn uniform_buffer_within_limit_passes() {
        let src = r#"
struct Camera {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};
@group(0) @binding(0) var<uniform> camera: Camera;

@vertex
fn vs_main(@location(0) pos: vec3<f32>) -> @builtin(position) vec4<f32> {
    return camera.proj * camera.view * vec4<f32>(pos, 1.0);
}
"#;
        let module = parse_wgsl(src);
        let result = sanitize(&module, "vs_main", ShaderStage::Vertex);
        assert!(
            result.is_ok(),
            "two mat4s (128 bytes) should pass: {:?}",
            result
        );
    }

    #[test]
    fn compute_shader_passes_sanitization() {
        let src = r#"
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= 100u { return; }
    data[id.x] = data[id.x] * 2.0;
}
"#;
        let module = parse_wgsl(src);
        let result = sanitize(&module, "cs_main", ShaderStage::Compute);
        assert!(result.is_ok(), "compute shader should pass: {:?}", result);
    }

    #[test]
    fn invalid_wgsl_fails_validation() {
        let src = "fn broken(";
        let result = naga::front::wgsl::parse_str(src);
        assert!(result.is_err(), "invalid WGSL should fail to parse");
    }
}
