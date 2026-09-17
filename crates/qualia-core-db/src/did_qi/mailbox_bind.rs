//! Publish Qualia Identifier `CscpMailbox` services into a CSCP private mailbox.
//!
//! Invitation-scoped only. Deactivated documents publish nothing. Direct
//! locators are not stored when the service disclosure forbids probing.
//! HostnameAlias is not a CSCP locator and is ignored here.

use crate::net::peer::connectivity::policy::Disclosure as FabricDisclosure;
use crate::net::peer::fabric::contact::{ContactDescriptor, LocatorKind};
use crate::net::peer::fabric::kernel::FabricError;
use crate::net::peer::fabric::mailbox::PrivateMailbox;

use super::document::QiDocument;
use super::service::{CscpMailbox, Disclosure as QiDisclosure, LocatorClass};

fn fabric_disclosure(d: QiDisclosure) -> FabricDisclosure {
    match d {
        QiDisclosure::DirectPermitted => FabricDisclosure::DirectPermitted,
        QiDisclosure::ApprovedRelaysOnly => FabricDisclosure::ApprovedRelaysOnly,
        QiDisclosure::QualifiedMultiHop => FabricDisclosure::QualifiedMultiHop,
        QiDisclosure::Isolated => FabricDisclosure::Isolated,
    }
}

fn locator_kind(kind: LocatorClass) -> LocatorKind {
    match kind {
        LocatorClass::Mailbox => LocatorKind::Mailbox,
        LocatorClass::RelayHint => LocatorKind::RelayHint,
        LocatorClass::Direct => LocatorKind::Direct,
    }
}

fn descriptor_from_service(s: &CscpMailbox, expiry_unix: u32) -> ContactDescriptor {
    let mut locator = [0u8; 16];
    if s.kind == LocatorClass::RelayHint && s.hint_count > 0 {
        locator.copy_from_slice(&s.hints[0].operator_hash[..16]);
    }
    ContactDescriptor {
        contact_key: s.contact_key,
        generation: s.generation,
        expiry_unix,
        network_generation: 1,
        kind: locator_kind(s.kind),
        locator,
    }
}

/// Publish each `CscpMailbox` service from `doc` into `mailbox`.
///
/// Returns the number of descriptors stored. `qi.deactivated` yields `0`.
pub fn publish_qi_document(
    mailbox: &mut PrivateMailbox,
    doc: &QiDocument,
    now_unix: u32,
    expiry_unix: u32,
) -> Result<usize, FabricError> {
    if doc.deactivated {
        return Ok(0);
    }
    if expiry_unix <= now_unix {
        return Err(FabricError::Expired);
    }
    let mut n = 0usize;
    let mut i = 0usize;
    while i < doc.service_count as usize {
        let s = &doc.services[i];
        let disclosure = fabric_disclosure(s.disclosure);
        if s.public_dht && disclosure != FabricDisclosure::DirectPermitted {
            return Err(FabricError::PolicyDenied);
        }
        let d = descriptor_from_service(s, expiry_unix);
        mailbox.publish_under(d, disclosure)?;
        n += 1;
        i += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::did_qi::git_object::GitObjectStore;
    use crate::did_qi::method::{create, deactivate};
    use crate::did_qi::service::{CscpMailbox, Disclosure, RelayHint};
    use crate::did_qi::sha256_32;
    use crate::did_qi::QiDocument;

    fn sk() -> [u8; 32] {
        [3u8; 32]
    }

    #[test]
    fn live_mailbox_publishes_contact_key() {
        let mut store = GitObjectStore::new();
        let mut doc = QiDocument::empty();
        let key = sha256_32(b"did:qi:bind:contact");
        doc.services[0] = CscpMailbox::mailbox(key, Disclosure::ApprovedRelaysOnly);
        doc.services[0].hints[0] =
            RelayHint::new(b"invite-1", sha256_32(b"did:qi:bind:operator")).unwrap();
        doc.services[0].hint_count = 1;
        doc.service_count = 1;
        create(&mut store, &sk(), &doc).unwrap();
        let mut mailbox = PrivateMailbox::new();
        let n = publish_qi_document(&mut mailbox, &doc, 10, 2_000_000_000).unwrap();
        assert_eq!(n, 1);
        let got = mailbox
            .resolve(&key, 10, FabricDisclosure::ApprovedRelaysOnly)
            .unwrap();
        assert_eq!(got.contact_key, key);
        assert_eq!(got.kind, LocatorKind::Mailbox);
        assert_eq!(got.locator, [0u8; 16]);
    }

    #[test]
    fn deactivated_document_publishes_nothing() {
        let mut store = GitObjectStore::new();
        let mut doc = QiDocument::empty();
        let key = [9u8; 32];
        doc.services[0] = CscpMailbox::mailbox(key, Disclosure::ApprovedRelaysOnly);
        doc.service_count = 1;
        let id = create(&mut store, &sk(), &doc).unwrap();
        deactivate(&mut store, &sk(), &id).unwrap();
        let mut tomb = QiDocument::empty();
        crate::did_qi::read(&store, &id, &mut tomb).unwrap();
        let mut mailbox = PrivateMailbox::new();
        assert_eq!(
            publish_qi_document(&mut mailbox, &tomb, 10, 2_000_000_000).unwrap(),
            0
        );
        assert_eq!(
            mailbox.resolve(&key, 10, FabricDisclosure::ApprovedRelaysOnly),
            Err(FabricError::Illegal)
        );
    }

    #[test]
    fn relay_only_direct_service_is_not_stored() {
        let mut doc = QiDocument::empty();
        let key = [4u8; 32];
        doc.services[0] = CscpMailbox::direct(key, Disclosure::ApprovedRelaysOnly);
        doc.service_count = 1;
        let mut mailbox = PrivateMailbox::new();
        assert_eq!(
            publish_qi_document(&mut mailbox, &doc, 10, 2_000_000_000),
            Err(FabricError::PolicyDenied)
        );
        assert_eq!(
            mailbox.resolve(&key, 10, FabricDisclosure::DirectPermitted),
            Err(FabricError::Illegal)
        );
    }

    #[test]
    fn hostname_alias_is_not_a_cscp_locator() {
        let mut doc = QiDocument::empty();
        doc.has_hostname_alias = true;
        doc.hostname_did_web =
            crate::did_qi::AkaEntry::from_slice(b"did:web:example.invalid").unwrap();
        let mut mailbox = PrivateMailbox::new();
        assert_eq!(
            publish_qi_document(&mut mailbox, &doc, 10, 2_000_000_000).unwrap(),
            0
        );
    }
}
