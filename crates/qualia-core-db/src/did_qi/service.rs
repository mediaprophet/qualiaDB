//! CSCP mailbox service entries (spec §10).
//!
//! Under relay-only disclosure, Direct locators and `publicDht` are forbidden.

use super::QiError;

pub const SERVICE_TYPE: &str = "CscpMailbox";
pub const MAX_HINTS: usize = 2;
pub const MAX_HINT_ID: usize = 32;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocatorClass {
    Mailbox = 1,
    RelayHint = 2,
    Direct = 3,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Disclosure {
    DirectPermitted = 1,
    ApprovedRelaysOnly = 2,
    QualifiedMultiHop = 3,
    Isolated = 4,
}

impl Disclosure {
    pub const fn is_relay_only(self) -> bool {
        !matches!(self, Disclosure::DirectPermitted)
    }

    pub const fn json_name(self) -> &'static [u8] {
        match self {
            Disclosure::DirectPermitted => b"DirectPermitted",
            Disclosure::ApprovedRelaysOnly => b"ApprovedRelaysOnly",
            Disclosure::QualifiedMultiHop => b"QualifiedMultiHop",
            Disclosure::Isolated => b"Isolated",
        }
    }
}

impl LocatorClass {
    pub const fn json_name(self) -> &'static [u8] {
        match self {
            LocatorClass::Mailbox => b"mailbox",
            LocatorClass::RelayHint => b"relayHint",
            LocatorClass::Direct => b"direct",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelayHint {
    pub hint_id: [u8; MAX_HINT_ID],
    pub hint_id_len: u8,
    pub operator_hash: [u8; 32],
}

impl RelayHint {
    pub fn new(hint_id: &[u8], operator_hash: [u8; 32]) -> Result<Self, QiError> {
        if hint_id.is_empty() || hint_id.len() > MAX_HINT_ID {
            return Err(QiError::MalformedDocument);
        }
        let mut h = Self {
            hint_id: [0u8; MAX_HINT_ID],
            hint_id_len: hint_id.len() as u8,
            operator_hash,
        };
        h.hint_id[..hint_id.len()].copy_from_slice(hint_id);
        Ok(h)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CscpMailbox {
    pub kind: LocatorClass,
    pub disclosure: Disclosure,
    pub contact_key: [u8; 32],
    pub public_dht: bool,
    pub generation: u32,
    pub hint_count: u8,
    pub hints: [RelayHint; MAX_HINTS],
}

impl CscpMailbox {
    pub const fn mailbox(contact_key: [u8; 32], disclosure: Disclosure) -> Self {
        Self {
            kind: LocatorClass::Mailbox,
            disclosure,
            contact_key,
            public_dht: false,
            generation: 0,
            hint_count: 0,
            hints: [RelayHint {
                hint_id: [0u8; MAX_HINT_ID],
                hint_id_len: 0,
                operator_hash: [0u8; 32],
            }; MAX_HINTS],
        }
    }

    pub const fn direct(contact_key: [u8; 32], disclosure: Disclosure) -> Self {
        let mut s = Self::mailbox(contact_key, disclosure);
        s.kind = LocatorClass::Direct;
        s
    }

    pub const fn is_direct(&self) -> bool {
        matches!(self.kind, LocatorClass::Direct)
    }
}

/// Spec §10.3: relay-only documents MUST NOT carry Direct locators or public DHT.
pub fn check_relay_only(services: &[CscpMailbox]) -> Result<(), QiError> {
    let mut relay_only = false;
    let mut i = 0;
    while i < services.len() {
        if services[i].disclosure.is_relay_only() {
            relay_only = true;
        }
        i += 1;
    }
    if !relay_only {
        return Ok(());
    }
    i = 0;
    while i < services.len() {
        if services[i].is_direct() || services[i].public_dht {
            return Err(QiError::DirectLocatorForbidden);
        }
        i += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_only_document_with_direct_locator_rejected() {
        let services = [CscpMailbox::direct(
            [9u8; 32],
            Disclosure::ApprovedRelaysOnly,
        )];
        assert_eq!(
            check_relay_only(&services),
            Err(QiError::DirectLocatorForbidden)
        );
        let mailbox = [CscpMailbox::mailbox(
            [9u8; 32],
            Disclosure::ApprovedRelaysOnly,
        )];
        assert_eq!(check_relay_only(&mailbox), Ok(()));
        let permitted_direct = [CscpMailbox::direct(
            [9u8; 32],
            Disclosure::DirectPermitted,
        )];
        assert_eq!(check_relay_only(&permitted_direct), Ok(()));
    }

    #[test]
    fn service_type_is_cscp_mailbox() {
        assert_eq!(SERVICE_TYPE, "CscpMailbox");
        assert_eq!(LocatorClass::Direct.json_name(), b"direct");
        assert_eq!(Disclosure::ApprovedRelaysOnly.json_name(), b"ApprovedRelaysOnly");
    }
}
