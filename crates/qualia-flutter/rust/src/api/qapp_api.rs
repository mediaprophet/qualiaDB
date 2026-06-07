//! Flutter Rust Bridge entry points for the Qapp Manifest execution boundary.

use flutter_rust_bridge::frb;
use qualia_client_core::qapp_api::{
    compile_wildcard_graph_query, execute_scoped_query, execute_scoped_query_in_place,
    ExecutionError, SCOPED_QUERY_MAX_FLOATS,
};
pub use qualia_client_core::qapp_manifest::{
    CapabilityClaims, HostMetadata, QappManifest,
};
use qualia_client_core::qapp_manifest::{compile_and_register_qapp, qapp_manifest_from_package};
use qualia_core_db::q_hash;

#[frb(sync)]
pub fn register_qapp_manifest(manifest: QappManifest) -> Result<u64, String> {
    compile_and_register_qapp(manifest).map_err(|e| format!("{e:?}"))
}

#[frb(sync)]
pub fn register_qapp_from_installed_manifest(qapp_name: String) -> Result<u64, String> {
    let manifest = qualia_client_core::api::load_installed_qapp_package(&qapp_name)?;
    let qapp = qapp_manifest_from_package(&manifest);
    compile_and_register_qapp(qapp).map_err(|e| format!("{e:?}"))
}

#[frb(sync)]
pub fn qapp_id_hash(app_id: String) -> u64 {
    q_hash(&app_id)
}

#[frb(sync)]
pub fn compile_anatomy_wildcard_query() -> Result<Vec<u8>, String> {
    let mut prog = [0u8; 1024];
    let len = compile_wildcard_graph_query(&mut prog).map_err(|e| e.as_str().to_string())?;
    Ok(prog[..len].to_vec())
}

#[frb(sync)]
pub fn execute_qapp_scoped_query(app_id_hash: u64, query_bytecode: Vec<u8>) -> Result<Vec<f32>, String> {
    execute_scoped_query(app_id_hash, query_bytecode).map_err(|e| e.as_str().to_string())
}

/// Zero-copy friendly result: fixed scratch buffer length + values copied once for Dart.
#[frb(sync)]
pub fn execute_qapp_scoped_query_zero_alloc(
    app_id_hash: u64,
    query_bytecode: Vec<u8>,
) -> Result<Vec<f32>, String> {
    let mut scratch = [0f32; SCOPED_QUERY_MAX_FLOATS];
    let len = execute_scoped_query_in_place(app_id_hash, &query_bytecode, &mut scratch)
        .map_err(|e| e.as_str().to_string())?;
    Ok(scratch[..len].to_vec())
}

#[frb(sync)]
pub fn submit_dicom_ingest(file_path: String, patient_did_hash: u64) -> Result<u64, String> {
    qualia_client_core::qapp_api::submit_dicom_ingest(file_path, patient_did_hash)
}

#[frb(sync)]
pub fn dicom_ingest_status(job_id: u64) -> u8 {
    qualia_client_core::qapp_api::dicom_ingest_status(job_id)
}

#[frb(sync)]
pub fn execute_dicom_volume_query(
    patient_did_hash: u64,
    series_hash: u64,
) -> Result<Vec<u8>, String> {
    qualia_client_core::qapp_api::execute_dicom_volume_query(patient_did_hash, series_hash)
}

#[frb(sync)]
pub fn eval_comorbidity_json_from_daemon(
    patient_did_hash: u64,
    target_organ_hash: u64,
) -> Result<String, String> {
    qualia_client_core::qapp_api::eval_comorbidity_json_from_daemon(
        patient_did_hash,
        target_organ_hash,
    )
}

#[frb(sync)]
pub fn installed_qapp_version(qapp_name: String) -> Result<Option<String>, String> {
    qualia_client_core::api::installed_qapp_version(&qapp_name)
}

#[frb(sync)]
pub fn check_qapp_update(qapp_name: String) -> Result<String, String> {
    qualia_client_core::api::check_qapp_update(qapp_name)
}

#[frb(sync)]
pub fn check_qapp_update_from_path(qapp_name: String, source_path: String) -> Result<String, String> {
    qualia_client_core::api::check_qapp_update_from_path(qapp_name, source_path)
}

#[frb(sync)]
pub fn list_qapp_update_offers() -> Result<String, String> {
    qualia_client_core::api::list_qapp_update_offers()
}

#[frb(sync)]
pub fn apply_qapp_update(qapp_name: String) -> Result<String, String> {
    qualia_client_core::api::apply_qapp_update(qapp_name)
}

#[frb(sync)]
pub fn apply_qapp_update_from_path(qapp_name: String, source_path: String) -> Result<String, String> {
    qualia_client_core::api::apply_qapp_update_from_path(qapp_name, source_path)
}

#[frb(sync)]
pub fn execution_error_label(code: String) -> String {
    match code.as_str() {
        "unregistered_app" => ExecutionError::UnregisteredApp.as_str().to_string(),
        "sanctuary_override_required" => ExecutionError::SanctuaryOverrideRequired.as_str().to_string(),
        "output_buffer_full" => ExecutionError::OutputBufferFull.as_str().to_string(),
        "invalid_bytecode" => ExecutionError::InvalidBytecode.as_str().to_string(),
        "clearance_violation" => ExecutionError::ClearanceViolation.as_str().to_string(),
        other => other.to_string(),
    }
}
