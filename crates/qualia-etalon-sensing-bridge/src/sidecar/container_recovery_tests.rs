use super::*;
use crate::container_custody::{recover_container_observation, ContainerRecoveryError};
use qualia_core_db::sensing::local_custody::{
    chain::{ChainAuthorityError, ChainHead},
    local_head::{LocalHeadLease, LocalHeadPolicy},
    local_recovery::LocalRecoveryPolicy,
    recovery::{MembershipAssertion, RecoveryDisposition},
};

struct RecoveryPolicy {
    deny_decoded: bool,
    panic_decoded: bool,
    decoded_calls: usize,
}
impl SidecarReceiptAuthority for RecoveryPolicy {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        true
    }
}
impl CustodyAuthority for RecoveryPolicy {
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
impl LocalHeadPolicy for RecoveryPolicy {
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
impl LocalRecoveryPolicy for RecoveryPolicy {
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
        Ok(MembershipAssertion::Unknown)
    }
}
impl StoredReadAuthority for RecoveryPolicy {
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
        self.decoded_calls += 1;
        assert_eq!(decoded.record().record_id(), 21);
        assert_eq!(payload, &[5; 16]);
        if self.panic_decoded {
            panic!("late decoded recovery authority failure");
        }
        !self.deny_decoded
    }
}

fn run_case(deny: bool, panic_decoded: bool, substituted: bool) {
    let value = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
    let (bytes, size) = container(*value.record());
    let mut scratch = [c::TypedFieldRecord::default(); 2];
    let mut fields = [c::TypedFieldRecord::default(); 2];
    let resolver =
        c::TypedPayloadResolver::new(&bytes[..size], &[KIND], &mut scratch, &mut fields).unwrap();
    let view = SensingIndexView::open(&resolver, 10, KIND, [1; 16]).unwrap();
    let selected = view.resolve_record(0, binding().physical()).unwrap();
    let source_payload = if substituted {
        &[6; 16][..]
    } else {
        selected.payload()
    };
    let (root, checkpoint) = bundle(&value, source_payload);
    let key = SigningKey::from_bytes(&[11; 32]);
    let mut policy = RecoveryPolicy {
        deny_decoded: deny,
        panic_decoded,
        decoded_calls: 0,
    };
    let mut lease = LocalHeadLease::acquire(root.path(), [2; 32], &key, 176, &mut policy).unwrap();
    let mut output = [0xaa; 32];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        recover_container_observation(
            root.path(),
            &[2; 32],
            &checkpoint,
            BUDGET,
            16,
            &selected,
            &mut output,
            &mut lease,
            &mut policy,
        )
    }));
    if panic_decoded {
        assert!(result.is_err());
        assert_eq!(policy.decoded_calls, 1);
        assert_eq!(output, [0; 32]);
    } else if substituted {
        assert!(matches!(
            result.unwrap(),
            Err(ContainerRecoveryError::PayloadMismatch)
        ));
        assert_eq!(policy.decoded_calls, 0);
        assert_eq!(output, [0; 32]);
    } else if deny {
        assert!(matches!(
            result.unwrap(),
            Err(ContainerRecoveryError::NotAuthorized)
        ));
        assert_eq!(policy.decoded_calls, 1);
        assert_eq!(output, [0; 32]);
    } else {
        let recovered = result.unwrap().unwrap();
        assert_eq!(
            recovered.recovery.disposition,
            RecoveryDisposition::Uncertain
        );
        assert_eq!(*recovered.historical.record(), selected.record());
        assert_eq!(&output[..16], selected.payload());
        assert_eq!(&output[16..], &[0; 16]);
    }
}

#[test]
fn exact_read_keeps_membership_uncertain_without_proof() {
    run_case(false, false, false);
}
#[test]
fn signed_substituted_payload_is_rejected_and_wiped() {
    run_case(false, false, true);
}
#[test]
fn decoded_denial_or_unwind_wipes_recovered_output() {
    run_case(true, false, false);
    run_case(false, true, false);
}
