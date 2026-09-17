//! Independent wire-payload oracle. Does not call AEAD open.

use crate::net::qdnf::errors::QdnfError;

/// View of one recorded frame payload. Ciphertext must not equal application bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProtectedView<'a> {
    pub wire_payload: &'a [u8],
    pub application: &'a [u8],
}

/// True when the recorded payload contains the application bytes in the clear.
pub fn plaintext_on_wire(view: ProtectedView<'_>) -> bool {
    if view.application.is_empty() {
        return false;
    }
    if view.wire_payload.len() < view.application.len() {
        return false;
    }
    let mut i = 0;
    while i + view.application.len() <= view.wire_payload.len() {
        if &view.wire_payload[i..i + view.application.len()] == view.application {
            return true;
        }
        i += 1;
    }
    false
}

/// Fail closed when plaintext is visible. Used as a qualification gate.
pub fn require_protected(view: ProtectedView<'_>) -> Result<(), QdnfError> {
    if plaintext_on_wire(view) {
        Err(QdnfError::Denied)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_plaintext_frame_payload() {
        let app = b"hello-qpr";
        let view = ProtectedView {
            wire_payload: app,
            application: app,
        };
        assert!(plaintext_on_wire(view));
        assert_eq!(require_protected(view), Err(QdnfError::Denied));
    }

    #[test]
    fn accepts_payload_that_does_not_contain_application() {
        let view = ProtectedView {
            wire_payload: &[0x9a, 0x13, 0x44, 0x80, 0x02, 0x77, 0x11, 0xce, 0x5d],
            application: b"hello-qpr",
        };
        assert!(!plaintext_on_wire(view));
        assert!(require_protected(view).is_ok());
    }
}
