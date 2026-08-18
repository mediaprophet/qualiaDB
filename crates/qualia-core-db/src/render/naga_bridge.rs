//! Naga bridge: WGSL source → naga IR → GLSL ES 300 compilation.
//!
//! This module wraps naga's `glsl-out` backend, providing a clean API for
//! compiling WGSL shaders to WebGL2-compatible GLSL ES 300. It integrates
//! with `naga_sanitize` to validate the IR before code generation.
//!
//! ## Pipeline
//!
//! 1. Parse WGSL source → `naga::Module`
//! 2. Sanitize the module (`naga_sanitize::sanitize`)
//! 3. Generate GLSL ES 300 via `naga::back::glsl::write`

use naga::back::glsl::{Options, PipelineOptions, Version, Writer, WriterFlags};
use naga::proc::BoundsCheckPolicies;
use std::collections::BTreeMap;
use naga::front::wgsl;
use naga::ShaderStage;

use super::naga_sanitize::{sanitize, SanitizeError};

/// GLSL ES 300 compilation error.
#[derive(Debug)]
pub enum CompileError {
    /// WGSL parsing failed.
    WgslParse(String),
    /// Sanitization failed — the shader is not WebGL2-compatible.
    Sanitize(SanitizeError),
    /// GLSL code generation failed.
    GlslEmit(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WgslParse(s) => write!(f, "WGSL parse error: {s}"),
            Self::Sanitize(e) => write!(f, "sanitization error: {e}"),
            Self::GlslEmit(s) => write!(f, "GLSL emission error: {s}"),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<SanitizeError> for CompileError {
    fn from(e: SanitizeError) -> Self {
        Self::Sanitize(e)
    }
}

/// Compiled GLSL ES 300 shader output.
#[derive(Debug, Clone)]
pub struct CompiledGlsl {
    /// The GLSL ES 300 source string.
    pub source: String,
    /// The shader stage.
    pub stage: ShaderStage,
    /// The entry point name in the original WGSL.
    pub entry_point: String,
}

/// Compile a WGSL shader to GLSL ES 300 for a specific entry point and stage.
///
/// # Example
///
/// ```ignore
/// let wgsl = r#"
/// @vertex
/// fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
///     return vec4<f32>(pos, 0.0, 1.0);
/// }
/// "#;
/// let glsl = compile_wgsl_to_glsl_es300(wgsl, "vs_main", ShaderStage::Vertex).unwrap();
/// assert!(glsl.source.contains("#version 300 es"));
/// ```
pub fn compile_wgsl_to_glsl_es300(
    wgsl_src: &str,
    entry_point: &str,
    stage: ShaderStage,
) -> Result<CompiledGlsl, CompileError> {
    // Step 1: Parse WGSL.
    let module = wgsl::parse_str(wgsl_src)
        .map_err(|e| CompileError::WgslParse(format!("{e}")))?;

    // Step 2: Sanitize for WebGL2.
    let sanitized = sanitize(&module, entry_point, stage)?;

    // Step 3: Configure GLSL ES 300 options.
    let options = Options {
        version: Version::Embedded { version: 300, is_webgl: true },
        zero_initialize_workgroup_memory: false,
        binding_map: BTreeMap::new(),
        writer_flags: WriterFlags::empty(),
    };
    let pipeline = PipelineOptions {
        shader_stage: stage,
        entry_point: entry_point.to_string(),
        multiview: None,
    };

    // Step 4: Generate GLSL.
    let mut output = String::new();
    let mut writer = Writer::new(
        &mut output,
        &module,
        &sanitized.info,
        &options,
        &pipeline,
        BoundsCheckPolicies::default(),
    )
    .map_err(|e| CompileError::GlslEmit(format!("{e}")))?;

    writer
        .write()
        .map_err(|e| CompileError::GlslEmit(format!("{e}")))?;

    Ok(CompiledGlsl {
        source: output,
        stage,
        entry_point: entry_point.to_string(),
    })
}

/// Compile all entry points in a WGSL module to GLSL ES 300.
///
/// Returns a vector of compiled shaders, one per entry point.
pub fn compile_all_entry_points(
    wgsl_src: &str,
) -> Result<Vec<CompiledGlsl>, CompileError> {
    let module = wgsl::parse_str(wgsl_src)
        .map_err(|e| CompileError::WgslParse(format!("{e}")))?;

    let mut results = Vec::new();

    for ep in &module.entry_points {
        let stage = ep.stage;
        let name = ep.name.as_str();

        let sanitized = sanitize(&module, name, stage)?;

        let options = Options {
            version: Version::Embedded { version: 300, is_webgl: true },
            zero_initialize_workgroup_memory: false,
            binding_map: BTreeMap::new(),
            writer_flags: WriterFlags::empty(),
        };
        let pipeline = PipelineOptions {
            shader_stage: stage,
            entry_point: name.to_string(),
            multiview: None,
        };

        let mut output = String::new();
        let mut writer = Writer::new(
            &mut output,
            &module,
            &sanitized.info,
            &options,
            &pipeline,
            BoundsCheckPolicies::default(),
        )
        .map_err(|e| CompileError::GlslEmit(format!("{e}")))?;

        writer
            .write()
            .map_err(|e| CompileError::GlslEmit(format!("{e}")))?;

        results.push(CompiledGlsl {
            source: output,
            stage,
            entry_point: name.to_string(),
        });
    }

    Ok(results)
}

/// Generate a std140-aligned uniform buffer layout from a WGSL struct.
///
/// This is used by the host side to pack uniform data before uploading
/// to WebGL2. The layout follows std140 rules:
/// - scalars: 4 bytes
/// - vec2: 8 bytes (aligned 8)
/// - vec3: 12 bytes (aligned 16)
/// - vec4: 16 bytes (aligned 16)
/// - mat4x4: 64 bytes (16 bytes per column, aligned 16)
/// - arrays: each element aligned to vec4 (16 bytes)
/// - structs: aligned to vec4 (16 bytes), size rounded up to 16
pub fn std140_layout(wgsl_struct_src: &str) -> Result<Vec<Std140Member>, CompileError> {
    // Parse as a full module with a dummy entry point.
    let full_src = format!(
        "{wgsl_struct_src}\n@vertex fn _dummy() -> @builtin(position) vec4<f32> {{ return vec4<f32>(0.0); }}"
    );
    let module = wgsl::parse_str(&full_src)
        .map_err(|e| CompileError::WgslParse(format!("{e}")))?;

    // Find the struct type.
    let mut layout = Vec::new();

    for (_, ty) in module.types.iter() {
        if let naga::TypeInner::Struct { members: struct_members, .. } = &ty.inner {
            let mut offset = 0u32;
            for member in struct_members {
                let (size, align) = std140_size_align(&member.ty, &module);
                // Align offset
                offset = ((offset + align - 1) / align) * align;
                layout.push(Std140Member {
                    name: member.name.clone().unwrap_or_default(),
                    offset,
                    size,
                    align,
                });
                offset += size;
            }
            // Round up struct size to alignment of largest member (at least 16 for std140)
            let struct_align = layout
                .iter()
                .map(|m| m.align)
                .max()
                .unwrap_or(16)
                .max(16);
            let _ = struct_align;
            let _struct_size = ((offset + struct_align - 1) / struct_align) * struct_align;
            break;
        }
    }

    Ok(layout)
}

/// A std140-aligned uniform buffer member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Std140Member {
    /// Field name in the WGSL struct.
    pub name: String,
    /// Byte offset within the uniform buffer.
    pub offset: u32,
    /// Byte size of the member.
    pub size: u32,
    /// Byte alignment of the member.
    pub align: u32,
}

fn std140_size_align(
    ty: &naga::Handle<naga::Type>,
    module: &naga::Module,
) -> (u32, u32) {
    let type_inner = &module.types[*ty];

    match &type_inner.inner {
        naga::TypeInner::Scalar(s) => {
            let size = if s.width == 8 { 8 } else { 4 };
            (size, size)
        }
        naga::TypeInner::Vector { size, scalar } => {
            let elem = if scalar.width == 8 { 8 } else { 4 };
            match size {
                naga::VectorSize::Bi => (elem * 2, elem * 2),
                naga::VectorSize::Tri => (elem * 3, 16), // vec3 is aligned to vec4 in std140
                naga::VectorSize::Quad => (elem * 4, elem * 4),
            }
        }
        naga::TypeInner::Matrix { columns, scalar, .. } => {
            let _elem = if scalar.width == 8 { 8 } else { 4 };
            let cols = match columns {
                naga::VectorSize::Bi => 2,
                naga::VectorSize::Tri => 3,
                naga::VectorSize::Quad => 4,
            };
            // std140: matCxR is an array of C vec4s (16 bytes each)
            (cols as u32 * 16, 16)
        }
        naga::TypeInner::Array { base, size, .. } => {
            let (elem_size, _) = std140_size_align(base, module);
            let stride = ((elem_size + 15) / 16) * 16; // Round up to 16
            let count = match size {
                naga::ArraySize::Constant(c) => c.get() as u32,
                naga::ArraySize::Dynamic => 1,
                naga::ArraySize::Pending(_) => 1,
            };
            (stride * count, stride)
        }
        naga::TypeInner::Struct { members, .. } => {
            let mut total = 0u32;
            let mut max_align = 16u32;
            for member in members {
                let (size, align) = std140_size_align(&member.ty, module);
                max_align = max_align.max(align);
                total = ((total + align - 1) / align) * align;
                total += size;
            }
            let struct_size = ((total + 15) / 16) * 16;
            (struct_size, max_align.max(16))
        }
        _ => (16, 16),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_simple_vertex_shader() {
        let wgsl = r#"
@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(pos, 0.0, 1.0);
}
"#;
        let result = compile_wgsl_to_glsl_es300(wgsl, "vs_main", ShaderStage::Vertex);
        assert!(result.is_ok(), "compilation should succeed: {:?}", result);
        let glsl = result.unwrap();
        assert!(
            glsl.source.contains("#version 300 es") || glsl.source.contains("#version"),
            "GLSL should contain version directive: {}",
            glsl.source
        );
    }

    #[test]
    fn compile_fragment_shader() {
        let wgsl = r#"
@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(uv.x, uv.y, 0.0, 1.0);
}
"#;
        let result = compile_wgsl_to_glsl_es300(wgsl, "fs_main", ShaderStage::Fragment);
        assert!(result.is_ok(), "fragment compilation should succeed: {:?}", result);
    }

    #[test]
    fn compile_uniform_buffer_vertex_shader() {
        let wgsl = r#"
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
        let result = compile_wgsl_to_glsl_es300(wgsl, "vs_main", ShaderStage::Vertex);
        assert!(result.is_ok(), "uniform buffer shader should compile: {:?}", result);
    }

    #[test]
    fn compile_compute_shader_not_supported_in_glsl_es300() {
        let wgsl = r#"
@group(0) @binding(0) var<storage, read_write> data: array<f32>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    if id.x >= 100u { return; }
    data[id.x] = data[id.x] * 2.0;
}
"#;
        let result = compile_wgsl_to_glsl_es300(wgsl, "cs_main", ShaderStage::Compute);
        assert!(result.is_err(), "compute shaders are not supported in GLSL ES 300: {:?}", result);
    }

    #[test]
    fn compile_all_entry_points_in_multi_shader() {
        let wgsl = r#"
@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(pos, 0.0, 1.0);
}

@fragment
fn fs_main(@location(0) uv: vec2<f32>) -> @location(0) vec4<f32> {
    return vec4<f32>(uv.x, uv.y, 0.0, 1.0);
}
"#;
        let results = compile_all_entry_points(wgsl);
        assert!(results.is_ok(), "multi-entry-point compilation should succeed: {:?}", results);
        let shaders = results.unwrap();
        assert_eq!(shaders.len(), 2, "should have 2 entry points");
        assert!(shaders.iter().any(|s| s.stage == ShaderStage::Vertex));
        assert!(shaders.iter().any(|s| s.stage == ShaderStage::Fragment));
    }

    #[test]
    fn invalid_wgsl_produces_parse_error() {
        let result = compile_wgsl_to_glsl_es300("fn broken(", "broken", ShaderStage::Vertex);
        assert!(matches!(result, Err(CompileError::WgslParse(_))));
    }

    #[test]
    fn std140_layout_for_camera_struct() {
        let wgsl = r#"
struct Camera {
    view: mat4x4<f32>,
    proj: mat4x4<f32>,
};
"#;
        let layout = std140_layout(wgsl);
        assert!(layout.is_ok(), "std140 layout should succeed: {:?}", layout);
        let members = layout.unwrap();
        assert_eq!(members.len(), 2, "Camera should have 2 members");
        assert_eq!(members[0].name, "view");
        assert_eq!(members[0].offset, 0);
        assert_eq!(members[0].size, 64, "mat4x4 is 64 bytes");
        assert_eq!(members[1].name, "proj");
        assert_eq!(members[1].offset, 64, "proj should be at offset 64");
    }

    #[test]
    fn std140_layout_vec3_alignment() {
        let wgsl = r#"
struct Data {
    a: vec3<f32>,
    b: f32,
};
"#;
        let layout = std140_layout(wgsl);
        assert!(layout.is_ok());
        let members = layout.unwrap();
        assert_eq!(members[0].name, "a");
        assert_eq!(members[0].offset, 0);
        assert_eq!(members[0].size, 12, "vec3 is 12 bytes");
        assert_eq!(members[0].align, 16, "vec3 aligns to 16 in std140");
        // 'b' should be at offset 12 (within the vec4 padding of 'a')
        assert_eq!(members[1].name, "b");
        assert_eq!(members[1].offset, 12, "f32 after vec3 should be at offset 12");
    }
}
