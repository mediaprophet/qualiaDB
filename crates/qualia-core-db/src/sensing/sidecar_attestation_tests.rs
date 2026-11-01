use super::*;

struct Authority(bool);
impl SidecarReceiptAuthority for Authority {
    fn permits(&self, _: &SidecarReceipt, _: &[u8; SIDECAR_BYTES], _: &[u8]) -> bool {
        self.0
    }
}

#[test]
fn weak_ed25519_public_key_is_not_accepted() {
    let sidecar = [9; SIDECAR_BYTES];
    let key = SigningKey::from_bytes(&[3; 32]);
    let mut receipt =
        SidecarReceipt::sign(&sidecar, b"data", 4, [0; 32], [1; 32], 0, None, &key).unwrap();
    receipt.signer = [0; 32];
    receipt.signer[0] = 1;
    receipt.signature = [0; 64];
    receipt.signature[0] = 1;
    assert!(matches!(
        receipt.verify_signature(&sidecar, b"data", 4),
        Err(SidecarReceiptError::InvalidSigner | SidecarReceiptError::InvalidSignature)
    ));
}

#[test]
fn receipt_roundtrip_and_authority_are_separate() {
    let sidecar = [9; SIDECAR_BYTES];
    let key = SigningKey::from_bytes(&[3; 32]);
    let receipt =
        SidecarReceipt::sign(&sidecar, b"data", 4, [0; 32], [1; 32], 0, None, &key).unwrap();
    let mut out = [0; SIDECAR_RECEIPT_BYTES];
    assert_eq!(receipt.encode(&mut out), Ok(SIDECAR_RECEIPT_BYTES));
    let parsed = SidecarReceipt::decode(&out).unwrap();
    assert_eq!(parsed, receipt);
    assert_eq!(parsed.verify_signature(&sidecar, b"data", 4), Ok(()));
    assert_eq!(
        parsed.verify_with_authority(&sidecar, b"data", 4, &Authority(false)),
        Err(SidecarReceiptError::NotAuthorized)
    );
    assert_eq!(
        parsed.verify_with_authority(&sidecar, b"data", 4, &Authority(true)),
        Ok(())
    );
}

#[test]
fn content_and_chain_mutations_fail() {
    let sidecar = [9; SIDECAR_BYTES];
    let key = SigningKey::from_bytes(&[3; 32]);
    let first =
        SidecarReceipt::sign(&sidecar, b"data", 4, [7; 32], [1; 32], 0, None, &key).unwrap();
    let next = SidecarReceipt::sign(
        &sidecar,
        b"data",
        4,
        [7; 32],
        [1; 32],
        1,
        Some(first.checkpoint()),
        &key,
    )
    .unwrap();
    assert_eq!(
        next.verify_signature(&sidecar, b"dAta", 4),
        Err(SidecarReceiptError::PayloadMismatch)
    );
    let mut changed = sidecar;
    changed[300] ^= 1;
    assert_eq!(
        next.verify_signature(&changed, b"data", 4),
        Err(SidecarReceiptError::SidecarMismatch)
    );
    let mut wire = [0; SIDECAR_RECEIPT_BYTES];
    next.encode(&mut wire).unwrap();
    wire[168] ^= 1;
    assert_eq!(
        SidecarReceipt::decode(&wire)
            .unwrap()
            .verify_signature(&sidecar, b"data", 4),
        Err(SidecarReceiptError::InvalidSignature)
    );
}

#[test]
fn fails_closed_on_budget_and_invalid_chain() {
    let sidecar = [0; SIDECAR_BYTES];
    let key = SigningKey::from_bytes(&[3; 32]);
    assert_eq!(
        SidecarReceipt::sign(&sidecar, b"data", 3, [0; 32], [1; 32], 0, None, &key),
        Err(SidecarReceiptError::PayloadBudgetExceeded)
    );
    assert_eq!(
        SidecarReceipt::sign(&sidecar, b"", 0, [0; 32], [1; 32], 1, None, &key),
        Err(SidecarReceiptError::InvalidChain)
    );
}
