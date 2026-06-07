//! Zero-allocation scoped query execution boundary for Flutter qapps.
//!
//! Hot-path functions use stack/fixed buffers only. Heap allocation is confined
//! to the FRB wrapper that copies geometry into a Dart `Float32List`.

use qualia_core_db::{
    daemon_graph, mini_parser, q_hash, wal::WriteAheadLog, webizen_bytecode, QualiaQuin,
};

use crate::qapp_manifest::{get_compiled_capability, CompiledCapability};

/// Maximum Quin results per scoped query (42MB Sentinel budget).
pub const MAX_QUERY_RESULTS: usize = 2048;
/// Float32 values emitted per matched Quin (six u64 lanes, no inline float tagging).
pub const FLOATS_PER_QUIN: usize = 6;
pub const SCOPED_QUERY_MAX_FLOATS: usize = MAX_QUERY_RESULTS * FLOATS_PER_QUIN;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionError {
    UnregisteredApp,
    SanctuaryOverrideRequired,
    OutputBufferFull,
    InvalidBytecode,
    ClearanceViolation,
}

impl ExecutionError {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnregisteredApp => "unregistered_app",
            Self::SanctuaryOverrideRequired => "sanctuary_override_required",
            Self::OutputBufferFull => "output_buffer_full",
            Self::InvalidBytecode => "invalid_bytecode",
            Self::ClearanceViolation => "clearance_violation",
        }
    }
}

/// Walk compiled bytecode and ensure bound operands stay inside declared ontology domains.
pub fn verify_execution_scope(
    caps: &CompiledCapability,
    query_bytecode: &[u8],
) -> Result<(), ExecutionError> {
    let mut i = 0usize;
    while i < query_bytecode.len() {
        let opcode = query_bytecode[i];
        match opcode {
            mini_parser::OP_MATCH_SUBJECT
            | mini_parser::OP_MATCH_PREDICATE
            | mini_parser::OP_MATCH_OBJECT => {
                if i + 9 > query_bytecode.len() {
                    return Err(ExecutionError::InvalidBytecode);
                }
                let mut bytes = [0u8; 8];
                bytes.copy_from_slice(&query_bytecode[i + 1..i + 9]);
                let operand = u64::from_le_bytes(bytes);
                if operand != 0 && !domain_permitted(caps, operand) {
                    return Err(ExecutionError::SanctuaryOverrideRequired);
                }
                i += 9;
            }
            mini_parser::OP_HALT_IF_FALSE => i += 1,
            mini_parser::OP_END => break,
            _ => return Err(ExecutionError::InvalidBytecode),
        }
    }
    Ok(())
}

#[inline]
fn domain_permitted(caps: &CompiledCapability, hash: u64) -> bool {
    if hash == caps.app_id_hash {
        return true;
    }
    let count = caps.domain_count as usize;
    for slot in caps.permitted_domains.iter().take(count) {
        if *slot == hash {
            return true;
        }
    }
    false
}

fn wal_path() -> Option<String> {
    let state = crate::state::APP_STATE.get()?;
    let storage = state.config.lock().ok()?.storage_path.clone();
    Some(format!("{storage}/qualia_global.wal"))
}

/// Append a conduct-violation Quin when a qapp breaches its compiled capability scope.
pub fn append_sanctuary_violation(app_id_hash: u64, reason: &'static str) {
    let mut quin = QualiaQuin::new_conduct_violation(reason.as_bytes());
    quin.subject = app_id_hash;
    quin.predicate = q_hash("q42:conductViolation");
    quin.context = q_hash("q42:qappSanctuaryOverride");
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;

    if let Some(path) = wal_path() {
        if let Ok(mut wal) = WriteAheadLog::open(path) {
            let _ = wal.append_mutation(&quin);
        }
    }
}

/// Map matched Quins into a flat `f32` lane for Dart `Float32List` rendering.
/// Uses `f32::from_bits` on the low 32 bits — no scalar tagging in Quin objects.
pub fn translate_quin_to_geometry(quin: &QualiaQuin, out: &mut [f32], base: usize) -> usize {
    if base + FLOATS_PER_QUIN > out.len() {
        return 0;
    }
    out[base] = f32::from_bits(quin.subject as u32);
    out[base + 1] = f32::from_bits(quin.predicate as u32);
    out[base + 2] = f32::from_bits(quin.object as u32);
    out[base + 3] = f32::from_bits(quin.context as u32);
    out[base + 4] = f32::from_bits(quin.metadata as u32);
    out[base + 5] = f32::from_bits(quin.parity as u32);
    FLOATS_PER_QUIN
}

#[inline]
fn result_passes_clearance(quin: &QualiaQuin, clearance: u8) -> bool {
    quin.get_sensitivity_byte() <= clearance
}

/// Hot-path scoped query — **must not allocate heap memory**.
pub fn execute_scoped_query_in_place(
    app_id_hash: u64,
    query_bytecode: &[u8],
    geometry_out: &mut [f32],
) -> Result<usize, ExecutionError> {
    let caps = get_compiled_capability(app_id_hash).ok_or(ExecutionError::UnregisteredApp)?;

    if let Err(err @ ExecutionError::SanctuaryOverrideRequired) =
        verify_execution_scope(&caps, query_bytecode)
    {
        append_sanctuary_violation(app_id_hash, err.as_str());
        return Err(err);
    }

    let graph_guard = daemon_graph::graph_read_guard();
    let db = graph_guard.as_slice();

    let mut result_buffer = [QualiaQuin::default(); MAX_QUERY_RESULTS];
    let (match_count, _vm_cycles) =
        webizen_bytecode::execute_program(query_bytecode, db, &mut result_buffer).map_err(
            |e| match e {
                webizen_bytecode::VmError::OutputBufferFull => ExecutionError::OutputBufferFull,
                webizen_bytecode::VmError::InvalidProgram => ExecutionError::InvalidBytecode,
            },
        )?;

    let mut float_offset = 0usize;
    for quin in &result_buffer[..match_count] {
        if !result_passes_clearance(quin, caps.clearance_level) {
            append_sanctuary_violation(app_id_hash, ExecutionError::ClearanceViolation.as_str());
            return Err(ExecutionError::ClearanceViolation);
        }
        let written = translate_quin_to_geometry(quin, geometry_out, float_offset);
        if written == 0 {
            return Err(ExecutionError::OutputBufferFull);
        }
        float_offset += written;
    }

    Ok(float_offset)
}

/// FRB/diagnostic wrapper — may allocate when copying geometry for Dart.
pub fn execute_scoped_query(
    app_id_hash: u64,
    query_bytecode: Vec<u8>,
) -> Result<Vec<f32>, ExecutionError> {
    let mut scratch = [0f32; SCOPED_QUERY_MAX_FLOATS];
    let len = execute_scoped_query_in_place(app_id_hash, &query_bytecode, &mut scratch)?;
    Ok(scratch[..len].to_vec())
}

/// Compile a wildcard anatomy graph query into fixed bytecode (install/boot helper).
pub fn compile_wildcard_graph_query(program: &mut [u8; 1024]) -> Result<usize, ExecutionError> {
    mini_parser::compile_ntriples_to_bytecode(b"?subject ?predicate ?object .", program)
        .map_err(|_| ExecutionError::InvalidBytecode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::qapp_manifest::{compile_and_register_qapp, CapabilityClaims, HostMetadata, QappManifest};
    use qualia_core_db::daemon_graph;

    fn register_anatomy_app() -> u64 {
        let manifest = QappManifest {
            app_id: "did:qualia:qapp:anatomy".to_string(),
            host_metadata: HostMetadata::default(),
            capability_claims: CapabilityClaims {
                required_ontologies: vec![
                    "q42:anatomy".to_string(),
                    "https://qualia.anatomy.example/ontology/bio#".to_string(),
                ],
                optional_remote_endpoints: vec![],
                max_sensitivity_clearance: "0x00".to_string(),
            },
        };
        compile_and_register_qapp(manifest).unwrap()
    }

    #[test]
    fn rejects_forbidden_domain_operands() {
        let app_id_hash = register_anatomy_app();
        let caps = get_compiled_capability(app_id_hash).unwrap();

        let mut prog = [0u8; 64];
        prog[0] = mini_parser::OP_MATCH_SUBJECT;
        let forbidden = q_hash("q42:financial");
        prog[1..9].copy_from_slice(&forbidden.to_le_bytes());
        prog[9] = mini_parser::OP_END;

        assert_eq!(
            verify_execution_scope(&caps, &prog[..10]),
            Err(ExecutionError::SanctuaryOverrideRequired)
        );
    }

    #[test]
    fn zero_allocation_scoped_query_hot_path() {
        daemon_graph::init_daemon_graph("/tmp/qualia-qapp-test");
        let app_id_hash = register_anatomy_app();

        let mut prog = [0u8; 1024];
        let prog_len =
            mini_parser::compile_ntriples_to_bytecode(b"?subject ?predicate ?object .", &mut prog)
                .unwrap();

        let _profiler = dhat::Profiler::builder().testing().build();
        let mut out = [0f32; 512];
        let result = execute_scoped_query_in_place(app_id_hash, &prog[..prog_len], &mut out);
        assert!(result.is_ok());
        assert!(result.unwrap() > 0);

        let stats = dhat::HeapStats::get();
        assert_eq!(
            stats.curr_blocks, 0,
            "execute_scoped_query_in_place must not allocate on the heap"
        );
        assert_eq!(stats.curr_bytes, 0);
    }
}

// ── DICOM split-ingest (Core 3) + volume query (FRB boundary) ─────────────────

fn storage_root() -> Result<std::path::PathBuf, String> {
    let state = crate::state::APP_STATE.get().ok_or("APP_STATE not initialized")?;
    Ok(std::path::PathBuf::from(
        state.config.lock().map_err(|e| e.to_string())?.storage_path.clone(),
    ))
}

/// Submit `.dcm` split-ingest to Core 3; returns lock-free job id.
pub fn submit_dicom_ingest(file_path: String, patient_did_hash: u64) -> Result<u64, String> {
    let root = storage_root()?;
    qualia_core_db::dicom_ingest::init_core3_dicom_worker(root.clone());
    qualia_core_db::dicom_ingest::submit_dicom_ingest(
        &root,
        std::path::Path::new(&file_path),
        patient_did_hash,
    )
    .map_err(|e| e.to_string())
}

/// Poll Core 3 ingest status: 0=pending, 1=complete, 2=failed.
pub fn dicom_ingest_status(job_id: u64) -> u8 {
    qualia_core_db::dicom_ingest::dicom_ingest_status(job_id)
}

/// Memory-map read of ingested pixel payload (single copy for Dart `Uint8List`).
pub fn execute_dicom_volume_query(
    patient_did_hash: u64,
    series_hash: u64,
) -> Result<Vec<u8>, String> {
    let root = storage_root()?;
    let record = qualia_core_db::dicom_ingest::find_series_record(patient_did_hash, series_hash)
        .ok_or_else(|| "DICOM series not found in ingest registry".to_string())?;
    qualia_core_db::dicom_ingest::read_volume_bytes(&root, &record).map_err(|e| e.to_string())
}

/// Comorbidity verdicts using the in-process daemon graph (FRB-friendly).
pub fn eval_comorbidity_json_from_daemon(
    patient_did_hash: u64,
    target_organ_hash: u64,
) -> Result<String, String> {
    let graph_guard = qualia_core_db::daemon_graph::graph_read_guard();
    eval_comorbidity_json(patient_did_hash, target_organ_hash, graph_guard.as_slice())
}

/// Hot-path comorbidity evaluation — stack buffers only inside engine call.
pub fn eval_comorbidity_json(
    patient_did_hash: u64,
    target_organ_hash: u64,
    graph_quins: &[qualia_core_db::QualiaQuin],
) -> Result<String, String> {
    use qualia_core_db::comorbidity_eval::{eval_comorbidity, ComorbidityVerdict, MAX_COMORBIDITY_VERDICTS};

    let mut out = [ComorbidityVerdict {
        condition_hash: 0,
        compounded_risk_milli: 0,
        status: qualia_core_db::comorbidity_eval::ComorbidityStatus::Active,
        _pad: [0; 3],
    }; MAX_COMORBIDITY_VERDICTS];

    let count = eval_comorbidity(patient_did_hash, target_organ_hash, graph_quins, &mut out)
        .map_err(|e| format!("{e:?}"))?;

    let verdicts: Vec<serde_json::Value> = out[..count]
        .iter()
        .map(|v| {
            serde_json::json!({
                "conditionHash": format!("{:#018x}", v.condition_hash),
                "compoundedRiskMilli": v.compounded_risk_milli,
                "status": v.status as u8,
            })
        })
        .collect();

    serde_json::to_string(&serde_json::json!({ "verdicts": verdicts })).map_err(|e| e.to_string())
}
