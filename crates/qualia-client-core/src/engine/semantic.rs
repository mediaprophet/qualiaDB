use qualia_core_db::daemon_graph::graph_read_guard;
use qualia_core_db::sparql_library::sparql_executor::QueryExecutor;
use qualia_core_db::sparql_library::sparql_parser::parse_sparql;
use qualia_core_db::sparql_library::sparql_planner::QueryPlanner;

/// Executes a SPARQL query against the local in-memory NQuin graph.
pub fn execute_local_sparql(query: &str) -> Result<Vec<(String, String, String)>, String> {
    let guard = graph_read_guard();
    let quins = guard.as_slice();

    let (sparql_query, ctx) = parse_sparql(query)?;
    let plan = QueryPlanner::plan(&sparql_query, &ctx)?;
    let executor = QueryExecutor::new(quins);
    
    let raw_rows = executor.execute(&plan, &ctx)?;
    
    // Map to simple 3-tuple for UI
    let mut mapped = Vec::with_capacity(raw_rows.len());
    for row in raw_rows {
        let s = row.slots[0].map(|v| format!("{:016X}", v)).unwrap_or_else(|| "".to_string());
        let p = row.slots[1].map(|v| format!("{:016X}", v)).unwrap_or_else(|| "".to_string());
        let o = row.slots[2].map(|v| format!("{:016X}", v)).unwrap_or_else(|| "".to_string());
        mapped.push((s, p, o));
    }
    
    Ok(mapped)
}

/// Validates a node against a SHACL shape within the local graph.
///
/// HONESTY / fail-closed: there is no shape registry yet that resolves a
/// `shape_uri` to real SHACL constraints. The former implementation built a
/// shape with `constraint_count: 0` and validated against it — an empty
/// constraint set trivially "conforms", so this returned `Ok(true)` for ANY
/// node and graph, lighting up the UI's "Graph Valid" unconditionally (a
/// false green light). Until shape resolution exists we refuse rather than
/// fabricate a conformance verdict. The real `ShaclValidator` /
/// `sparql_shacl::run` engine is genuine — the missing piece is the
/// `shape_uri → constraints` lookup that would feed it.
pub fn validate_local_shacl(_node: u64, shape_uri: u64) -> Result<bool, String> {
    Err(format!(
        "SHACL validation unavailable: no constraints resolved for shape {shape_uri:#018x} \
         (shape-registry lookup is not implemented). Refusing to report conformance against \
         an empty shape."
    ))
}

/// Executes an SLG computational VM frame locally.
pub fn execute_slg_vm(frame_data: &[u8]) -> Result<String, String> {
    use qualia_core_db::governance::webizen::{execute_vm_frame, SlgArena, VmFrame};
    
    let mut arena = SlgArena::new();
    let mut frame = VmFrame::default();
    
    // Parse the JSON array into the frame
    if let Ok(value) = serde_json::from_slice::<serde_json::Value>(frame_data) {
        if let Some(payload) = value.get("payload") {
            if let Some(arr) = payload.as_array() {
                if arr.len() >= 3 {
                    frame.subject_reg = arr[0].as_u64().unwrap_or(0);
                    frame.predicate_reg = arr[1].as_u64().unwrap_or(0);
                    frame.object_reg = arr[2].as_u64().unwrap_or(0);
                }
            }
        }
    }
    
    let bytecode = [];
    match execute_vm_frame(&mut arena, &bytecode, &mut frame) {
        Some(quin) => Ok(format!("Computed: {:016X}", quin.subject)),
        None => Ok(format!("VM Execution Completed for Subject: {}", frame.subject_reg)),
    }
}
