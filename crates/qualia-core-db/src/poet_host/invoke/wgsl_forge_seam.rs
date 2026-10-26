//! WGSL Forge & Shaders VibeScript invoke seam (`WgslForge.*` and `Shaders.list`).
//!
//! Exposes deterministic WGSL/SPIR-V/HLSL/PTX/CUDA/MSL kernel generation, Naga
//! validation, Roofline arithmetic-intensity analysis, schedule pruning, and
//! causal-attention synthesis to VibeScript and POET Studio.
//! Follows `AGENTS.md` §0-B (<500 lines).

#[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
use std::str::FromStr;

use super::args;
use super::ids;
#[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
use crate::wgsl_forge::{
    generate_builtin, roofline_for, synthesize_causal_attention, validate_wgsl, AdapterConstraints,
    AttentionSchedule, AttentionShape, BuiltinKernel, Schedule, ScheduleSpace, TargetBackend,
};
use vibe::{Diagnostic, Span, Value};

const VIEWPORT_PIPELINES: [&str; 8] = [
    "cyber_glass",
    "barnes_hut_graph_physics",
    "wire_particles",
    "manifold_grid_10d",
    "aurora_curtains",
    "terrain_pbr_pom",
    "ocean_gerstner",
    "black_hole_lensing",
];

#[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
pub fn list_kernels(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let filter = args::rec_str(args_v, "filter").unwrap_or("");
    let mut kernels = Vec::new();
    for builtin in BuiltinKernel::ALL {
        let name = builtin.name();
        if !filter.is_empty() && !name.contains(filter) {
            continue;
        }
        let spec = builtin.spec();
        kernels.push(args::record([
            ("name", Value::String(name.into())),
            ("version", Value::U64(spec.semantic_version as u64)),
            ("description", Value::String(spec.description.into())),
            ("has_gpu_oracle", Value::Bool(builtin.has_gpu_oracle())),
        ]));
    }

    let viewport_pipelines: Vec<Value> = VIEWPORT_PIPELINES
        .iter()
        .map(|s| Value::String((*s).into()))
        .collect();

    Ok(args::record([
        ("op", Value::String("list_kernels".into())),
        ("id", Value::String(ids::WGSL_FORGE_LIST_KERNELS.into())),
        ("kernel_count", Value::U64(kernels.len() as u64)),
        ("kernels", Value::List(kernels)),
        ("viewport_pipelines", Value::List(viewport_pipelines)),
    ]))
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "wgsl-forge")))]
pub fn list_kernels(_args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let _ = span;
    // Discovery reports actual target availability, not a synthetic catalogue.
    Ok(args::record([
        ("op", Value::String("list_kernels".into())),
        ("id", Value::String(ids::WGSL_FORGE_LIST_KERNELS.into())),
        ("available", Value::Bool(false)),
        ("native_service_required", Value::Bool(true)),
        ("kernel_count", Value::U64(0)),
        ("kernels", Value::List(Vec::new())),
        ("viewport_pipelines", Value::List(Vec::new())),
    ]))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
pub fn generate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let kernel_name = args::rec_str(args_v, "kernel").unwrap_or("affine-f32");
    let target_str = args::rec_str(args_v, "target").unwrap_or("wgsl");
    let workgroup_size = args::rec_u64(args_v, "workgroup_size").unwrap_or(64) as u32;
    let items_per_invocation = args::rec_u64(args_v, "items_per_invocation").unwrap_or(1) as u32;
    let vector_width = args::rec_u64(args_v, "vector_width").unwrap_or(1) as u32;

    let builtin = BuiltinKernel::from_str(kernel_name)
        .map_err(|e| args::bad(span, format!("WgslForge.generate unknown kernel: {e}")))?;
    let target = TargetBackend::from_str(target_str)
        .map_err(|e| args::bad(span, format!("WgslForge.generate invalid target: {e}")))?;
    let schedule = Schedule {
        workgroup_size,
        items_per_invocation,
        vector_width,
        ..Default::default()
    };

    let generated = generate_builtin(builtin, schedule, target)
        .map_err(|e| args::bad(span, format!("WgslForge.generate failed: {e}")))?;

    Ok(args::record([
        ("op", Value::String("generate".into())),
        ("id", Value::String(ids::WGSL_FORGE_GENERATE.into())),
        ("kernel_id", Value::String(generated.kernel_id)),
        ("target", Value::String(target_str.into())),
        ("entry_point", Value::String("main".into())),
        ("semantic_hash", Value::String(generated.semantic_hash)),
        ("source_hash", Value::String(generated.source_hash)),
        ("workgroup_size", Value::U64(workgroup_size as u64)),
        ("source", Value::String(generated.source)),
    ]))
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "wgsl-forge")))]
pub fn generate(_args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::bad(span, "WgslForge.generate requires the enabled native WGSL Forge service; no shader, validation or execution result was produced"))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
pub fn validate(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let source_owned: String = if let Some(src) = args::rec_str(args_v, "source") {
        src.to_string()
    } else {
        let kernel_name = args::rec_str(args_v, "kernel").unwrap_or("affine-f32");
        let workgroup_size = args::rec_u64(args_v, "workgroup_size").unwrap_or(64) as u32;
        let items_per_invocation =
            args::rec_u64(args_v, "items_per_invocation").unwrap_or(1) as u32;
        let vector_width = args::rec_u64(args_v, "vector_width").unwrap_or(1) as u32;
        let builtin = BuiltinKernel::from_str(kernel_name)
            .map_err(|e| args::bad(span, format!("WgslForge.validate unknown kernel: {e}")))?;
        let schedule = Schedule {
            workgroup_size,
            items_per_invocation,
            vector_width,
            ..Default::default()
        };
        generate_builtin(builtin, schedule, TargetBackend::Wgsl)
            .map_err(|e| args::bad(span, format!("WgslForge.validate generation failed: {e}")))?
            .source
    };

    let report = validate_wgsl(&source_owned)
        .map_err(|e| args::bad(span, format!("WgslForge.validate Naga error: {e}")))?;
    let entry_points: Vec<Value> = report
        .entry_points
        .into_iter()
        .map(Value::String)
        .collect();

    Ok(args::record([
        ("op", Value::String("validate".into())),
        ("id", Value::String(ids::WGSL_FORGE_VALIDATE.into())),
        ("naga_validated", Value::Bool(report.naga_validated)),
        ("binding_count", Value::U64(report.binding_count as u64)),
        ("entry_points", Value::List(entry_points)),
    ]))
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "wgsl-forge")))]
pub fn validate(_args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::bad(span, "WgslForge.validate requires the enabled native WGSL Forge service; no shader, validation or execution result was produced"))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
pub fn roofline(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let kernel_name = args::rec_str(args_v, "kernel").unwrap_or("affine-f32");
    let n = args::rec_u64(args_v, "n").unwrap_or(65_536).max(1);
    let builtin = BuiltinKernel::from_str(kernel_name)
        .map_err(|e| args::bad(span, format!("WgslForge.roofline unknown kernel: {e}")))?;
    let est = roofline_for(builtin, n);

    Ok(args::record([
        ("op", Value::String("roofline".into())),
        ("id", Value::String(ids::WGSL_FORGE_ROOFLINE.into())),
        ("kernel", Value::String(builtin.name().into())),
        ("n", Value::U64(n)),
        ("flops", Value::U64(est.flops)),
        ("bytes", Value::U64(est.bytes)),
        ("arithmetic_intensity", Value::F64(est.arithmetic_intensity)),
        ("bound", Value::String(format!("{:?}", est.bound))),
    ]))
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "wgsl-forge")))]
pub fn roofline(_args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::bad(span, "WgslForge.roofline requires the enabled native WGSL Forge service; no shader, validation or execution result was produced"))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
pub fn schedule_candidates(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let kernel_name = args::rec_str(args_v, "kernel").unwrap_or("affine-f32");
    let builtin = BuiltinKernel::from_str(kernel_name).map_err(|e| {
        args::bad(
            span,
            format!("WgslForge.schedule_candidates unknown kernel: {e}"),
        )
    })?;
    let spec = builtin.spec();
    let constraints = AdapterConstraints::portable();
    let space = ScheduleSpace::default();
    let candidates = space.candidates(&spec, &constraints);

    let list: Vec<Value> = candidates
        .iter()
        .map(|c| {
            args::record([
                ("workgroup_size", Value::U64(c.workgroup_size as u64)),
                (
                    "items_per_invocation",
                    Value::U64(c.items_per_invocation as u64),
                ),
                ("vector_width", Value::U64(c.vector_width as u64)),
            ])
        })
        .collect();

    Ok(args::record([
        ("op", Value::String("schedule_candidates".into())),
        ("id", Value::String(ids::WGSL_FORGE_SCHEDULE_CANDIDATES.into())),
        ("kernel", Value::String(builtin.name().into())),
        ("candidate_count", Value::U64(list.len() as u64)),
        ("candidates", Value::List(list)),
    ]))
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "wgsl-forge")))]
pub fn schedule_candidates(_args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::bad(span, "WgslForge.schedule_candidates requires the enabled native WGSL Forge service; no shader, validation or execution result was produced"))
}

#[cfg(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))]
pub fn attention_synthesize(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let head_dim = args::rec_u64(args_v, "head_dim").unwrap_or(64) as u32;
    let head_count = args::rec_u64(args_v, "head_count").unwrap_or(16) as u32;
    let context_tokens = args::rec_u64(args_v, "context_tokens").unwrap_or(512) as u32;
    let batch_tokens = args::rec_u64(args_v, "batch_tokens").unwrap_or(1) as u32;
    let mode = args::rec_str(args_v, "mode").unwrap_or("decode");

    let shape = AttentionShape {
        head_dim,
        head_count,
        context_tokens,
        batch_tokens,
    };
    let schedule = if mode.eq_ignore_ascii_case("prefill") {
        AttentionSchedule::a2000_prefill()
    } else {
        AttentionSchedule::a2000_decode()
    };

    let shader = synthesize_causal_attention(shape, schedule)
        .map_err(|e| args::bad(span, format!("WgslForge.attention_synthesize failed: {e}")))?;

    Ok(args::record([
        ("op", Value::String("attention_synthesize".into())),
        (
            "id",
            Value::String(ids::WGSL_FORGE_ATTENTION_SYNTHESIZE.into()),
        ),
        ("source_hash", Value::String(shader.source_hash)),
        (
            "workgroup_size",
            Value::U64(shader.schedule.workgroup_size as u64),
        ),
        (
            "naga_validated",
            Value::Bool(shader.validation.naga_validated),
        ),
        (
            "binding_count",
            Value::U64(shader.validation.binding_count as u64),
        ),
        ("source", Value::String(shader.source)),
    ]))
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "wgsl-forge")))]
pub fn attention_synthesize(_args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    Err(args::bad(span, "WgslForge.attention_synthesize requires the enabled native WGSL Forge service; no shader, validation or execution result was produced"))
}

#[cfg(all(test, not(target_arch = "wasm32"), feature = "wgsl-forge"))]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn rec(pairs: &[(&str, Value)]) -> Value {
        let mut m = BTreeMap::new();
        for (k, v) in pairs {
            m.insert((*k).into(), v.clone());
        }
        Value::Record(m)
    }

    #[test]
    fn wgsl_forge_seam_generates_and_validates() {
        let Span { start, end } = Span { start: 0, end: 0 };
        let Value::Record(r) = list_kernels(&rec(&[]), Span { start, end }).unwrap() else {
            panic!("expected record");
        };
        assert!(matches!(r.get("kernel_count"), Some(Value::U64(n)) if *n > 0));

        let Value::Record(g) = generate(
            &rec(&[("kernel", Value::String("affine-f32".into()))]),
            Span { start, end },
        )
        .unwrap() else {
            panic!("expected record");
        };
        assert!(g.get("source").is_some());

        let Value::Record(v) = validate(
            &rec(&[("kernel", Value::String("ray-probe".into()))]),
            Span { start, end },
        )
        .unwrap() else {
            panic!("expected record");
        };
        assert_eq!(v.get("naga_validated"), Some(&Value::Bool(true)));
    }
}

#[cfg(all(test, not(all(not(target_arch = "wasm32"), feature = "wgsl-forge"))))]
mod unavailable_tests {
    use super::*;
    #[test]
    fn disabled_native_service_never_fabricates_a_kernel_or_validation() {
        let value=Value::Record(Default::default());
        let span=Span::point(0);
        let Value::Record(catalogue)=list_kernels(&value,span).unwrap() else { panic!("catalogue record") };
        assert_eq!(catalogue.get("available"),Some(&Value::Bool(false)));
        assert_eq!(catalogue.get("kernel_count"),Some(&Value::U64(0)));
        assert!(generate(&value,span).is_err());
        assert!(validate(&value,span).is_err());
        assert!(roofline(&value,span).is_err());
        assert!(schedule_candidates(&value,span).is_err());
        assert!(attention_synthesize(&value,span).is_err());
    }
}
