//! MCP tool for WGSL Forge: deterministic WGSL generation, validation, certification, and tuning.
//!
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_tool_args, McpSystemError};
use qualia_core_db::poet_host::PoetSnapshot;
use qualia_core_db::wgsl_forge::{
    generate_builtin, roofline_for, synthesize_causal_attention, validate_wgsl,
    AdapterConstraints, AttentionSchedule, AttentionShape, BuiltinKernel, Schedule,
    ScheduleSpace, TargetBackend,
};
use serde_json::{json, Value};
use std::str::FromStr;

/// `wgsl_forge` tool — deterministic WGSL kernel generation, validation, roofline analysis, and attention synthesis.
pub fn wgsl_forge(raw_args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let op = v
        .get("op")
        .and_then(Value::as_str)
        .unwrap_or("list_kernels");

    // Attempt dispatch via PoetSnapshot invoke host first
    let canonical_id = if op.starts_with("WgslForge.") {
        op.to_string()
    } else {
        format!("WgslForge.{op}")
    };
    let vibe_args = super::vibe::json_to_vibe(&v);
    let mut snap = PoetSnapshot::live();

    if let Ok(res) = snap.invoke_id(&canonical_id, vibe_args) {
        return Ok(json!({
            "status": "ok",
            "op": op,
            "id": canonical_id,
            "result": super::vibe::vibe_to_json(&res),
        })
        .to_string());
    }

    // Direct WGSL Forge dispatch fallback
    match op {
        "list_kernels" => {
            let filter = v.get("filter").and_then(Value::as_str).unwrap_or("");
            let mut kernels = Vec::new();
            for builtin in BuiltinKernel::ALL {
                let name = builtin.name();
                if !filter.is_empty() && !name.contains(filter) {
                    continue;
                }
                let spec = builtin.spec();
                kernels.push(json!({
                    "name": name,
                    "version": spec.semantic_version,
                    "description": spec.description,
                    "has_gpu_oracle": builtin.has_gpu_oracle(),
                }));
            }

            let viewport_pipelines = vec![
                "cyber_glass",
                "barnes_hut_graph_physics",
                "wire_particles",
                "manifold_grid_10d",
                "aurora_curtains",
                "terrain_pbr_pom",
                "ocean_gerstner",
                "black_hole_lensing",
            ];

            Ok(json!({
                "status": "ok",
                "op": "list_kernels",
                "kernel_count": kernels.len(),
                "kernels": kernels,
                "viewport_pipelines": viewport_pipelines,
            })
            .to_string())
        }

        "generate" => {
            let kernel_name = v.get("kernel").and_then(Value::as_str).unwrap_or("affine-f32");
            let target_str = v.get("target").and_then(Value::as_str).unwrap_or("wgsl");
            let workgroup_size = v
                .get("workgroup_size")
                .and_then(Value::as_u64)
                .unwrap_or(64) as u32;
            let items_per_invocation = v
                .get("items_per_invocation")
                .and_then(Value::as_u64)
                .unwrap_or(1) as u32;
            let vector_width = v
                .get("vector_width")
                .and_then(Value::as_u64)
                .unwrap_or(1) as u32;

            let builtin = BuiltinKernel::from_str(kernel_name).map_err(|e| {
                McpSystemError::ExecutionFailed(format!("WgslForge.generate unknown kernel: {e}"))
            })?;
            let target = TargetBackend::from_str(target_str).map_err(|e| {
                McpSystemError::ExecutionFailed(format!("WgslForge.generate invalid target: {e}"))
            })?;
            let schedule = Schedule {
                workgroup_size,
                items_per_invocation,
                vector_width,
                ..Default::default()
            };

            let generated = generate_builtin(builtin, schedule, target).map_err(|e| {
                McpSystemError::ExecutionFailed(format!("WgslForge.generate failed: {e}"))
            })?;

            Ok(json!({
                "status": "ok",
                "op": "generate",
                "kernel_id": generated.kernel_id,
                "target": target_str,
                "entry_point": "main",
                "semantic_hash": generated.semantic_hash,
                "source_hash": generated.source_hash,
                "workgroup_size": workgroup_size,
                "source": generated.source,
            })
            .to_string())
        }

        "validate" => {
            let source_to_validate = if let Some(src) = v.get("source").and_then(Value::as_str) {
                src.to_string()
            } else {
                let kernel_name = v.get("kernel").and_then(Value::as_str).unwrap_or("affine-f32");
                let workgroup_size = v
                    .get("workgroup_size")
                    .and_then(Value::as_u64)
                    .unwrap_or(64) as u32;
                let items_per_invocation = v
                    .get("items_per_invocation")
                    .and_then(Value::as_u64)
                    .unwrap_or(1) as u32;
                let vector_width = v
                    .get("vector_width")
                    .and_then(Value::as_u64)
                    .unwrap_or(1) as u32;
                let builtin = BuiltinKernel::from_str(kernel_name).map_err(|e| {
                    McpSystemError::ExecutionFailed(format!("WgslForge.validate unknown kernel: {e}"))
                })?;
                let schedule = Schedule {
                    workgroup_size,
                    items_per_invocation,
                    vector_width,
                    ..Default::default()
                };
                generate_builtin(builtin, schedule, TargetBackend::Wgsl)
                    .map_err(|e| {
                        McpSystemError::ExecutionFailed(format!("WgslForge.validate generation failed: {e}"))
                    })?
                    .source
            };

            let report = validate_wgsl(&source_to_validate).map_err(|e| {
                McpSystemError::ExecutionFailed(format!("WgslForge.validate Naga error: {e}"))
            })?;

            Ok(json!({
                "status": "ok",
                "op": "validate",
                "naga_validated": report.naga_validated,
                "binding_count": report.binding_count,
                "entry_points": report.entry_points,
            })
            .to_string())
        }

        "roofline" => {
            let kernel_name = v.get("kernel").and_then(Value::as_str).unwrap_or("affine-f32");
            let n = v.get("n").and_then(Value::as_u64).unwrap_or(65_536).max(1);
            let builtin = BuiltinKernel::from_str(kernel_name).map_err(|e| {
                McpSystemError::ExecutionFailed(format!("WgslForge.roofline unknown kernel: {e}"))
            })?;
            let est = roofline_for(builtin, n);

            Ok(json!({
                "status": "ok",
                "op": "roofline",
                "kernel": builtin.name(),
                "n": n,
                "flops": est.flops,
                "bytes": est.bytes,
                "arithmetic_intensity": est.arithmetic_intensity,
                "bound": format!("{:?}", est.bound),
            })
            .to_string())
        }

        "schedule_candidates" => {
            let kernel_name = v.get("kernel").and_then(Value::as_str).unwrap_or("affine-f32");
            let builtin = BuiltinKernel::from_str(kernel_name).map_err(|e| {
                McpSystemError::ExecutionFailed(format!("WgslForge.schedule_candidates unknown kernel: {e}"))
            })?;
            let spec = builtin.spec();
            let constraints = AdapterConstraints::portable();
            let space = ScheduleSpace::default();
            let candidates = space.candidates(&spec, &constraints);

            let list: Vec<Value> = candidates
                .iter()
                .map(|c| {
                    json!({
                        "workgroup_size": c.workgroup_size,
                        "items_per_invocation": c.items_per_invocation,
                        "vector_width": c.vector_width,
                    })
                })
                .collect();

            Ok(json!({
                "status": "ok",
                "op": "schedule_candidates",
                "kernel": builtin.name(),
                "candidate_count": list.len(),
                "candidates": list,
            })
            .to_string())
        }

        "attention_synthesize" => {
            let head_dim = v.get("head_dim").and_then(Value::as_u64).unwrap_or(64) as u32;
            let head_count = v.get("head_count").and_then(Value::as_u64).unwrap_or(16) as u32;
            let context_tokens = v
                .get("context_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(512) as u32;
            let batch_tokens = v.get("batch_tokens").and_then(Value::as_u64).unwrap_or(1) as u32;
            let mode = v.get("mode").and_then(Value::as_str).unwrap_or("decode");

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

            let shader = synthesize_causal_attention(shape, schedule).map_err(|e| {
                McpSystemError::ExecutionFailed(format!("WgslForge.attention_synthesize failed: {e}"))
            })?;

            Ok(json!({
                "status": "ok",
                "op": "attention_synthesize",
                "source_hash": shader.source_hash,
                "workgroup_size": shader.schedule.workgroup_size,
                "naga_validated": shader.validation.naga_validated,
                "binding_count": shader.validation.binding_count,
                "source": shader.source,
            })
            .to_string())
        }

        other => Err(McpSystemError::ExecutionFailed(format!(
            "Unsupported wgsl_forge operation: {other}"
        ))),
    }
}
