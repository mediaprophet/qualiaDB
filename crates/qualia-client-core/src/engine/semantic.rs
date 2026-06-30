use qualia_core_db::daemon_graph::graph_read_guard;
use qualia_core_db::sparql_library::sparql_executor::QueryExecutor;
use qualia_core_db::sparql_library::sparql_parser::parse_sparql;
use qualia_core_db::sparql_library::sparql_planner::QueryPlanner;
use qualia_core_db::sparql_library::sparql_shacl::ShaclValidator;

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
pub fn validate_local_shacl(node: u64, shape_uri: u64) -> Result<bool, String> {
    let guard = graph_read_guard();
    let quins = guard.as_slice();
    
    let mut validator = ShaclValidator::new(quins);
    
    let shape = qualia_core_db::sparql_library::sparql_shacl::ShaclShape {
        shape_iri: shape_uri,
        target_class: None,
        target_node: Some(node),
        constraints: [0; 32],
        constraint_count: 0,
    };
    
    if let Err(e) = validator.add_shape(shape) {
        return Err(e);
    }

    match validator.validate_node(node, &shape) {
        Ok(result) => Ok(result.conforms),
        Err(e) => Err(e),
    }
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
