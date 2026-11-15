use super::{binding, observation, Authority};
use crate::{
    bridge_observation,
    container_custody::{
        bind_container_observation, read_container_observation, ContainerBindingAuthority,
        ContainerReadError, ContainerSelectionError,
    },
    read_store::StoredReadAuthority,
    sidecar::{encode_sidecar, UntrustedSidecar, SIDECAR_SIZE},
};
use ed25519_dalek::SigningKey;
use qualia_core_db::{
    container_10d as c,
    sensing::{
        container::{pack_sensing_index_field, ResolvedSensingMeasurement, SensingIndexView},
        local_custody::{publish_bundle, CustodyAuthority},
        record::{MeasurementRecord, ResolvedFieldPayloadBinding},
        sidecar_attestation::{SidecarReceipt, SidecarReceiptAuthority, SIDECAR_BYTES},
    },
};
use std::path::Path;

const KIND: u16 = 0x8101;
const BUDGET: usize = 16 + SIDECAR_SIZE + 272 + 16;

#[path = "container_recovery_tests.rs"]
mod recovery_tests;

#[path = "container_payload_budget_tests.rs"]
mod payload_budget_tests;

fn container(record: MeasurementRecord) -> ([u8; 8192], usize) {
    let raw = c::TypedFieldRecord {
        field_id: 9,
        kind: c::FIELD_KIND_SCALAR,
        flags: 0,
        unit_id: 11,
        payload_section_id: c::SectionType::FieldSidecar as u32,
        payload_offset: 0,
        payload_length: 16,
    };
    let mut packed = [0u8; 16 + 264];
    let index =
        pack_sensing_index_field([1; 16], 10, KIND, &[5; 16], &[record], &mut packed).unwrap();
    let section = c::SectionInput {
        section_type: c::SectionType::FieldSidecar,
        alignment_tier: c::AlignmentTier::Word,
        stride: 0,
        element_count: 0,
        payload: &packed,
    };
    let identity = c::ManifoldIdentityV2 {
        entity_id: c::StableEntityId([7; 16]),
        profile_id: 1,
        domain_id: 2,
        coordinate_convention_id: 3,
        coordinates: [0.0; 10],
        external_digest: None,
    };
    let mut bytes = [0; 8192];
    let size = c::encode_container_with_manifold_identity_extensions(
        &c::Container10dHeader::proposed(),
        &[section],
        &identity,
        &[raw, index.index_field],
        &[KIND],
        &mut [0; 4096],
        &mut [section; 2],
        &mut bytes,
    )
    .unwrap();
    (bytes, size)
}

struct Permit(bool);
impl ContainerBindingAuthority for Permit {
    fn permit_binding(
        &self,
        _: &ResolvedSensingMeasurement<'_>,
        _: &crate::BridgedObservation,
    ) -> bool {
        self.0
    }
}
impl SidecarReceiptAuthority for Permit {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        self.0
    }
}
impl CustodyAuthority for Permit {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
        _: usize,
    ) -> bool {
        self.0
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        self.0
    }
}
impl StoredReadAuthority for Permit {
    fn permit_decoded(
        &mut self,
        _: &Path,
        _: &[u8; 32],
        _: &ResolvedFieldPayloadBinding,
        _: &SidecarReceipt,
        _: &UntrustedSidecar,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
    ) -> bool {
        self.0
    }
}

fn bundle(value: &crate::BridgedObservation, payload: &[u8]) -> (tempfile::TempDir, [u8; 32]) {
    let mut sidecar = [0; SIDECAR_SIZE];
    encode_sidecar(value, &mut sidecar).unwrap();
    let root = tempfile::tempdir().unwrap();
    let receipt = SidecarReceipt::sign(
        &sidecar,
        payload,
        16,
        [1; 32],
        [2; 32],
        0,
        None,
        &SigningKey::from_bytes(&[11; 32]),
    )
    .unwrap();
    let published = publish_bundle(
        root.path(),
        &sidecar,
        payload,
        &receipt,
        BUDGET,
        &mut Permit(true),
    )
    .unwrap();
    (root, published.checkpoint)
}

#[test]
fn selection_binds_canonical_row_and_borrows_exact_payload() {
    let value = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let (bytes, size) = container(*value.record());
    let mut scratch = [c::TypedFieldRecord::default(); 2];
    let mut fields = [c::TypedFieldRecord::default(); 2];
    let resolver =
        c::TypedPayloadResolver::new(&bytes[..size], &[KIND], &mut scratch, &mut fields).unwrap();
    let view = SensingIndexView::open(&resolver, 10, KIND, [1; 16]).unwrap();
    let select = || view.resolve_record(0, binding().physical()).unwrap();
    assert!(matches!(
        bind_container_observation(select(), &value, 15, &Permit(true)),
        Err(ContainerSelectionError::PayloadBudget)
    ));
    assert!(matches!(
        bind_container_observation(select(), &value, 16, &Permit(false)),
        Err(ContainerSelectionError::NotAuthorized)
    ));
    let other = bridge_observation(&Authority, 22, 17, observation(), binding()).unwrap();
    assert!(matches!(
        bind_container_observation(select(), &other, 16, &Permit(true)),
        Err(ContainerSelectionError::RecordMismatch)
    ));
    let bound = bind_container_observation(select(), &value, 16, &Permit(true)).unwrap();
    assert_eq!(
        bound.selected().payload().as_ptr(),
        resolver.field(9).unwrap().unwrap().bytes.as_ptr()
    );
}

#[test]
fn valid_receipt_for_different_payload_cannot_replace_selected_container_bytes() {
    let value = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let (bytes, size) = container(*value.record());
    let mut scratch = [c::TypedFieldRecord::default(); 2];
    let mut fields = [c::TypedFieldRecord::default(); 2];
    let resolver =
        c::TypedPayloadResolver::new(&bytes[..size], &[KIND], &mut scratch, &mut fields).unwrap();
    let view = SensingIndexView::open(&resolver, 10, KIND, [1; 16]).unwrap();
    let selected = view.resolve_record(0, binding().physical()).unwrap();
    let mut sidecar = [0; SIDECAR_SIZE];
    encode_sidecar(&value, &mut sidecar).unwrap();
    let root = tempfile::tempdir().unwrap();
    let receipt = SidecarReceipt::sign(
        &sidecar,
        &[6; 16],
        16,
        [1; 32],
        [2; 32],
        0,
        None,
        &SigningKey::from_bytes(&[11; 32]),
    )
    .unwrap();
    let published = publish_bundle(
        root.path(),
        &sidecar,
        &[6; 16],
        &receipt,
        BUDGET,
        &mut Permit(true),
    )
    .unwrap();
    let mut output = [0xaa; 32];
    assert!(matches!(
        read_container_observation(
            root.path(),
            &published.checkpoint,
            BUDGET,
            16,
            &selected,
            &mut output,
            &mut Permit(true)
        ),
        Err(ContainerReadError::PayloadMismatch)
    ));
    assert_eq!(output, [0; 32]);
}

#[test]
fn container_bound_read_returns_exact_record_payload_and_clears_tail() {
    let value = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let (bytes, size) = container(*value.record());
    let mut scratch = [c::TypedFieldRecord::default(); 2];
    let mut fields = [c::TypedFieldRecord::default(); 2];
    let resolver =
        c::TypedPayloadResolver::new(&bytes[..size], &[KIND], &mut scratch, &mut fields).unwrap();
    let view = SensingIndexView::open(&resolver, 10, KIND, [1; 16]).unwrap();
    let selected = view.resolve_record(0, binding().physical()).unwrap();
    let (root, checkpoint) = bundle(&value, selected.payload());
    let mut output = [0xaa; 32];
    let read = read_container_observation(
        root.path(),
        &checkpoint,
        BUDGET,
        16,
        &selected,
        &mut output,
        &mut Permit(true),
    )
    .unwrap();
    assert_eq!(*read.historical.record(), selected.record());
    assert_eq!(read.payload_bytes, 16);
    assert_eq!(&output[..16], selected.payload());
    assert_eq!(&output[16..], &[0; 16]);
}

struct PanicDecoded {
    calls: usize,
}
impl SidecarReceiptAuthority for PanicDecoded {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        true
    }
}
impl CustodyAuthority for PanicDecoded {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
        _: usize,
    ) -> bool {
        false
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        true
    }
}
impl StoredReadAuthority for PanicDecoded {
    fn permit_decoded(
        &mut self,
        _: &Path,
        _: &[u8; 32],
        _: &ResolvedFieldPayloadBinding,
        _: &SidecarReceipt,
        decoded: &UntrustedSidecar,
        _: &[u8; SIDECAR_BYTES],
        payload: &[u8],
    ) -> bool {
        self.calls += 1;
        assert_eq!(decoded.record().record_id(), 21);
        assert_eq!(payload, &[5; 16]);
        panic!("decoded authority unwind after verified read")
    }
}

#[test]
fn container_read_authority_unwind_clears_entire_output() {
    let value = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let (bytes, size) = container(*value.record());
    let mut scratch = [c::TypedFieldRecord::default(); 2];
    let mut fields = [c::TypedFieldRecord::default(); 2];
    let resolver =
        c::TypedPayloadResolver::new(&bytes[..size], &[KIND], &mut scratch, &mut fields).unwrap();
    let view = SensingIndexView::open(&resolver, 10, KIND, [1; 16]).unwrap();
    let selected = view.resolve_record(0, binding().physical()).unwrap();
    let (root, checkpoint) = bundle(&value, selected.payload());
    let mut output = [0xaa; 32];
    let mut policy = PanicDecoded { calls: 0 };
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        read_container_observation(
            root.path(),
            &checkpoint,
            BUDGET,
            16,
            &selected,
            &mut output,
            &mut policy,
        )
    }));
    assert!(result.is_err());
    assert_eq!(policy.calls, 1);
    assert_eq!(output, [0; 32]);
}
