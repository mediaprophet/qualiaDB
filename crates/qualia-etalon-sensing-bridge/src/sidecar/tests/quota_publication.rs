//! Guarded quota paths that stop before producer filesystem I/O.
use super::*;
use crate::publish_store::{
    publish_prepared_store_with_optional_quota, ExactPublicationAuthority, PublishStoreError,
    StoreConsent,
};
use qualia_core_db::sensing::{
    local_custody::{
        chain::{
            ChainAuthority, ChainAuthorityError, ChainHead, ChainPublicationError, HeadCommit,
        },
        quota::{
            CustodyQuota, QuotaLimits, QuotaPublicationError, QuotaSnapshot, QuotaSnapshotAuthority,
        },
        CustodyAuthority, CustodyError,
    },
    sidecar_attestation::{SidecarReceipt, SidecarReceiptAuthority, SIDECAR_RECEIPT_BYTES},
};
use std::path::Path;

struct Snapshot;
impl QuotaSnapshotAuthority for Snapshot {
    fn admit_usage(&self, _: &Path, _: QuotaLimits, _: QuotaSnapshot) -> bool {
        true
    }
}

struct GuardedHost {
    exact_checks: usize,
    permit_first: bool,
}
impl SidecarReceiptAuthority for GuardedHost {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; 768], _: &[u8]) -> bool {
        true
    }
}
impl CustodyAuthority for GuardedHost {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; 768],
        _: &[u8],
        _: usize,
    ) -> bool {
        true
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        false
    }
}
impl ChainAuthority for GuardedHost {
    fn authenticated_head(
        &mut self,
        _: &Path,
        _: &[u8; 32],
    ) -> Result<ChainHead, ChainAuthorityError> {
        panic!("guard denial must stop before head lookup")
    }
    fn compare_and_commit(
        &mut self,
        _: &Path,
        _: &[u8; 32],
        _: ChainHead,
        _: ChainHead,
        _: &SidecarReceipt,
    ) -> HeadCommit {
        panic!("guard denial must stop before head commit")
    }
}
impl ExactPublicationAuthority for GuardedHost {
    fn permits_exact(
        &mut self,
        _: &ConsentGrant,
        _: &SensingRequest,
        _: &Path,
        _: &crate::BridgedObservation,
        _: &SidecarReceipt,
        _: &[u8; 768],
        _: &[u8],
    ) -> bool {
        self.exact_checks += 1;
        self.permit_first && self.exact_checks == 1
    }
}

#[test]
fn exact_guard_denial_before_and_after_admission_preserves_quota_semantics() {
    for permit_first in [false, true] {
        let root = std::env::temp_dir();
        let grant = signed_grant();
        let bridged = bridge_observation(&Authority, 21, 17, observation(), binding()).unwrap();
        let permit = permit(&grant, SensingAction::Store, 17);
        let request = permit.request();
        let mut sidecar = [0; SIDECAR_SIZE];
        let mut encoded = [0; SIDECAR_RECEIPT_BYTES];
        let receipt = prepare_store_receipt(
            &StoreAuthority(true),
            permit,
            &grant,
            &Identity,
            &SensorScope(true),
            &Revocations(false),
            &mut Clock(60),
            &bridged,
            &[5; 16],
            16,
            [1; 32],
            0,
            None,
            &SigningKey::from_bytes(&[11; 32]),
            &mut sidecar,
            &mut encoded,
        )
        .unwrap();
        let mut quota = CustodyQuota::<1>::from_authenticated_usage(
            &root,
            QuotaLimits {
                encoded_bytes: 4096,
                entries: 4,
            },
            QuotaSnapshot {
                id: [1; 32],
                encoded_bytes: 0,
                entries: 0,
            },
            &Snapshot,
        )
        .unwrap();
        let mut clock = Clock(60);
        let mut consent = StoreConsent {
            grant: &grant,
            identity: &Identity,
            scope: &SensorScope(true),
            revocations: &Revocations(false),
            clock: &mut clock,
        };
        let mut host = GuardedHost {
            exact_checks: 0,
            permit_first,
        };
        let result = publish_prepared_store_with_optional_quota(
            &root,
            &bridged,
            &[5; 16],
            2048,
            request,
            &mut consent,
            &mut host,
            &sidecar,
            receipt,
            Some(&mut quota),
        );
        if permit_first {
            match result {
                Err(PublishStoreError::Quota(QuotaPublicationError::AttemptCharged {
                    checkpoint,
                    bytes,
                    error: ChainPublicationError::Publication(CustodyError::PermissionDenied),
                })) => {
                    assert_eq!(checkpoint, receipt.checkpoint());
                    // Exact encoded QCB1 header + sidecar + receipt + payload.
                    assert_eq!(bytes, 16 + SIDECAR_SIZE + SIDECAR_RECEIPT_BYTES + 16);
                    assert_eq!(quota.charged_usage(), (bytes as u64, 1));
                }
                _ => panic!("expected charged exact-guard failure"),
            }
            assert_eq!(host.exact_checks, 2);
        } else {
            assert!(matches!(
                result,
                Err(PublishStoreError::Quota(QuotaPublicationError::Preflight(
                    CustodyError::PermissionDenied
                )))
            ));
            assert_eq!(quota.charged_usage(), (0, 0));
            assert_eq!(host.exact_checks, 1);
        }
    }
}
