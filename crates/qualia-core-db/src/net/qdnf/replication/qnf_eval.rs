//! E06.5 QNF evaluation: Q42 + existing artifact envelopes vs a QNF extension.
//!
//! Workloads are real in-tree sizes (route edges, contract bundles, ML-DSA-65
//! proofs, evidence originals). Large graph size does not select the format.
//! [`QnfExtensionAdopted`] is the recorded decision and is never implied by
//! the presence of this library or by QNF draft magic.

use crate::crypto::network::types::ML_DSA_65_SIG_LEN;
use crate::net::qdnf::contracts::ContractBundle;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::evidence::promote::{EvidenceRecord, MAX_ORIGINAL};
use crate::net::qdnf::replication::stream::PAGE_BYTES;
use crate::net::qdnf::route::ads::PathAdvert;
use crate::net::qdnf::route::validate::ValidatedEdge;
use crate::NQuin;

/// Candidate QNF header (draft §4.1). Not a frozen on-disk layout.
pub const QNF_HEADER_BYTES: u32 = 256;
/// Directory entry width (draft §4.2).
pub const QNF_DIR_ENTRY_BYTES: u32 = 128;
/// AUTH + BIND + CHNK + DATA + OBJS. VIEW is optional and not counted.
pub const QNF_REQUIRED_SECTIONS: u32 = 5;
/// Dual COSE generation proofs: two ML-DSA-65 signatures (draft AUTH).
pub const QNF_AUTH_BYTES: u32 = (ML_DSA_65_SIG_LEN * 2) as u32;
/// Bounded BIND CBOR budget used for the comparison (not a parser).
pub const QNF_BIND_BYTES: u32 = 128;
pub const QNF_CHUNK_DESC_BYTES: u32 = 64;
pub const QNF_OBJECT_DESC_BYTES: u32 = 64;
pub const QNF_CHUNK_DATA_BYTES: u32 = 65536;
/// QNF draft major/minor while unselected. Not frozen (CORE-04.05).
pub const QNF_DRAFT_MAJOR: u16 = 0;
pub const QNF_DRAFT_MINOR: u16 = 1;
/// Existing artifact envelope version: SHA-384 identity + exact length.
pub const ARTIFACT_ENVELOPE_VERSION: u16 = 1;
/// Q42 48-byte projection ABI. Unchanged by this evaluation.
pub const Q42_ABI_BYTES: u16 = 48;

/// ASCII `QNFNET01` from the candidate header. Not an adopted reader magic.
pub const QNF_MAGIC: [u8; 8] = *b"QNFNET01";

/// Recorded E06.5 decision. False: QNF is unselected.
#[allow(non_upper_case_globals)]
pub const QnfExtensionAdopted: bool = EVALUATION.adopted;

/// Evaluation artifact produced from the measured workloads.
pub const EVALUATION: FormatDecision = FormatDecision {
    adopted: false,
    selected: SelectedRepresentation::Q42ExistingArtifacts,
    artifact_version: ARTIFACT_ENVELOPE_VERSION,
    qnf_draft_major: QNF_DRAFT_MAJOR,
    qnf_draft_minor: QNF_DRAFT_MINOR,
    q42_abi_bytes: Q42_ABI_BYTES,
    qnf_header_bytes: QNF_HEADER_BYTES as u16,
    migration: MigrationPath::RetainExisting,
    qnf_layout_frozen: false,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedRepresentation {
    Q42ExistingArtifacts = 1,
    QnfExtension = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MigrationPath {
    /// Stay on Q42 projections + SHA-384 artifact originals. No dual-read.
    RetainExisting = 1,
    /// Only if adopted: digest-keyed dual-read, then a frozen container_qnf.
    DualReadThenQnf = 2,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkloadKind {
    Route = 1,
    Contract = 2,
    PqProof = 3,
    Evidence = 4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkloadCost {
    pub kind: WorkloadKind,
    pub payload_bytes: u32,
    pub q42_envelope_bytes: u32,
    pub qnf_candidate_bytes: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatDecision {
    pub adopted: bool,
    pub selected: SelectedRepresentation,
    pub artifact_version: u16,
    pub qnf_draft_major: u16,
    pub qnf_draft_minor: u16,
    pub q42_abi_bytes: u16,
    pub qnf_header_bytes: u16,
    pub migration: MigrationPath,
    pub qnf_layout_frozen: bool,
}

/// Evaluation artifact: decision plus the four measured costs.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FormatRecord {
    pub decision: FormatDecision,
    pub costs: [WorkloadCost; 4],
    pub large_graph_selects_qnf: bool,
    pub page_window_bytes: u32,
}

/// Measure representative workloads and return the evaluation artifact.
pub fn evaluate_qnf() -> FormatRecord {
    let costs = [
        cost_of(WorkloadKind::Route, route_payload_bytes()),
        cost_of(WorkloadKind::Contract, contract_payload_bytes()),
        cost_of(WorkloadKind::PqProof, pq_payload_bytes()),
        cost_of(WorkloadKind::Evidence, evidence_payload_bytes()),
    ];
    let selected = select_representation(&costs);
    let adopted = selected == SelectedRepresentation::QnfExtension;
    FormatRecord {
        decision: FormatDecision {
            adopted,
            selected,
            artifact_version: ARTIFACT_ENVELOPE_VERSION,
            qnf_draft_major: QNF_DRAFT_MAJOR,
            qnf_draft_minor: QNF_DRAFT_MINOR,
            q42_abi_bytes: Q42_ABI_BYTES,
            qnf_header_bytes: QNF_HEADER_BYTES as u16,
            migration: if adopted {
                MigrationPath::DualReadThenQnf
            } else {
                MigrationPath::RetainExisting
            },
            qnf_layout_frozen: adopted,
        },
        costs,
        large_graph_selects_qnf: false,
        page_window_bytes: PAGE_BYTES as u32,
    }
}

/// QNF magic is Unsupported while unselected. Never parsed as a Q42 volume.
pub fn open_qnf_magic(magic: &[u8; 8]) -> Result<(), QdnfError> {
    if *magic != QNF_MAGIC {
        return Err(QdnfError::Malformed);
    }
    if QnfExtensionAdopted {
        Ok(())
    } else {
        Err(QdnfError::Unsupported)
    }
}

#[inline]
pub fn decision_matches_flag(record: &FormatRecord) -> bool {
    record.decision.adopted == QnfExtensionAdopted
        && (record.decision.adopted
            == (record.decision.selected == SelectedRepresentation::QnfExtension))
}

fn route_payload_bytes() -> u32 {
    (core::mem::size_of::<ValidatedEdge>() + core::mem::size_of::<PathAdvert>()) as u32
}

fn contract_payload_bytes() -> u32 {
    core::mem::size_of::<ContractBundle>() as u32
}

fn pq_payload_bytes() -> u32 {
    ML_DSA_65_SIG_LEN as u32
}

fn evidence_payload_bytes() -> u32 {
    (core::mem::size_of::<EvidenceRecord>() + MAX_ORIGINAL) as u32
}

fn q42_envelope_bytes(payload: u32) -> u32 {
    // SHA-384 identity + u32 length + 48-byte NQuin projection + payload.
    let identity = 48u32.saturating_add(4);
    let projection = core::mem::size_of::<NQuin>() as u32;
    identity
        .saturating_add(projection)
        .saturating_add(payload)
}

fn qnf_candidate_bytes(payload: u32) -> u32 {
    let dir = QNF_DIR_ENTRY_BYTES.saturating_mul(QNF_REQUIRED_SECTIONS);
    let chunks = chunk_descriptors(payload);
    QNF_HEADER_BYTES
        .saturating_add(dir)
        .saturating_add(QNF_AUTH_BYTES)
        .saturating_add(QNF_BIND_BYTES)
        .saturating_add(chunks)
        .saturating_add(payload)
        .saturating_add(QNF_OBJECT_DESC_BYTES)
}

fn chunk_descriptors(payload: u32) -> u32 {
    if payload == 0 {
        return QNF_CHUNK_DESC_BYTES;
    }
    let n = payload.div_ceil(QNF_CHUNK_DATA_BYTES).max(1);
    QNF_CHUNK_DESC_BYTES.saturating_mul(n)
}

fn cost_of(kind: WorkloadKind, payload: u32) -> WorkloadCost {
    WorkloadCost {
        kind,
        payload_bytes: payload,
        q42_envelope_bytes: q42_envelope_bytes(payload),
        qnf_candidate_bytes: qnf_candidate_bytes(payload),
    }
}

/// QNF wins only if it is smaller on a workload *and* it offers a range-read
/// specialization beyond the existing 4 KiB page window. Large graph size is
/// ignored (CORE-04.03).
fn select_representation(costs: &[WorkloadCost; 4]) -> SelectedRepresentation {
    let mut cheaper = false;
    let mut i = 0usize;
    while i < costs.len() {
        if costs[i].qnf_candidate_bytes < costs[i].q42_envelope_bytes {
            cheaper = true;
            break;
        }
        i += 1;
    }
    if cheaper && qnf_range_specialization_beyond_pages() {
        SelectedRepresentation::QnfExtension
    } else {
        SelectedRepresentation::Q42ExistingArtifacts
    }
}

/// Existing replica paging is 4 KiB. QNF draft chunks are 64 KiB — coarser,
/// not a measured specialization that justifies a new container.
const fn qnf_range_specialization_beyond_pages() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_artifact_exists_and_adopted_matches_decision() {
        let record = evaluate_qnf();
        assert_eq!(record.decision, EVALUATION);
        assert!(decision_matches_flag(&record));
        assert!(!QnfExtensionAdopted);
        assert!(!record.decision.adopted);
        assert_eq!(
            record.decision.selected,
            SelectedRepresentation::Q42ExistingArtifacts
        );
        assert_eq!(record.decision.migration, MigrationPath::RetainExisting);
        assert!(!record.decision.qnf_layout_frozen);
        assert!(!record.large_graph_selects_qnf);
        assert_eq!(record.page_window_bytes, PAGE_BYTES as u32);
        assert_eq!(record.decision.q42_abi_bytes, 48);
        assert_eq!(record.decision.artifact_version, ARTIFACT_ENVELOPE_VERSION);
        assert_eq!(record.decision.qnf_draft_major, 0);
        assert_eq!(record.decision.qnf_draft_minor, 1);

        let mut i = 0usize;
        while i < record.costs.len() {
            let c = record.costs[i];
            assert!(c.payload_bytes > 0);
            assert!(c.q42_envelope_bytes > c.payload_bytes);
            assert!(c.qnf_candidate_bytes > c.q42_envelope_bytes);
            i += 1;
        }
    }

    #[test]
    fn qnf_magic_is_unsupported_while_unselected() {
        assert_eq!(open_qnf_magic(&QNF_MAGIC), Err(QdnfError::Unsupported));
        assert_eq!(open_qnf_magic(b"Q42VOL01"), Err(QdnfError::Malformed));
        assert!(!QnfExtensionAdopted);
    }

    #[test]
    fn adopted_flag_is_not_a_silent_true() {
        assert_eq!(QnfExtensionAdopted, EVALUATION.adopted);
        assert_ne!(
            EVALUATION.selected,
            SelectedRepresentation::QnfExtension
        );
        assert_eq!(QnfExtensionAdopted, false);
    }
}
