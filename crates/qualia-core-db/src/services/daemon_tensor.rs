//! Cold-path export: daemon graph `NQuin` slice → binary `Tensor10D` SOA for U2 viewport upload.
//!
//! Layout: `TensorBufferHeader` (32 B) + N × `Tensor10D` (40 B). Served at `GET /tensor/slice`.
//! Identifier/Vault standpoints (class ≥ 2) require Ed25519 over the canonical request string.

use crate::key_vault::KeyVault;
use crate::render::telemetry::{STANDPOINT_DID, STANDPOINT_VAULT};
use crate::tensor::bake_pipeline::bake_quin_to_tensor;
use crate::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};
use crate::NQuin;

pub const DEFAULT_SLICE_MAX_NODES: usize = 12_000;
pub const ABSOLUTE_SLICE_MAX_NODES: usize = 50_000;

/// Routing lane mirrored from `ObserverStandpoint` (PR-C9c.2 vault vs commons).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorSliceLane {
    Commons,
    Identifier,
}

impl TensorSliceLane {
    pub fn from_header(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "identifier" | "did" | "vault" => Self::Identifier,
            _ => Self::Commons,
        }
    }
}

/// Standpoint-aligned slice parameters (matches WGSL temporal discard).
#[derive(Debug, Clone, Copy)]
pub struct TensorSliceRequest {
    pub max_nodes: usize,
    pub t_slice: f32,
    pub t_window: f32,
    pub lane: TensorSliceLane,
    pub standpoint_class: u32,
}

impl Default for TensorSliceRequest {
    fn default() -> Self {
        Self {
            max_nodes: DEFAULT_SLICE_MAX_NODES,
            t_slice: 0.5,
            t_window: 1.0,
            lane: TensorSliceLane::Commons,
            standpoint_class: 0,
        }
    }
}

impl TensorSliceRequest {
    #[inline]
    pub fn clamp_max_nodes(max_nodes: usize) -> usize {
        max_nodes.clamp(1, ABSOLUTE_SLICE_MAX_NODES)
    }

    #[inline]
    pub fn temporal_passes(&self, tensor_t: f32) -> bool {
        if self.t_window <= 0.0 {
            return true;
        }
        (tensor_t - self.t_slice).abs() <= self.t_window
    }

    #[inline]
    pub fn requires_identifier_auth(&self) -> bool {
        self.standpoint_class >= STANDPOINT_DID
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum TensorSliceError {
    EmptyGraph,
    BufferTooSmall,
}

#[derive(Debug, PartialEq, Eq)]
pub enum TensorSliceAuthError {
    IdentifierDidRequired,
    SessionNonceRequired,
    SignatureRequired,
    InvalidSignatureEncoding,
    InvalidSignature,
}

/// Canonical UTF-8 bytes both client (`crypto.subtle`) and daemon must sign/verify.
/// Format: `"{nonce}|{standpoint_class}|{t_slice}|{t_window}"`
pub fn canonical_tensor_slice_payload(
    nonce: &str,
    standpoint_class: u32,
    t_slice: f32,
    t_window: f32,
) -> String {
    format!(
        "{}|{}|{}|{}",
        nonce,
        standpoint_class,
        format_canonical_f32(t_slice),
        format_canonical_f32(t_window),
    )
}

#[inline]
fn format_canonical_f32(v: f32) -> String {
    let s = format!("{:.6}", v);
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Verify Ed25519 signature for identifier/vault tensor slice requests (fail closed).
pub fn verify_tensor_slice_signature(
    vault: &KeyVault,
    identifier_did: &str,
    nonce: &str,
    standpoint_class: u32,
    t_slice: f32,
    t_window: f32,
    signature_hex: &str,
) -> Result<(), TensorSliceAuthError> {
    if identifier_did.trim().is_empty() {
        return Err(TensorSliceAuthError::IdentifierDidRequired);
    }
    if nonce.trim().is_empty() {
        return Err(TensorSliceAuthError::SessionNonceRequired);
    }
    if signature_hex.trim().is_empty() {
        return Err(TensorSliceAuthError::SignatureRequired);
    }

    let payload = canonical_tensor_slice_payload(nonce, standpoint_class, t_slice, t_window);
    let sig_bytes = hex::decode(signature_hex.trim())
        .map_err(|_| TensorSliceAuthError::InvalidSignatureEncoding)?;
    if sig_bytes.len() != 64 {
        return Err(TensorSliceAuthError::InvalidSignatureEncoding);
    }
    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);

    let pk = vault.public_key_bytes_for_context(identifier_did);
    KeyVault::verify_signature(&pk, payload.as_bytes(), &sig_arr)
        .map_err(|_| TensorSliceAuthError::InvalidSignature)
}

/// Commons = public sensitivity only; identifier/vault = full sealed graph.
#[inline]
pub fn quin_matches_lane(quin: &NQuin, lane: TensorSliceLane) -> bool {
    match lane {
        TensorSliceLane::Commons => quin.get_sensitivity_byte() == NQuin::SENSITIVITY_PUBLIC,
        TensorSliceLane::Identifier => true,
    }
}

/// Bake filtered graph quins into a tensor buffer blob (cold path; heap OK).
pub fn build_tensor_slice_bytes(
    quins: &[NQuin],
    req: &TensorSliceRequest,
) -> Result<Vec<u8>, TensorSliceError> {
    if quins.is_empty() {
        return Err(TensorSliceError::EmptyGraph);
    }

    let cap = TensorSliceRequest::clamp_max_nodes(req.max_nodes);
    let mut tensors = Vec::with_capacity(cap.min(quins.len()));

    for q in quins {
        if !quin_matches_lane(q, req.lane) {
            continue;
        }
        let t = bake_quin_to_tensor(q);
        if !req.temporal_passes(t.t) {
            continue;
        }
        tensors.push(t);
        if tensors.len() >= cap {
            break;
        }
    }

    if tensors.is_empty() {
        return Err(TensorSliceError::EmptyGraph);
    }

    let need = TensorBufferHeader::total_bytes(tensors.len());
    let mut buf = vec![0u8; need];
    write_tensor_buffer(&tensors, &mut buf).map_err(|_| TensorSliceError::BufferTooSmall)?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::Signer;

    fn triple_quin(subject: &str, predicate: &str, object: &str, context: &str) -> NQuin {
        let subject = crate::q_hash(subject);
        let predicate = crate::q_hash(predicate);
        let object = crate::q_hash(object);
        let context = crate::q_hash(context) & 0x00FF_FFFF_FFFF_FFFF;
        NQuin {
            subject,
            predicate,
            object,
            context,
            metadata: 0,
            parity: subject ^ predicate ^ object ^ context,
        }
    }

    fn sample_quin() -> NQuin {
        triple_quin(
            "http://q.test/s/0",
            "http://q.test/p/geo",
            "http://q.test/o/0",
            "did:qualia:commons",
        )
    }

    fn test_vault() -> KeyVault {
        KeyVault::new()
    }

    #[test]
    fn canonical_payload_matches_spec_example() {
        assert_eq!(
            canonical_tensor_slice_payload("a1b2c3d4e5f6", 2, 0.5, 0.1),
            "a1b2c3d4e5f6|2|0.5|0.1"
        );
    }

    #[test]
    fn verify_tensor_slice_signature_round_trip() {
        let vault = test_vault();
        let did = "did:qualia:alice";
        let nonce = "a1b2c3d4e5f6";
        let sk = vault.derive_key(did);
        let payload = canonical_tensor_slice_payload(nonce, STANDPOINT_DID, 0.5, 0.1);
        let sig = sk.sign(payload.as_bytes());
        let sig_hex = hex::encode(sig.to_bytes());
        verify_tensor_slice_signature(&vault, did, nonce, STANDPOINT_DID, 0.5, 0.1, &sig_hex)
            .expect("valid signature");
    }

    #[test]
    fn verify_rejects_wrong_nonce() {
        let vault = test_vault();
        let did = "did:qualia:alice";
        let sk = vault.derive_key(did);
        let payload = canonical_tensor_slice_payload("nonce-a", STANDPOINT_DID, 0.5, 0.1);
        let sig_hex = hex::encode(sk.sign(payload.as_bytes()).to_bytes());
        assert_eq!(
            verify_tensor_slice_signature(
                &vault,
                did,
                "nonce-b",
                STANDPOINT_DID,
                0.5,
                0.1,
                &sig_hex
            ),
            Err(TensorSliceAuthError::InvalidSignature)
        );
    }

    #[test]
    fn commons_lane_excludes_restricted_quins() {
        let mut public = sample_quin();
        let mut restricted = sample_quin();
        restricted.set_sensitivity_byte(NQuin::SENSITIVITY_RESTRICTED);
        let req = TensorSliceRequest {
            lane: TensorSliceLane::Commons,
            t_window: 0.0,
            max_nodes: 8,
            ..Default::default()
        };
        let buf = build_tensor_slice_bytes(&[public, restricted], &req).expect("public only");
        assert_eq!(
            crate::tensor::buffer_export::tensor_node_count(&buf).expect("count"),
            1
        );
    }

    #[test]
    fn identifier_lane_includes_restricted_quins() {
        let mut restricted = sample_quin();
        restricted.set_sensitivity_byte(NQuin::SENSITIVITY_RESTRICTED);
        let req = TensorSliceRequest {
            lane: TensorSliceLane::Identifier,
            standpoint_class: STANDPOINT_VAULT,
            t_window: 0.0,
            max_nodes: 8,
            ..Default::default()
        };
        let buf = build_tensor_slice_bytes(&[restricted], &req).expect("vault slice");
        assert_eq!(
            crate::tensor::buffer_export::tensor_node_count(&buf).expect("count"),
            1
        );
    }

    #[test]
    fn build_slice_respects_max_nodes() {
        let quins = [sample_quin(); 4];
        let req = TensorSliceRequest {
            max_nodes: 2,
            t_window: 0.0,
            ..Default::default()
        };
        let buf = build_tensor_slice_bytes(&quins, &req).expect("slice");
        let count = crate::tensor::buffer_export::tensor_node_count(&buf).expect("header");
        assert_eq!(count, 2);
    }

    #[test]
    fn temporal_filter_matches_standpoint_window() {
        let mut q = sample_quin();
        q.metadata = (100u64) << 32;
        let baked = bake_quin_to_tensor(&q);
        let req = TensorSliceRequest {
            t_slice: baked.t,
            t_window: 0.01,
            max_nodes: 8,
            ..Default::default()
        };
        let buf = build_tensor_slice_bytes(&[q], &req).expect("in window");
        assert_eq!(
            crate::tensor::buffer_export::tensor_node_count(&buf).expect("count"),
            1
        );

        let req_out = TensorSliceRequest {
            t_slice: baked.t + 1.0,
            t_window: 0.01,
            ..req
        };
        assert_eq!(
            build_tensor_slice_bytes(&[q], &req_out),
            Err(TensorSliceError::EmptyGraph)
        );
    }
}
