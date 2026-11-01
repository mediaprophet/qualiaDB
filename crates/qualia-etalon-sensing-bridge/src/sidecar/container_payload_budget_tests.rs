use super::*;
use crate::{
    container_custody::{recover_container_observation, ContainerRecoveryError},
    read_store::StoredReadError,
};
use qualia_core_db::sensing::local_custody::{
    chain::{ChainAuthorityError, ChainHead},
    local_head::{LocalHeadLease, LocalHeadPolicy},
    local_recovery::LocalRecoveryPolicy,
    recovery::{MembershipAssertion, RecoveryError},
    CustodyError,
};
use std::cell::Cell;

struct BudgetPolicy {
    receipt_checks: Cell<usize>,
    decoded_checks: usize,
}
impl SidecarReceiptAuthority for BudgetPolicy {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        self.receipt_checks.set(self.receipt_checks.get() + 1);
        true
    }
}
impl CustodyAuthority for BudgetPolicy {
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
impl LocalHeadPolicy for BudgetPolicy {
    fn permit_open(&mut self, _: &Path, _: &[u8; 32]) -> bool {
        true
    }
    fn admit_current(&mut self, _: &Path, _: &[u8; 32], _: ChainHead) -> bool {
        true
    }
    fn permit_commit(
        &mut self,
        _: &Path,
        _: &[u8; 32],
        _: ChainHead,
        _: ChainHead,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
    ) -> bool {
        false
    }
}
impl LocalRecoveryPolicy for BudgetPolicy {
    fn permit_recovery_read(&mut self, _: &Path, _: &[u8; 32], _: &[u8; 32], _: usize) -> bool {
        true
    }
    fn verify_recovery_membership(
        &mut self,
        _: &Path,
        _: &[u8; 32],
        _: ChainHead,
        _: &SidecarReceipt,
    ) -> Result<MembershipAssertion, ChainAuthorityError> {
        panic!("oversized payload reached membership verification")
    }
}
impl StoredReadAuthority for BudgetPolicy {
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
        self.decoded_checks += 1;
        true
    }
}

fn oversized_case(recovery: bool) {
    let value = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let (bytes, size) = container(*value.record());
    let mut scratch = [c::TypedFieldRecord::default(); 2];
    let mut fields = [c::TypedFieldRecord::default(); 2];
    let resolver =
        c::TypedPayloadResolver::new(&bytes[..size], &[KIND], &mut scratch, &mut fields).unwrap();
    let view = SensingIndexView::open(&resolver, 10, KIND, [1; 16]).unwrap();
    let selected = view.resolve_record(0, binding().physical()).unwrap();
    assert_eq!(selected.payload().len(), 16);
    // The larger bundle is correctly signed and admitted by the test store,
    // but its declared payload must not consume the caller's spare capacity.
    let mut sidecar = [0; SIDECAR_SIZE];
    encode_sidecar(&value, &mut sidecar).unwrap();
    let root = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[11; 32]);
    let receipt =
        SidecarReceipt::sign(&sidecar, &[6; 24], 24, [1; 32], [2; 32], 0, None, &key).unwrap();
    let bundle_budget = BUDGET + 8;
    let published = publish_bundle(
        root.path(),
        &sidecar,
        &[6; 24],
        &receipt,
        bundle_budget,
        &mut Permit(true),
    )
    .unwrap();
    let mut policy = BudgetPolicy {
        receipt_checks: Cell::new(0),
        decoded_checks: 0,
    };
    let mut output = [0xaa; 64];
    if recovery {
        let mut lease =
            LocalHeadLease::acquire(root.path(), [2; 32], &key, 176, &mut policy).unwrap();
        assert!(matches!(
            recover_container_observation(
                root.path(),
                &[2; 32],
                &published.checkpoint,
                bundle_budget,
                16,
                &selected,
                &mut output,
                &mut lease,
                &mut policy
            ),
            Err(ContainerRecoveryError::Recovery(RecoveryError::Custody(
                CustodyError::TooLarge
            )))
        ));
    } else {
        assert!(matches!(
            read_container_observation(
                root.path(),
                &published.checkpoint,
                bundle_budget,
                16,
                &selected,
                &mut output,
                &mut policy
            ),
            Err(ContainerReadError::Read(StoredReadError::Custody(
                CustodyError::TooLarge
            )))
        ));
    }
    assert_eq!(policy.receipt_checks.get(), 0);
    assert_eq!(policy.decoded_checks, 0);
    assert_eq!(output, [0; 64]);
}

#[test]
fn read_rejects_signed_payload_above_selected_budget_before_verification() {
    oversized_case(false);
}

#[test]
fn recovery_rejects_signed_payload_above_selected_budget_before_verification() {
    oversized_case(true);
}
