//! Opt-in local sensing inspection. No capture, model activation or publication.
mod binding;
mod index;
mod local_file;
pub mod release;

use crate::cli::SenseAction;
use qualia_core_db::{
    container_10d::{TypedFieldRecord, TypedPayloadResolver},
    sensing::sidecar_attestation::{SidecarReceipt, SIDECAR_BYTES, SIDECAR_RECEIPT_BYTES},
};
use qualia_etalon_sensing_bridge::sidecar::{decode_sidecar, SIDECAR_SIZE};
use std::{error::Error, io};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;
fn error(message: impl Into<String>) -> Box<dyn Error> {
    Box::new(io::Error::new(io::ErrorKind::InvalidData, message.into()))
}
fn fail<T>(message: impl Into<String>) -> Result<T> {
    Err(error(message))
}

pub fn run(action: &SenseAction) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(&inspect(action)?)?);
    Ok(())
}

/// Cold bounded file inspection with no stdout, capture or publication.
/// The returned JSON report is descriptive data, not a permission token.
pub fn inspect(action: &SenseAction) -> Result<serde_json::Value> {
    if let SenseAction::VerifyRelease {
        package_directory,
        signature,
        trusted_public_key,
        expected_target,
        expected_version,
        max_package_bytes,
    } = action
    {
        return release::verify_release(
            package_directory,
            signature,
            trusted_public_key,
            expected_target,
            expected_version,
            *max_package_bytes,
        );
    }
    let SenseAction::Inspect {
        container,
        sidecar,
        receipt,
        binding,
        index_field_id,
        index_row,
        index_kind,
        max_container_bytes,
        max_payload_bytes,
    } = action
    else {
        return fail("unsupported inspection action");
    };
    let index_selection = index::Selection::optional(*index_field_id, *index_row, *index_kind)?;
    if *max_container_bytes == 0
        || *max_container_bytes > 268_435_456
        || *max_payload_bytes == 0
        || *max_payload_bytes > *max_container_bytes
    {
        return fail("container budget must be 1..=256 MiB and payload budget no greater than container budget");
    }
    for path in [container, sidecar, receipt, binding] {
        local_file::validate_path(path)?;
    }
    let declaration: binding::Binding = serde_json::from_slice(&local_file::read(binding, 4096)?)?;
    declaration.validate()?;
    if let Some(selection) = index_selection {
        selection.require_registered(&declaration.supported_extension_kinds)?;
    }
    let container_bytes = local_file::read(container, *max_container_bytes)?;
    let sidecar_bytes = local_file::read(sidecar, SIDECAR_SIZE)?;
    let receipt_bytes = local_file::read(receipt, SIDECAR_RECEIPT_BYTES)?;
    let sidecar_array: &[u8; SIDECAR_BYTES] = sidecar_bytes
        .as_slice()
        .try_into()
        .map_err(|_| error("incorrect sidecar length"))?;
    let mut scratch = [TypedFieldRecord::default(); 256];
    let mut fields = [TypedFieldRecord::default(); 256];
    let resolver = TypedPayloadResolver::new(
        &container_bytes,
        &declaration.supported_extension_kinds,
        &mut scratch,
        &mut fields,
    )?;
    let field_binding = declaration.resolve(&resolver)?;
    let payload = resolver
        .field(declaration.field_id)?
        .ok_or_else(|| error("field not found"))?;
    if payload.bytes.len() > *max_payload_bytes {
        return fail("payload exceeds byte budget");
    }
    let decoded = decode_sidecar(sidecar_array, field_binding)
        .map_err(|e| error(format!("sidecar validation: {e:?}")))?;
    if let Some(selection) = index_selection {
        selection.verify(&resolver, field_binding, decoded.record())?;
    }
    let signed = SidecarReceipt::decode(&receipt_bytes)
        .map_err(|e| error(format!("receipt decoding: {e:?}")))?;
    signed
        .verify_signature(sidecar_array, payload.bytes, *max_payload_bytes)
        .map_err(|e| error(format!("receipt verification: {e:?}")))?;
    Ok(serde_json::json!({
        "schema": "qualia.sensing.local-inspection.v1",
        "structural_validation_against_supplied_binding": true,
        "signature_valid_for_exact_bytes": true,
        "binding_authenticated": false,
        "index_record_matches_sidecar": index_selection.map(|_| true),
        "index_field_id": index_field_id,
        "index_row": index_row,
        "index_kind": index_kind,
        "index_kind_namespace_authenticated": false,
        "signer_authorized": false,
        "current_permission_established": false,
        "chain_membership": "unassessed",
        "historical_verification_claimed": decoded.evidence_verification_claimed(),
        "payload_bytes": payload.bytes.len(),
        "container_bytes": container_bytes.len()
    }))
}
