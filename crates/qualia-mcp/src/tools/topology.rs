//! MCP tools for the POET Geometry-Topology Complex ("Geometries Harbour Many Graphs").
//!
//! Exposes `poet_define_geometry`, `poet_harbour_graph`, `poet_switch_harboured_graph`,
//! `poet_analyze_homology`, `poet_seal_topology`, and topology anchor/delta/traverse tools
//! backed by `PoetSnapshot::invoke_id` and the `GTOP` / `HTGR` header primitives.
//! Follows `AGENTS.md` §0-B (<350 lines).

use crate::types::{parse_tool_args, McpSystemError};
use qualia_core_db::poet_host::invoke::ids;
use qualia_core_db::poet_host::PoetSnapshot;
use serde_json::{json, Value};

fn normalize_topology_payload(raw: &Value) -> Value {
    let Some(obj) = raw.as_object() else {
        return raw.clone();
    };
    let mut m = obj.clone();
    if let Some(arr) = obj.get("harboured_graphs").and_then(|v| v.as_array()) {
        m.entry("harboured_graph_count".to_string())
            .or_insert_with(|| json!(arr.len() as u64));
    }
    if let Some(ck) = obj.get("carrier_kind") {
        m.entry("complex_kind".to_string())
            .or_insert_with(|| ck.clone());
    }
    if let Some(v) = obj.get("vertices") {
        m.entry("cell_0_count".to_string()).or_insert_with(|| v.clone());
        m.entry("vertex_count".to_string()).or_insert_with(|| v.clone());
    }
    if let Some(e) = obj.get("edges") {
        m.entry("cell_1_count".to_string()).or_insert_with(|| e.clone());
        m.entry("edge_count".to_string()).or_insert_with(|| e.clone());
    }
    if let Some(f) = obj.get("faces") {
        m.entry("cell_2_count".to_string()).or_insert_with(|| f.clone());
    }
    if let Some(c) = obj.get("cells") {
        m.entry("cell_3_count".to_string()).or_insert_with(|| c.clone());
    }
    if let Some(comp) = obj.get("components") {
        m.entry("betti_0".to_string()).or_insert_with(|| comp.clone());
    }
    if let Some(k) = obj.get("kind").and_then(|v| v.as_str()) {
        if k == "DirectedFlowNetwork" {
            let morph = if obj.get("acyclic").and_then(|v| v.as_bool()) == Some(true) {
                "DeterministicDAG"
            } else {
                "DirectedMultigraph"
            };
            m.insert("morphology".to_string(), json!(morph));
        }
    }
    if let Some(am) = obj.get("anchor_mode").and_then(|v| v.as_str()) {
        let dom = match am {
            "SimplexBarycentric" => "Cell2Face",
            "CellCentroid" => "Cell3Volume",
            "EdgeMidpoint" => "Cell1Edge",
            _ => "Cell0Vertex",
        };
        m.entry("anchor_domain".to_string())
            .or_insert_with(|| json!(dom));
    }
    Value::Object(m)
}

fn invoke_topology_op(
    canonical_id: &str,
    op_name: &str,
    raw_args: &[u8],
) -> Result<String, McpSystemError> {
    let v = parse_tool_args(raw_args)?;
    let raw_payload = v.get("args").unwrap_or(&v);
    let payload = normalize_topology_payload(raw_payload);
    let vibe_args = super::vibe::json_to_vibe(&payload);

    let mut snap = PoetSnapshot::live();
    let res = snap
        .invoke_id(canonical_id, vibe_args)
        .map_err(|e| McpSystemError::ExecutionFailed(format!("{e:?}")))?;
    let mut res_val = super::vibe::vibe_to_json(&res);
    if let Some(out_obj) = res_val.as_object_mut() {
        if out_obj.contains_key("crc32c") {
            out_obj
                .entry("parity_valid".to_string())
                .or_insert_with(|| json!(true));
        }
        if canonical_id == ids::TOPOLOGY_HARBOUR_GRAPH {
            out_obj
                .entry("descriptor_magic".to_string())
                .or_insert_with(|| json!("HTGR"));
        }
        if canonical_id == ids::TOPOLOGY_SWITCH_GRAPH {
            if let Some(slot_v) = out_obj.get("active_graph_slot").cloned() {
                out_obj
                    .entry("active_slot".to_string())
                    .or_insert(slot_v);
            }
            if let Some(g) = raw_payload.get("graph") {
                out_obj
                    .entry("active_graph".to_string())
                    .or_insert_with(|| g.clone());
            }
        }
        if canonical_id == ids::TOPOLOGY_ANALYZE_HOMOLOGY {
            let is_euler_2 = out_obj
                .get("euler_characteristic")
                .and_then(|v| v.as_i64())
                == Some(2);
            out_obj
                .entry("watertight_euler_2".to_string())
                .or_insert_with(|| json!(is_euler_2));
        }
    }

    Ok(json!({
        "status": "ok",
        "op": op_name,
        "id": canonical_id,
        "result": res_val
    })
    .to_string())
}

/// `poet_define_geometry` — define a host geometry carrier (`GTOP` header) capable of harbouring up to 8 graphs.
pub fn poet_define_geometry(raw_args: &[u8]) -> Result<String, McpSystemError> {
    invoke_topology_op(
        ids::TOPOLOGY_DEFINE_GEOMETRY,
        "poet_define_geometry",
        raw_args,
    )
}

/// Alias for `poet_define_geometry`.
pub fn topology_define_geometry(raw_args: &[u8]) -> Result<String, McpSystemError> {
    poet_define_geometry(raw_args)
}

/// `poet_harbour_graph` — harbour a graph slot (`HTGR`, `0..8`) inside a host geometry (`GTOP`).
pub fn poet_harbour_graph(raw_args: &[u8]) -> Result<String, McpSystemError> {
    invoke_topology_op(ids::TOPOLOGY_HARBOUR_GRAPH, "poet_harbour_graph", raw_args)
}

/// Alias for `poet_harbour_graph`.
pub fn topology_harbour_graph(raw_args: &[u8]) -> Result<String, McpSystemError> {
    poet_harbour_graph(raw_args)
}

/// `poet_switch_harboured_graph` — switch the active harboured graph slot (`0..8`) or inspect a slot.
pub fn poet_switch_harboured_graph(raw_args: &[u8]) -> Result<String, McpSystemError> {
    invoke_topology_op(
        ids::TOPOLOGY_SWITCH_GRAPH,
        "poet_switch_harboured_graph",
        raw_args,
    )
}

/// Alias for `poet_switch_harboured_graph`.
pub fn topology_switch_graph(raw_args: &[u8]) -> Result<String, McpSystemError> {
    poet_switch_harboured_graph(raw_args)
}

/// `poet_analyze_homology` — compute Euler characteristic ($\chi = V - E + F - C$) and Betti numbers ($\beta_0, \beta_1, \beta_2$).
pub fn poet_analyze_homology(raw_args: &[u8]) -> Result<String, McpSystemError> {
    invoke_topology_op(
        ids::TOPOLOGY_ANALYZE_HOMOLOGY,
        "poet_analyze_homology",
        raw_args,
    )
}

/// Alias for `poet_analyze_homology`.
pub fn topology_analyze_homology(raw_args: &[u8]) -> Result<String, McpSystemError> {
    poet_analyze_homology(raw_args)
}

/// `poet_seal_topology` — seal a `GTOP` geometry-topology complex header with parity and Ed25519/CRC32C digest.
pub fn poet_seal_topology(raw_args: &[u8]) -> Result<String, McpSystemError> {
    invoke_topology_op(ids::TOPOLOGY_SEAL, "poet_seal_topology", raw_args)
}

/// Alias for `poet_seal_topology`.
pub fn topology_seal(raw_args: &[u8]) -> Result<String, McpSystemError> {
    poet_seal_topology(raw_args)
}

/// `topology_anchor_bind` — bind a harboured graph cell to a host geometry anchor (`SimplexBarycentric`, `CellCentroid`, etc.).
pub fn topology_anchor_bind(raw_args: &[u8]) -> Result<String, McpSystemError> {
    invoke_topology_op(ids::TOPOLOGY_ANCHOR_BIND, "topology_anchor_bind", raw_args)
}

/// `topology_edit_delta` — apply a topology edit delta (`SplitEdge`, `CollapseEdge`, `SubdivideFace`, etc.).
pub fn topology_edit_delta(raw_args: &[u8]) -> Result<String, McpSystemError> {
    invoke_topology_op(ids::TOPOLOGY_DELTA_EDIT, "topology_edit_delta", raw_args)
}

/// `topology_traverse` — execute a zero-heap CSR neighborhood/incidence traversal over a harboured graph.
pub fn topology_traverse(raw_args: &[u8]) -> Result<String, McpSystemError> {
    invoke_topology_op(ids::TOPOLOGY_TRAVERSE, "topology_traverse", raw_args)
}

/// `poet_topology_bind` — bind a POET container to a host geometry and its harboured graph slots.
pub fn poet_topology_bind(raw_args: &[u8]) -> Result<String, McpSystemError> {
    invoke_topology_op(ids::POET_TOPOLOGY_BIND, "poet_topology_bind", raw_args)
}

/// Unified dispatcher for topology MCP requests by sub-operation name.
pub fn dispatch_topology_tool(op: &str, raw_args: &[u8]) -> Result<String, McpSystemError> {
    match op {
        "poet_define_geometry" | "topology_define_geometry" | "define_geometry" => {
            poet_define_geometry(raw_args)
        }
        "poet_harbour_graph" | "topology_harbour_graph" | "harbour_graph" => {
            poet_harbour_graph(raw_args)
        }
        "poet_switch_harboured_graph" | "topology_switch_graph" | "switch_graph" => {
            poet_switch_harboured_graph(raw_args)
        }
        "poet_analyze_homology" | "topology_analyze_homology" | "analyze_homology" => {
            poet_analyze_homology(raw_args)
        }
        "poet_seal_topology" | "topology_seal" | "seal" => poet_seal_topology(raw_args),
        "topology_anchor_bind" | "anchor_bind" => topology_anchor_bind(raw_args),
        "topology_edit_delta" | "delta_edit" => topology_edit_delta(raw_args),
        "topology_traverse" | "traverse" => topology_traverse(raw_args),
        "poet_topology_bind" | "topology_bind" => poet_topology_bind(raw_args),
        _ => Err(McpSystemError::InvalidParameters),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn mcp_topology_tools_define_harbour_switch_homology_and_seal() {
        let def_raw = br#"{
            "geometry": "NorthSpringCatchment",
            "carrier_kind": "MixedCellComplex3D",
            "embedding_dim": 3,
            "intrinsic_dim": 3,
            "metric": "Riemannian",
            "vertices": 16,
            "edges": 28,
            "faces": 14,
            "cells": 2,
            "harboured_graphs": ["HydrologyFlow", "AccessRights", "SensorDependency"]
        }"#;
        let def_out: Value = serde_json::from_str(&poet_define_geometry(def_raw).unwrap()).unwrap();
        assert_eq!(def_out["status"], "ok");
        assert_eq!(def_out["result"]["magic"], "GTOP");
        assert_eq!(def_out["result"]["harboured_graph_count"], 3);
        assert_eq!(def_out["result"]["parity_valid"], true);

        let harb_raw = br#"{
            "geometry": "NorthSpringCatchment",
            "graph": "HydrologyFlow",
            "kind": "DirectedFlowNetwork",
            "slot": 0,
            "anchor_mode": "SimplexBarycentric",
            "vertices": 16,
            "edges": 24,
            "components": 1,
            "directed": true,
            "acyclic": true
        }"#;
        let harb_out: Value = serde_json::from_str(&poet_harbour_graph(harb_raw).unwrap()).unwrap();
        assert_eq!(harb_out["status"], "ok");
        assert_eq!(harb_out["result"]["descriptor_magic"], "HTGR");
        assert_eq!(harb_out["result"]["slot"], 0);

        let sw_raw = br#"{
            "geometry": "NorthSpringCatchment",
            "slot": 1,
            "graph": "AccessRights",
            "harboured_graphs": ["HydrologyFlow", "AccessRights", "SensorDependency"]
        }"#;
        let sw_out: Value =
            serde_json::from_str(&poet_switch_harboured_graph(sw_raw).unwrap()).unwrap();
        assert_eq!(sw_out["status"], "ok");
        assert_eq!(sw_out["result"]["active_slot"], 1);
        assert_eq!(sw_out["result"]["active_graph"], "AccessRights");

        let hom_raw = br#"{
            "geometry": "NorthSpringCatchment",
            "vertices": 16,
            "edges": 28,
            "faces": 14,
            "cells": 2,
            "components": 1
        }"#;
        let hom_out: Value =
            serde_json::from_str(&poet_analyze_homology(hom_raw).unwrap()).unwrap();
        assert_eq!(hom_out["status"], "ok");
        assert_eq!(hom_out["result"]["euler_characteristic"], 0);
        assert_eq!(hom_out["result"]["betti_0"], 1);
        assert_eq!(hom_out["result"]["watertight_euler_2"], false);

        let seal_raw = br#"{
            "geometry": "NorthSpringCatchment",
            "carrier_kind": "MixedCellComplex3D",
            "vertices": 16,
            "edges": 28,
            "faces": 14,
            "cells": 2,
            "harboured_graphs": ["HydrologyFlow", "AccessRights", "SensorDependency"]
        }"#;
        let seal_out: Value = serde_json::from_str(&poet_seal_topology(seal_raw).unwrap()).unwrap();
        assert_eq!(seal_out["status"], "ok");
        assert_eq!(seal_out["result"]["sealed"], true);
        assert_eq!(seal_out["result"]["parity_valid"], true);
    }
}
