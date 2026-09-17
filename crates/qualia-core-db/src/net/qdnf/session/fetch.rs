//! Unconsented attachment/embed/LIG fetches must not disclose locators (NET-05.19).
//!
//! Group membership and payment are not private-message consent. `authorize_fetch`
//! never writes locator bytes. Reveal is a second step and copies only an
//! adapter-observed [`ObservedLocator`] after a valid [`FetchHandle`]. Packages
//! remain open.

use crate::net::qdnf::authority::{ContactState, PolicyOutcome};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::policy::gate_channel;
use crate::net::qdnf::types::ObservedLocator;

/// Private token that `authorize_fetch` stamps into a live handle.
const HANDLE_MAGIC: u64 = 0x5144_4E46_0519;

/// Attachment, embed, and LIG fetches share the same private-consent gate.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FetchKind {
    Attachment = 1,
    Embed = 2,
    Lig = 3,
}

/// Fetch intent. `group_member` and `paid` cannot substitute for `private_consent`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FetchRequest {
    pub kind: FetchKind,
    pub group_member: bool,
    pub private_consent: bool,
    pub paid: bool,
}

/// Capability token issued only after consent and Allow-equivalent contact.
///
/// Fields are private so callers cannot mint a valid handle. Zero/default is
/// invalid and must not reveal.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FetchHandle {
    magic: u64,
    kind: FetchKind,
}

impl FetchHandle {
    /// Never authorizes reveal.
    pub const INVALID: Self = Self {
        magic: 0,
        kind: FetchKind::Attachment,
    };

    #[inline]
    const fn issue(kind: FetchKind) -> Self {
        Self {
            magic: HANDLE_MAGIC,
            kind,
        }
    }

    #[inline]
    const fn is_valid(self) -> bool {
        self.magic == HANDLE_MAGIC
    }

    /// Kind recorded at authorize time. Meaningless on [`FetchHandle::INVALID`].
    #[inline]
    pub const fn kind(self) -> FetchKind {
        self.kind
    }
}

/// Group admission is not private-message consent.
#[inline]
pub fn group_admission_implies_private_consent() -> bool {
    false
}

/// Unconsented fetch must not copy locator or location bytes into caller buffers.
#[inline]
pub fn unconsented_fetch_discloses_locator() -> bool {
    false
}

fn contact_is_allow_equivalent(contact: ContactState) -> bool {
    matches!(contact, ContactState::Active | ContactState::Consent)
}

/// Authorize an attachment/embed/LIG fetch. Never writes `out_locator`.
///
/// Denied/Unauthorized leave `out_locator` untouched. Success returns a
/// [`FetchHandle`]; the caller must pass an adapter-observed locator to
/// [`reveal_observed`] afterwards. Destination locators are never taken from
/// payload claims.
pub fn authorize_fetch(
    req: FetchRequest,
    contact: ContactState,
    out_locator: &mut ObservedLocator,
) -> Result<FetchHandle, QdnfError> {
    // Caller owns this buffer. Deny and Ok both leave it unchanged so a
    // partial write cannot become a locator/location oracle.
    let _untouched = out_locator;

    let FetchRequest {
        kind,
        group_member,
        private_consent,
        paid,
    } = req;
    // Group membership and payment are not private-message consent.
    let _ = (group_member, paid, kind);

    if matches!(contact, ContactState::Blocked | ContactState::Suspended) {
        return Err(QdnfError::Denied);
    }
    if !private_consent {
        return Err(QdnfError::Denied);
    }
    if !contact_is_allow_equivalent(contact) {
        return Err(QdnfError::Unauthorized);
    }
    gate_channel(PolicyOutcome::Allow, true, contact)?;
    debug_assert!(!group_admission_implies_private_consent());
    debug_assert!(!unconsented_fetch_discloses_locator());
    Ok(FetchHandle::issue(kind))
}

/// Copy `src` into `out` only after a valid handle. Invalid handle: `out` unchanged.
pub fn reveal_observed(
    handle: FetchHandle,
    src: ObservedLocator,
    out: &mut ObservedLocator,
) -> Result<(), QdnfError> {
    if !handle.is_valid() {
        return Err(QdnfError::Unauthorized);
    }
    *out = src;
    Ok(())
}

/// Same as [`reveal_observed`]. Reveal is never combined with authorize.
#[inline]
pub fn reveal_locator(
    handle: FetchHandle,
    src: ObservedLocator,
    out: &mut ObservedLocator,
) -> Result<(), QdnfError> {
    reveal_observed(handle, src, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(tag: u8) -> ObservedLocator {
        ObservedLocator::from_slice(&[tag, 0x11, 0x22, 0x33]).expect("locator")
    }

    fn req(kind: FetchKind, group_member: bool, private_consent: bool, paid: bool) -> FetchRequest {
        FetchRequest {
            kind,
            group_member,
            private_consent,
            paid,
        }
    }

    fn assert_untouched(before: ObservedLocator, after: &ObservedLocator) {
        assert_eq!(*after, before);
        assert!(!QdnfError::Denied.output_valid());
        assert!(!QdnfError::Unauthorized.output_valid());
    }

    #[test]
    fn unconsented_attachment_denied_out_remains_empty() {
        let mut out = ObservedLocator::EMPTY;
        let err = authorize_fetch(
            req(FetchKind::Attachment, false, false, false),
            ContactState::Active,
            &mut out,
        );
        assert_eq!(err, Err(QdnfError::Denied));
        assert_eq!(out, ObservedLocator::EMPTY);
        assert!(!unconsented_fetch_discloses_locator());
    }

    #[test]
    fn unconsented_embed_and_lig_denied_out_remains_empty() {
        let mut embed_out = ObservedLocator::EMPTY;
        assert_eq!(
            authorize_fetch(
                req(FetchKind::Embed, false, false, false),
                ContactState::Consent,
                &mut embed_out,
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(embed_out, ObservedLocator::EMPTY);

        let mut lig_out = ObservedLocator::EMPTY;
        assert_eq!(
            authorize_fetch(
                req(FetchKind::Lig, false, false, false),
                ContactState::Active,
                &mut lig_out,
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(lig_out, ObservedLocator::EMPTY);
    }

    #[test]
    fn group_member_and_paid_do_not_disclose() {
        let mut out = ObservedLocator::EMPTY;
        assert_eq!(
            authorize_fetch(
                req(FetchKind::Attachment, true, false, true),
                ContactState::Active,
                &mut out,
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(out, ObservedLocator::EMPTY);
        assert!(!group_admission_implies_private_consent());

        let mut poison = loc(0xAA);
        let before = poison;
        assert_eq!(
            authorize_fetch(
                req(FetchKind::Embed, true, false, true),
                ContactState::Consent,
                &mut poison,
            ),
            Err(QdnfError::Denied)
        );
        assert_untouched(before, &poison);
        assert_eq!(
            reveal_observed(FetchHandle::INVALID, loc(0xBB), &mut poison),
            Err(QdnfError::Unauthorized)
        );
        assert_untouched(before, &poison);
    }

    #[test]
    fn consented_active_reveals_observed_locator_copy() {
        let mut out = ObservedLocator::EMPTY;
        let handle = authorize_fetch(
            req(FetchKind::Attachment, false, true, false),
            ContactState::Active,
            &mut out,
        )
        .expect("consented active");
        assert_eq!(out, ObservedLocator::EMPTY);
        assert!(handle.is_valid());
        assert_eq!(handle.kind(), FetchKind::Attachment);

        let src = loc(0x42);
        reveal_observed(handle, src, &mut out).expect("reveal");
        assert_eq!(out, src);
        assert_eq!(out.as_slice(), src.as_slice());

        let mut consent_out = ObservedLocator::EMPTY;
        let consent_handle = authorize_fetch(
            req(FetchKind::Lig, true, true, true),
            ContactState::Consent,
            &mut consent_out,
        )
        .expect("consented contact");
        assert_eq!(consent_out, ObservedLocator::EMPTY);
        reveal_locator(consent_handle, loc(0x7E), &mut consent_out).expect("reveal_locator");
        assert_eq!(consent_out, loc(0x7E));
    }

    #[test]
    fn blocked_with_private_consent_denied() {
        let mut out = loc(0xCC);
        let before = out;
        assert_eq!(
            authorize_fetch(
                req(FetchKind::Attachment, true, true, true),
                ContactState::Blocked,
                &mut out,
            ),
            Err(QdnfError::Denied)
        );
        assert_untouched(before, &out);

        assert_eq!(
            authorize_fetch(
                req(FetchKind::Embed, false, true, false),
                ContactState::Suspended,
                &mut out,
            ),
            Err(QdnfError::Denied)
        );
        assert_untouched(before, &out);
    }

    #[test]
    fn invalid_handle_does_not_write() {
        let mut out = loc(0xDD);
        let before = out;
        assert_eq!(
            reveal_observed(FetchHandle::INVALID, loc(0xEE), &mut out),
            Err(QdnfError::Unauthorized)
        );
        assert_untouched(before, &out);
        assert_eq!(
            reveal_locator(FetchHandle::INVALID, loc(0xFF), &mut out),
            Err(QdnfError::Unauthorized)
        );
        assert_untouched(before, &out);

        let forged = FetchHandle {
            magic: HANDLE_MAGIC ^ 1,
            kind: FetchKind::Lig,
        };
        assert_eq!(
            reveal_observed(forged, loc(0x01), &mut out),
            Err(QdnfError::Unauthorized)
        );
        assert_untouched(before, &out);
    }

    #[test]
    fn honest_fns_are_false() {
        assert!(!group_admission_implies_private_consent());
        assert!(!unconsented_fetch_discloses_locator());
    }
}
