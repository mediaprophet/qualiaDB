//! MCP tools for VibeScript 0.1 engine: compilation, checking, formatting, evaluation, and invocation.
//!
//! Exposes Vibe DSL parsing, typechecking, AST generation, bytecode compilation,
//! runtime evaluation, and capability invocation to support building software within
//! the QualiaDB ecosystem.
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_tool_args, McpSystemError};
use serde_json::{json, Value};

/// Converts `serde_json::Value` into `vibe::Value`.
pub fn json_to_vibe(v: &serde_json::Value) -> vibe::Value {
    match v {
        serde_json::Value::Null => vibe::Value::Null,
        serde_json::Value::Bool(b) => vibe::Value::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                vibe::Value::I64(i)
            } else if let Some(u) = n.as_u64() {
                vibe::Value::U64(u)
            } else if let Some(f) = n.as_f64() {
                vibe::Value::F64(f)
            } else {
                vibe::Value::F64(0.0)
            }
        }
        serde_json::Value::String(s) => vibe::Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            vibe::Value::List(arr.iter().map(json_to_vibe).collect())
        }
        serde_json::Value::Object(map) => {
            let mut record = std::collections::BTreeMap::new();
            for (k, val) in map {
                record.insert(k.clone(), json_to_vibe(val));
            }
            vibe::Value::Record(record)
        }
    }
}

/// Converts `vibe::Value` into `serde_json::Value`.
pub fn vibe_to_json(v: &vibe::Value) -> serde_json::Value {
    match v {
        vibe::Value::Null => serde_json::Value::Null,
        vibe::Value::Bool(b) => serde_json::json!(b),
        vibe::Value::I64(i) => serde_json::json!(i),
        vibe::Value::U64(u) => serde_json::json!(u),
        vibe::Value::F64(f) => serde_json::json!(f),
        vibe::Value::String(s)
        | vibe::Value::Iri(s)
        | vibe::Value::Blank(s)
        | vibe::Value::Var(s) => serde_json::json!(s),
        vibe::Value::Prefixed(p, s) => serde_json::json!(format!("{p}:{s}")),
        vibe::Value::List(list) => {
            serde_json::Value::Array(list.iter().map(vibe_to_json).collect())
        }
        vibe::Value::Record(map) => {
            let mut obj = serde_json::Map::with_capacity(map.len());
            for (k, val) in map {
                obj.insert(k.clone(), vibe_to_json(val));
            }
            serde_json::Value::Object(obj)
        }
        vibe::Value::Receipt => serde_json::json!({ "receipt": true }),
        vibe::Value::Ok(inner) => serde_json::json!({ "ok": vibe_to_json(inner) }),
        vibe::Value::Err(inner) => serde_json::json!({ "err": vibe_to_json(inner) }),
        vibe::Value::Triple(s, p, o) => serde_json::json!({
            "s": vibe_to_json(s),
            "p": vibe_to_json(p),
            "o": vibe_to_json(o),
        }),
        vibe::Value::Reified { s, p, o, r } => serde_json::json!({
            "s": vibe_to_json(s),
            "p": vibe_to_json(p),
            "o": vibe_to_json(o),
            "r": vibe_to_json(r),
        }),
        vibe::Value::Instant(i) => serde_json::json!({
            "secs": i.secs,
            "nanos": i.nanos,
        }),
        vibe::Value::Duration(d) => serde_json::json!({
            "secs": d.secs,
            "nanos": d.nanos,
        }),
        vibe::Value::Quantity(q) => serde_json::json!({
            "value": q.value,
            "unit": q.unit,
        }),
        vibe::Value::QuinRef(q) => serde_json::json!({
            "quin_ref": q.content_hash(),
        }),
        vibe::Value::Enum(e) => serde_json::json!({
            "enum": e.enum_name,
            "variant": e.variant_name,
            "payload": e.payload.iter().map(vibe_to_json).collect::<Vec<_>>(),
        }),
        other => serde_json::json!(format!("{other:?}")),
    }
}

/// `vibe_check` tool — parses and typechecks VibeScript source code.
/// Returns syntax/type diagnostics, evidential bindings, and validation verdict.
pub fn vibe_check(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let source = v
        .get("source")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;

    let report = vibe::diagnose(source);
    Ok(report.to_json())
}

/// `vibe_compile` tool — compiles VibeScript source into stack-based bytecode `Chunk`.
/// Returns disassembly, constants, metadata, and hex-encoded bytecode binary.
pub fn vibe_compile(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let source = v
        .get("source")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;

    let program = vibe::parse_program(source).map_err(|e| {
        log::error!("Vibe parse failed: {:?}", e);
        McpSystemError::InvalidParameters
    })?;

    vibe::check_program(&program).map_err(|e| {
        log::error!("Vibe typecheck failed: {:?}", e);
        McpSystemError::InvalidParameters
    })?;

    let chunk = vibe::bytecode::compile(&program).map_err(|e| {
        log::error!("Vibe bytecode compilation failed: {:?}", e);
        McpSystemError::InvalidParameters
    })?;

    let encoded = vibe::bytecode::encode_chunk(&chunk);
    let hex_bytes: String = encoded.iter().map(|b| format!("{b:02x}")).collect();

    let mut instructions = Vec::new();
    for (idx, op) in chunk.code.iter().enumerate() {
        instructions.push(format!("{idx:04}: {:?}", op));
    }

    Ok(json!({
        "status": "success",
        "instruction_count": chunk.code.len(),
        "constants_count": chunk.constants.len(),
        "functions_count": chunk.functions.len(),
        "bytecode_hex": hex_bytes,
        "disassembly": instructions,
    })
    .to_string())
}

/// `vibe_format` tool — canonical formatting of VibeScript declarations.
pub fn vibe_format(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let source = v
        .get("source")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;

    let program = vibe::parse_program(source).map_err(|_| McpSystemError::InvalidParameters)?;
    let formatted = vibe::project_program(&program, &vibe::ProjectOptions::default());

    Ok(json!({
        "status": "success",
        "formatted": formatted
    })
    .to_string())
}

/// `vibe_invoke` tool — invokes a capability by ID across logic, math, physics, audio, and studio domains.
pub fn vibe_invoke(raw_args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let id = v
        .get("id")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;

    let empty_obj = json!({});
    let args_val = v.get("args").unwrap_or(&empty_obj);
    let vibe_args = json_to_vibe(args_val);

    let mut snap = qualia_core_db::poet_host::PoetSnapshot::live();
    let seam = qualia_core_db::poet_host::invoke::ids::seam_for(id);

    match snap.invoke_id(id, vibe_args) {
        Ok(val) => Ok(json!({
            "status": "ok",
            "id": id,
            "seam": seam,
            "result": vibe_to_json(&val),
        })
        .to_string()),
        Err(diag) => Ok(json!({
            "status": "error",
            "id": id,
            "seam": seam,
            "error": format!("{diag}"),
        })
        .to_string()),
    }
}

/// `vibe_eval` tool — parses and evaluates VibeScript source code against the live snapshot.
pub fn vibe_eval(raw_args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let source = v
        .get("source")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?;

    let program = vibe::parse_program(source).map_err(|e| {
        log::error!("Vibe parse failed: {:?}", e);
        McpSystemError::InvalidParameters
    })?;

    vibe::check_program(&program).map_err(|e| {
        log::error!("Vibe typecheck failed: {:?}", e);
        McpSystemError::InvalidParameters
    })?;

    let mut snap = qualia_core_db::poet_host::PoetSnapshot::live();
    let mut env = vibe::Env::default();
    let mut engine = vibe::Engine::with_program(&mut snap, vibe::Budget::default(), &program);
    let val = engine.eval_program(&program, &mut env).map_err(|e| {
        log::error!("Vibe eval failed: {:?}", e);
        McpSystemError::ExecutionFailed(format!("{e}"))
    })?;

    let quin_count = snap.visible_count();
    let mut bindings = serde_json::Map::new();
    for (k, var_val) in &env.vars {
        bindings.insert(k.clone(), vibe_to_json(var_val));
    }

    Ok(json!({
        "status": "ok",
        "result": vibe_to_json(&val),
        "bindings": serde_json::Value::Object(bindings),
        "quin_count": quin_count,
    })
    .to_string())
}

/// `vibe_catalog` tool — queries the catalog of all bound VibeScript capability IDs.
pub fn vibe_catalog(raw_args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let prefix = v.get("prefix").and_then(Value::as_str);
    let seam = v.get("seam").and_then(Value::as_str);
    let limit = v.get("limit").and_then(Value::as_u64).unwrap_or(200) as usize;

    let all_bound = qualia_core_db::poet_host::invoke::ids::ALL_BOUND;
    let total_bound = all_bound.len();

    let mut matches = Vec::new();
    for &id in all_bound {
        let id_seam = qualia_core_db::poet_host::invoke::ids::seam_for(id);
        if let Some(p) = prefix {
            if !id.starts_with(p) && !id.contains(p) {
                continue;
            }
        }
        if let Some(s) = seam {
            if !id_seam.eq_ignore_ascii_case(s) {
                continue;
            }
        }
        matches.push(json!({
            "id": id,
            "seam": id_seam,
        }));
        if matches.len() >= limit {
            break;
        }
    }

    Ok(json!({
        "status": "ok",
        "total_bound": total_bound,
        "matches": matches,
    })
    .to_string())
}
