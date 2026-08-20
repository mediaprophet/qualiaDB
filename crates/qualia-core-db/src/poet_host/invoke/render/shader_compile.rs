//! `Render.gpu_validate_shader` / `Render.gpu_compile_to_glsl` /
//! `Render.gpu_compile_shader` invoke handlers — runtime WGSL compilation,
//! validation, and hot-reload support (plan §7.3 W7).
//!
//! These handlers expose the naga WGSL → IR → GLSL ES 300 compilation pipeline
//! to VibeScript, enabling live shader editing in Studio. Validation and
//! GLSL cross-compilation work on all targets (pure naga, no GPU device
//! required); native shader module compilation requires a portal handle.
//!
//! ## Invoke surface
//!
//! | ID | Arguments | Returns |
//! |----|-----------|---------|
//! | `Render.gpu_validate_shader` | `{ wgsl }` | `{ valid, errors[], entry_points[] }` |
//! | `Render.gpu_compile_to_glsl` | `{ wgsl, entry?, stage? }` | `{ glsl, stage, entry_point }` |
//! | `Render.gpu_compile_shader` | `{ handle, wgsl, entry? }` | `{ compiled, shader_id, errors[] }` |

use super::super::args;
use poet_vibe::{Diagnostic, Span, Value};

#[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
use crate::render::naga_bridge::{compile_wgsl_to_glsl_es300, CompileError};
#[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
use crate::render::naga_sanitize;
#[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
use naga::ShaderStage;

/// Parse a stage string ("vertex"/"fragment"/"compute") into a naga ShaderStage.
#[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
fn parse_stage(s: &str) -> Result<ShaderStage, String> {
    match s {
        "vertex" | "vs" => Ok(ShaderStage::Vertex),
        "fragment" | "fs" => Ok(ShaderStage::Fragment),
        "compute" | "cs" => Ok(ShaderStage::Compute),
        other => Err(format!(
            "unknown stage '{other}' (expected vertex|fragment|compute)"
        )),
    }
}

/// Convert a naga CompileError into a diagnostic string.
#[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
fn compile_err_string(e: &CompileError) -> String {
    e.to_string()
}

/// `Render.gpu_validate_shader` — validate WGSL source without compiling.
///
/// Parses the WGSL, runs naga validation + WebGL2 sanitization, and returns
/// the list of entry points and any validation errors. Works on all targets
/// (no GPU device required).
pub fn gpu_validate_shader(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
    {
        let wgsl = args::rec_str(args, "wgsl")
            .ok_or_else(|| args::bad(span, "gpu_validate_shader needs { wgsl: string }"))?;

        let module = match naga::front::wgsl::parse_str(wgsl) {
            Ok(m) => m,
            Err(e) => {
                return Ok(args::record([
                    ("valid", Value::Bool(false)),
                    (
                        "errors",
                        Value::List(vec![Value::String(format!("WGSL parse error: {e}"))]),
                    ),
                    ("entry_points", Value::List(vec![])),
                ]));
            }
        };

        // Collect entry point names + stages.
        let entry_points: Vec<Value> = module
            .entry_points
            .iter()
            .map(|ep| {
                let stage = match ep.stage {
                    ShaderStage::Vertex => "vertex",
                    ShaderStage::Fragment => "fragment",
                    ShaderStage::Compute => "compute",
                    _ => "other",
                };
                args::record([
                    ("name", Value::String(ep.name.as_str().to_string())),
                    ("stage", Value::String(stage.to_string())),
                ])
            })
            .collect();

        // Run naga validation.
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        let mut errors: Vec<String> = Vec::new();
        if let Err(e) = validator.validate(&module) {
            errors.push(format!("naga validation: {e}"));
        }

        // Run WebGL2 sanitization on each entry point (catches f64, oversized
        // uniform blocks, workgroup-in-vertex, etc.).
        for ep in &module.entry_points {
            if let Err(e) = naga_sanitize::sanitize(&module, ep.name.as_str(), ep.stage) {
                errors.push(format!("sanitize [{}]: {e}", ep.name));
            }
        }

        let valid = errors.is_empty();
        Ok(args::record([
            ("valid", Value::Bool(valid)),
            (
                "errors",
                Value::List(errors.into_iter().map(Value::String).collect()),
            ),
            ("entry_points", Value::List(entry_points)),
        ]))
    }
    #[cfg(not(any(feature = "webgl2", feature = "wgsl-forge")))]
    {
        let _ = (args, span);
        Err(args::bad(
            span,
            "gpu_validate_shader requires the webgl2 or wgsl-forge feature",
        ))
    }
}

/// `Render.gpu_compile_to_glsl` — cross-compile WGSL to GLSL ES 300.
///
/// Takes WGSL source, an entry point name, and a stage, and returns the
/// compiled GLSL ES 300 source string. This is the WebGL2 fallback path:
/// Studio can request GLSL output for browsers without WebGPU. Works on all
/// targets (pure naga, no GPU device required).
pub fn gpu_compile_to_glsl(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
    {
        let wgsl = args::rec_str(args, "wgsl")
            .ok_or_else(|| args::bad(span, "gpu_compile_to_glsl needs { wgsl: string }"))?;
        let entry = args::rec_str(args, "entry").unwrap_or("main");
        let stage_str = args::rec_str(args, "stage").unwrap_or("vertex");
        let stage = parse_stage(stage_str)
            .map_err(|e| args::bad(span, format!("gpu_compile_to_glsl: {e}")))?;

        let compiled = compile_wgsl_to_glsl_es300(wgsl, entry, stage)
            .map_err(|e| args::bad(span, compile_err_string(&e)))?;

        Ok(args::record([
            ("glsl", Value::String(compiled.source)),
            ("stage", Value::String(stage_str.to_string())),
            ("entry_point", Value::String(compiled.entry_point)),
        ]))
    }
    #[cfg(not(any(feature = "webgl2", feature = "wgsl-forge")))]
    {
        let _ = (args, span);
        Err(args::bad(
            span,
            "gpu_compile_to_glsl requires the webgl2 or wgsl-forge feature",
        ))
    }
}

/// `Render.gpu_compile_shader` — compile WGSL to a native wgpu shader module.
///
/// On native targets, compiles the WGSL into a wgpu shader module stored in
/// the portal's shader cache, returning a `shader_id` handle. On wasm or
/// without GPU features, falls back to validation-only.
///
/// The shader handle can be used with future `gpu_compute_dispatch` variants
/// or render pipeline hot-reload. For immediate compute dispatch, use
/// `Render.gpu_compute_dispatch` directly (it accepts raw WGSL).
pub fn gpu_compile_shader(args: &Value, span: Span) -> Result<Value, Diagnostic> {
    #[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
    {
        let handle = args::rec_u64(args, "handle")
            .ok_or_else(|| args::bad(span, "gpu_compile_shader needs { handle: u64 }"))?;
        let wgsl = args::rec_str(args, "wgsl")
            .ok_or_else(|| args::bad(span, "gpu_compile_shader needs { wgsl: string }"))?;
        let entry = args::rec_str(args, "entry").unwrap_or("main").to_string();

        // Validate first (catch errors before GPU shader creation).
        let module = naga::front::wgsl::parse_str(wgsl)
            .map_err(|e| args::bad(span, format!("gpu_compile_shader: WGSL parse error: {e}")))?;
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator
            .validate(&module)
            .map_err(|e| args::bad(span, format!("gpu_compile_shader: naga validation: {e}")))?;

        let shader_id =
            super::gpu::slot_with(handle, |portal| portal.compile_shader_module(wgsl, &entry))
                .ok_or_else(|| args::bad(span, "gpu_compile_shader: invalid handle"))?
                .map_err(|e| args::bad(span, format!("gpu_compile_shader: {e}")))?;

        Ok(args::record([
            ("compiled", Value::Bool(true)),
            ("shader_id", Value::U64(shader_id)),
            ("entry_point", Value::String(entry)),
        ]))
    }
    #[cfg(not(all(not(target_arch = "wasm32"), feature = "gpu-runtime")))]
    {
        // Fallback: validate only (no GPU device available).
        let _ = (args, span);
        Err(args::bad(
            span,
            "gpu_compile_shader requires native build with gpu-runtime; use gpu_validate_shader for validation-only",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> crate::poet_host::PoetSnapshot {
        crate::poet_host::PoetSnapshot::default()
    }

    fn eval(src: &str) -> Value {
        let mut snap = snap();
        snap.eval_fn(src, "go", vec![]).expect("script should eval")
    }

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    const VALID_VERTEX_WGSL: &str = r#"
@vertex
fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> {
    return vec4<f32>(pos, 0.0, 1.0);
}
"#;

    const INVALID_WGSL: &str = "fn broken(";

    #[test]
    #[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
    fn g_gpu_validate_shader_valid() {
        let args = args::record([("wgsl", Value::String(VALID_VERTEX_WGSL.into()))]);
        let result = gpu_validate_shader(&args, dummy_span()).expect("validate");
        assert_eq!(args::rec(&result, "valid"), Some(&Value::Bool(true)));
        let Value::List(errors) = args::rec(&result, "errors").unwrap() else {
            panic!("no errors list")
        };
        assert!(errors.is_empty(), "should have no errors");
        let Value::List(eps) = args::rec(&result, "entry_points").unwrap() else {
            panic!("no entry_points list")
        };
        assert_eq!(eps.len(), 1, "should have 1 entry point");
    }

    #[test]
    #[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
    fn g_gpu_validate_shader_invalid() {
        let args = args::record([("wgsl", Value::String(INVALID_WGSL.into()))]);
        let result = gpu_validate_shader(&args, dummy_span()).expect("validate");
        assert_eq!(args::rec(&result, "valid"), Some(&Value::Bool(false)));
        let Value::List(errors) = args::rec(&result, "errors").unwrap() else {
            panic!("no errors list")
        };
        assert!(!errors.is_empty(), "should have errors");
    }

    #[test]
    #[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
    fn g_gpu_compile_to_glsl_vertex() {
        let args = args::record([
            ("wgsl", Value::String(VALID_VERTEX_WGSL.into())),
            ("entry", Value::String("vs_main".into())),
            ("stage", Value::String("vertex".into())),
        ]);
        let result = gpu_compile_to_glsl(&args, dummy_span()).expect("compile");
        let glsl = args::rec_str(&result, "glsl").expect("has glsl");
        assert!(
            glsl.contains("#version") || glsl.contains("void main"),
            "GLSL should contain version or main: {glsl}"
        );
    }

    #[test]
    #[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
    fn g_gpu_compile_to_glsl_invalid_errors() {
        let args = args::record([
            ("wgsl", Value::String(INVALID_WGSL.into())),
            ("entry", Value::String("main".into())),
            ("stage", Value::String("vertex".into())),
        ]);
        let result = gpu_compile_to_glsl(&args, dummy_span());
        assert!(result.is_err(), "invalid WGSL should error");
    }

    #[test]
    #[cfg(any(feature = "webgl2", feature = "wgsl-forge"))]
    fn g_gpu_validate_shader_via_vibescript() {
        let src = r#"
        requires [ capability("capability.invoke") ];
        effect fn go() {
            return capability.invoke("Render.gpu_validate_shader", {
                wgsl: "@vertex fn vs_main(@location(0) pos: vec2<f32>) -> @builtin(position) vec4<f32> { return vec4<f32>(pos, 0.0, 1.0); }"
            });
        }
        "#;
        let result = eval(src);
        assert!(
            matches!(result, Value::Record(_)),
            "expected record, got {result:?}"
        );
        if let Value::Record(m) = &result {
            assert_eq!(
                m.get("valid"),
                Some(&Value::Bool(true)),
                "shader should be valid"
            );
        }
    }
}
