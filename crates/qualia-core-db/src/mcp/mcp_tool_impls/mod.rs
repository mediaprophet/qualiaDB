//! Full-parameter MCP tool implementations (cold path -- serde JSON allowed).
//!
//! Library-ized per CLAUDE.md §11: the per-domain tool entrypoints live in
//! focused submodules and are re-exported here so every tool remains reachable
//! at `mcp::mcp_server::mcp_tool_impls::*`. This module keeps the shared imports
//! and helper functions.

use super::McpSystemError;
use crate::NQuin;
use serde_json::{json, Value};

mod algebra;
mod chemistry;
mod engineering;
mod geometry;
mod governance;
mod logic;
mod medical;
mod ml_finance;
mod physics;
mod stats;
mod vision;

pub use algebra::*;
pub use chemistry::*;
pub use engineering::*;
pub use geometry::*;
pub use governance::*;
pub use logic::*;
pub use medical::*;
pub use ml_finance::*;
pub use physics::*;
pub use stats::*;
pub use vision::*;

#[cfg(test)]
mod tests;

pub fn parse_tool_args(args: &[u8]) -> Result<Value, McpSystemError> {
    if args.is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_slice(args).map_err(|_| McpSystemError::ParseError)
}

fn json_str<'a>(v: &'a Value, key: &str, default: &'a str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or(default)
}

fn json_f64(v: &Value, key: &str, default: f64) -> f64 {
    v.get(key)
        .and_then(|x| x.as_f64().or_else(|| x.as_u64().map(|n| n as f64)))
        .unwrap_or(default)
}

fn json_u64(v: &Value, key: &str, default: u64) -> u64 {
    v.get(key)
        .and_then(|x| x.as_u64().or_else(|| x.as_i64().map(|n| n as u64)))
        .unwrap_or(default)
}

fn json_bool(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(Value::as_bool).unwrap_or(default)
}

fn json_f64_array(v: &Value, key: &str) -> Result<Vec<f64>, McpSystemError> {
    let arr = v
        .get(key)
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    arr.iter()
        .map(|x| x.as_f64().ok_or(McpSystemError::InvalidParameters))
        .collect()
}

fn json_u8_array(v: &Value, key: &str) -> Result<Vec<u8>, McpSystemError> {
    let arr = v
        .get(key)
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    arr.iter()
        .map(|x| {
            x.as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or(McpSystemError::InvalidParameters)
        })
        .collect()
}

fn parse_quin(v: &Value) -> Result<NQuin, McpSystemError> {
    Ok(NQuin {
        subject: json_u64(v, "subject", 0),
        predicate: json_u64(v, "predicate", 0),
        object: json_u64(v, "object", 0),
        context: json_u64(v, "context", 0),
        metadata: json_u64(v, "metadata", 0),
        parity: json_u64(v, "parity", 0),
    })
}

pub fn parse_quin_slice(v: &Value, key: &str) -> Result<Vec<NQuin>, McpSystemError> {
    let arr = v
        .get(key)
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    arr.iter().map(parse_quin).collect()
}

fn ensure_parity(q: &mut NQuin) {
    if q.parity == 0 {
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
    }
}

fn parse_matrix_def(v: &Value) -> Result<(String, usize, usize, Vec<f64>), McpSystemError> {
    let id = v
        .get("id")
        .and_then(Value::as_str)
        .ok_or(McpSystemError::InvalidParameters)?
        .to_string();
    let rows = v
        .get("rows")
        .and_then(Value::as_u64)
        .ok_or(McpSystemError::InvalidParameters)? as usize;
    let cols = v
        .get("cols")
        .and_then(Value::as_u64)
        .ok_or(McpSystemError::InvalidParameters)? as usize;
    let data = json_f64_array(v, "data")?;
    if data.len() != rows * cols {
        return Err(McpSystemError::InvalidParameters);
    }
    Ok((id, rows, cols, data))
}

/// Pure decision for [`cooperation_gate`] (env-free, so it is unit-testable). `None` = not
/// enforcing (pass); `Some(verdict)` = the gate's verdict when enforcing. A call with no/false
/// `verified` is DeniedUnverified; with `verified` but `grounded:false` is DeniedUngrounded.
fn gate_verdict(
    args: &[u8],
    enforcing: bool,
) -> Option<crate::mcp_cooperation::CooperationVerdict> {
    use crate::mcp_cooperation::{authorize, CallerStandpoint};
    use crate::modalities::interaction_governance::Governance;
    use crate::modalities::logic::deontic::DeonticStatus;
    if !enforcing {
        return None;
    }
    let v = parse_tool_args(args).unwrap_or(Value::Null);
    let bool_of = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
    let standpoint = CallerStandpoint {
        agent: crate::q_hash(json_str(&v, "caller", "")),
        role: crate::q_hash(json_str(&v, "role", "role:caller")),
        verified: bool_of("verified"),
    };
    // When enforcing, grounding must be positively asserted (strict default).
    let grounded = bool_of("grounded");
    Some(authorize(
        &standpoint,
        grounded,
        DeonticStatus::Active,
        Governance::default(),
    ))
}

pub(crate) fn hex_decode(s: &str) -> Result<Vec<u8>, McpSystemError> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(McpSystemError::InvalidParameters);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| McpSystemError::InvalidParameters)
        })
        .collect()
}
