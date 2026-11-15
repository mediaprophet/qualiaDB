use super::*;
use ed25519_dalek::SigningKey;

struct Authority {
    publish: bool,
    read: bool,
    signature: bool,
}
impl SidecarReceiptAuthority for Authority {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        self.signature
    }
}
impl CustodyAuthority for Authority {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
        _: usize,
    ) -> bool {
        self.publish
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        self.read
    }
}

fn receipt(sidecar: &[u8; SIDECAR_BYTES], payload: &[u8]) -> SidecarReceipt {
    SidecarReceipt::sign(
        sidecar,
        payload,
        payload.len(),
        [1; 32],
        [2; 32],
        0,
        None,
        &SigningKey::from_bytes(&[3; 32]),
    )
    .unwrap()
}

#[test]
fn bundle_roundtrip_and_duplicate_create_fail_closed() {
    let dir = tempfile::tempdir().unwrap();
    let sidecar = [9; SIDECAR_BYTES];
    let payload = b"abc";
    let receipt = receipt(&sidecar, payload);
    let mut authority = Authority {
        publish: true,
        read: true,
        signature: true,
    };
    let published = publish_bundle(
        dir.path(),
        &sidecar,
        payload,
        &receipt,
        FIXED_BYTES + payload.len(),
        &mut authority,
    )
    .unwrap();
    assert!(publish_bundle(
        dir.path(),
        &sidecar,
        payload,
        &receipt,
        FIXED_BYTES + payload.len(),
        &mut authority
    )
    .is_err());
    let mut read_sidecar = [0; SIDECAR_BYTES];
    let mut read_payload = [0; 8];
    let result = read_bundle(
        dir.path(),
        &published.checkpoint,
        FIXED_BYTES + payload.len(),
        &mut read_sidecar,
        &mut read_payload,
        &mut authority,
    )
    .unwrap();
    assert_eq!(result.payload_bytes, payload.len());
    assert_eq!(read_sidecar, sidecar);
    assert_eq!(&read_payload[..payload.len()], payload);
}

#[test]
fn denied_and_over_budget_operations_fail_before_publication() {
    let dir = tempfile::tempdir().unwrap();
    let sidecar = [0; SIDECAR_BYTES];
    let receipt = receipt(&sidecar, b"x");
    let mut authority = Authority {
        publish: false,
        read: false,
        signature: true,
    };
    assert!(matches!(
        publish_bundle(
            dir.path(),
            &sidecar,
            b"x",
            &receipt,
            FIXED_BYTES + 1,
            &mut authority
        ),
        Err(CustodyError::PermissionDenied)
    ));
    assert!(matches!(
        publish_bundle(
            dir.path(),
            &sidecar,
            b"x",
            &receipt,
            FIXED_BYTES,
            &mut authority
        ),
        Err(CustodyError::TooLarge)
    ));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    let mut bytes = [0xff; SIDECAR_BYTES];
    let mut payload = [0xff; 4];
    assert!(matches!(
        read_bundle(
            dir.path(),
            &[1; 32],
            FIXED_BYTES + 4,
            &mut bytes,
            &mut payload,
            &mut authority
        ),
        Err(CustodyError::PermissionDenied)
    ));
    assert_eq!(bytes, [0; SIDECAR_BYTES]);
    assert_eq!(payload, [0; 4]);
}

struct ReadRaceAuthority {
    read_calls: usize,
    panic_on_verify: bool,
}

#[test]
fn root_with_parent_components_is_rejected() {
    let parent = std::env::temp_dir().join("qualia-parent").join("..");
    assert!(matches!(
        validate_root(&parent),
        Err(CustodyError::InvalidRoot)
    ));
}

impl SidecarReceiptAuthority for ReadRaceAuthority {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        if self.panic_on_verify {
            panic!("authority failure during verification");
        }
        true
    }
}

impl CustodyAuthority for ReadRaceAuthority {
    fn permit_publish(
        &mut self,
        _: &Path,
        _: &SidecarReceipt,
        _: &[u8; SIDECAR_BYTES],
        _: &[u8],
        _: usize,
    ) -> bool {
        true
    }
    fn permit_read(&mut self, _: &Path, _: &[u8; 32], _: usize) -> bool {
        self.read_calls += 1;
        self.read_calls == 1
    }
}

#[test]
fn read_revoked_at_final_recheck_clears_both_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let sidecar = [9; SIDECAR_BYTES];
    let payload = b"private";
    let receipt = receipt(&sidecar, payload);
    let mut authority = ReadRaceAuthority {
        read_calls: 0,
        panic_on_verify: false,
    };
    let published = publish_bundle(
        dir.path(),
        &sidecar,
        payload,
        &receipt,
        FIXED_BYTES + payload.len(),
        &mut authority,
    )
    .unwrap();
    let mut read_sidecar = [0xff; SIDECAR_BYTES];
    let mut read_payload = [0xff; 16];
    let result = read_bundle(
        dir.path(),
        &published.checkpoint,
        FIXED_BYTES + payload.len(),
        &mut read_sidecar,
        &mut read_payload,
        &mut authority,
    );
    assert!(matches!(result, Err(CustodyError::PermissionDenied)));
    assert_eq!(authority.read_calls, 2);
    assert_eq!(read_sidecar, [0; SIDECAR_BYTES]);
    assert_eq!(read_payload, [0; 16]);
}

#[test]
fn read_authority_unwind_clears_both_outputs() {
    let dir = tempfile::tempdir().unwrap();
    let sidecar = [9; SIDECAR_BYTES];
    let payload = b"private";
    let receipt = receipt(&sidecar, payload);
    let mut authority = ReadRaceAuthority {
        read_calls: 0,
        panic_on_verify: false,
    };
    let published = publish_bundle(
        dir.path(),
        &sidecar,
        payload,
        &receipt,
        FIXED_BYTES + payload.len(),
        &mut authority,
    )
    .unwrap();
    authority.panic_on_verify = true;
    let mut read_sidecar = [0xff; SIDECAR_BYTES];
    let mut read_payload = [0xff; 16];
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        read_bundle(
            dir.path(),
            &published.checkpoint,
            FIXED_BYTES + payload.len(),
            &mut read_sidecar,
            &mut read_payload,
            &mut authority,
        )
    }));
    assert!(result.is_err());
    assert_eq!(read_sidecar, [0; SIDECAR_BYTES]);
    assert_eq!(read_payload, [0; 16]);
}
