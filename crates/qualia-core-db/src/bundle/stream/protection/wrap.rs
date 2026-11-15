//! Grant-gated metadata-master wrap. Provider policy is not disclosure authority.
use crate::bundle::stream::policy::remote::{
    ControllerRecognition, DisclosureAction, RemoteDisclosureGrant,
};
use crate::bundle::stream::policy::{BoundClosure, LocalWholeSourceGrant, PolicyError};
use crate::crypto::network::{
    authority::{AuthorityReceipt, AuthoritySource, PinAuthority, PinHandle},
    cose,
    errors::CryptoError,
    object_stream::{RecipientRecord, RECIPIENT_PURPOSE, WRAP_BYTES},
};
use crate::net::qdnf::{
    authority::{AuthorityBinding, AuthorityOwner, CurrentAuthorityGuard},
    errors::QdnfError,
};
use crate::specialized_libs::cryptographic_library::ManagedObjectPublication;
use std::sync::Mutex;
use zeroize::Zeroize;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WrapError {
    Unauthorized,
    Closed,
    Spent,
    Capacity,
}

/// Recognized issuer pin for recipient-authority. Audience is taken from A.
pub struct RecipientAuthorityProof<'a> {
    pub pin: PinHandle,
    pub key_ids: cose::KeyIds,
    pub epoch: u64,
    pub envelope: &'a [u8],
}

/// Rechecks the current whole-source grant and recognized recipient proof, then
/// wraps. Success is a candidate envelope, not durable publication or plaintext.
pub fn authorize_and_wrap(
    publication: &mut ManagedObjectPublication<'_>,
    grant: &mut LocalWholeSourceGrant<'_, '_>,
    pins: &PinAuthority,
    clock: impl Fn() -> u64,
    record: RecipientRecord<'_>,
    proof: RecipientAuthorityProof<'_>,
    workspace: &mut cose::Workspace,
    out: &mut [u8],
) -> Result<usize, WrapError> {
    struct Wipe<'a> {
        out: &'a mut [u8],
        keep: bool,
    }
    impl Drop for Wipe<'_> {
        fn drop(&mut self) {
            if !self.keep {
                self.out.zeroize();
            }
        }
    }
    let mut wipe = Wipe { out, keep: false };
    if wipe.out.len() < WRAP_BYTES {
        return Err(WrapError::Capacity);
    }
    let expected = grant.binding();
    let n = grant
        .accept_local_scoped(
            pins,
            || clock(),
            |gate| {
                gate.recheck()?;
                match_grant(record, expected, gate.closure())?;
                let now = clock();
                record.valid_at(now)?;
                let auth = record.id();
                let binding = cose::Binding {
                    descriptor_digest: &auth.0,
                    key_ids: proof.key_ids,
                    controller_epoch: proof.epoch,
                    purpose: RECIPIENT_PURPOSE,
                    audience: &record.fields().publisher.0,
                };
                let receipt = pins.verify_source(
                    AuthoritySource::LocalPin(proof.pin),
                    proof.envelope,
                    &binding,
                    now,
                    workspace,
                )?;
                pins.recheck(&receipt, &auth.0, now)?;
                publication
                    .wrap_for(record, wipe.out)
                    .map_err(PolicyError::from)
            },
        )
        .map_err(map_policy)?;
    wipe.keep = true;
    Ok(n)
}

/// Rechecks a live remote disclosure grant (MetadataRelease) and recognized
/// recipient proof, then wraps. A locally trusted pin with the right
/// purpose/audience but no rights over this source refuses Unauthorized.
#[allow(clippy::too_many_arguments)]
pub fn authorize_and_wrap_remote(
    publication: &mut ManagedObjectPublication<'_>,
    grant: &RemoteDisclosureGrant<'_>,
    guard: &CurrentAuthorityGuard<'_>,
    owner: &Mutex<AuthorityOwner>,
    pins: &PinAuthority,
    recognition: &ControllerRecognition,
    receipt: &AuthorityReceipt,
    clock: impl Fn() -> u64,
    record: RecipientRecord<'_>,
    proof: RecipientAuthorityProof<'_>,
    audience: &[u8],
    workspace: &mut cose::Workspace,
    out: &mut [u8],
) -> Result<usize, WrapError> {
    struct Wipe<'a> {
        out: &'a mut [u8],
        keep: bool,
    }
    impl Drop for Wipe<'_> {
        fn drop(&mut self) {
            if !self.keep {
                self.out.zeroize();
            }
        }
    }
    let mut wipe = Wipe { out, keep: false };
    if wipe.out.len() < WRAP_BYTES {
        return Err(WrapError::Capacity);
    }
    let now = clock();
    grant
        .recheck(
            guard,
            owner,
            pins,
            recognition,
            receipt,
            now,
            DisclosureAction::MetadataRelease,
        )
        .map_err(map_policy)?;
    match_remote_grant(record, grant)?;
    record.valid_at(now).map_err(|e| map_policy(PolicyError::from(e)))?;
    let auth = record.id();
    let binding = cose::Binding {
        descriptor_digest: &auth.0,
        key_ids: proof.key_ids,
        controller_epoch: proof.epoch,
        purpose: RECIPIENT_PURPOSE,
        audience,
    };
    let verified = pins
        .verify_source(
            AuthoritySource::LocalPin(proof.pin),
            proof.envelope,
            &binding,
            now,
            workspace,
        )
        .map_err(|e| map_policy(PolicyError::from(e)))?;
    pins.recheck(&verified, &auth.0, now)
        .map_err(|e| map_policy(PolicyError::from(e)))?;
    let n = publication
        .wrap_for(record, wipe.out)
        .map_err(|e| map_policy(PolicyError::from(e)))?;
    wipe.keep = true;
    Ok(n)
}

fn match_remote_grant(
    record: RecipientRecord<'_>,
    grant: &RemoteDisclosureGrant<'_>,
) -> Result<(), WrapError> {
    let f = record.fields();
    let b = grant.binding();
    let r = grant.recognized();
    if f.recipient != b.recipient
        || f.purpose != b.purpose
        || f.operation != b.operation.0
        || f.instrument != b.instrument
        || f.scope != b.scope
        || f.source != r.source()
        || f.source_length != r.source_length()
        || f.policy != r.policy()
        || f.publisher != r.publisher()
        || f.purpose != r.purpose()
        || f.operation != r.operation().0
    {
        Err(WrapError::Unauthorized)
    } else {
        Ok(())
    }
}

fn match_grant(
    record: RecipientRecord<'_>,
    expected: AuthorityBinding,
    closure: &BoundClosure<'_>,
) -> Result<(), PolicyError> {
    let f = record.fields();
    let m = closure.closure().manifest().fields();
    if f.recipient != expected.recipient
        || f.publisher != expected.principal
        || f.purpose != expected.purpose
        || f.operation != expected.operation.0
        || f.instrument != expected.instrument
        || f.scope != expected.scope
        || f.closure != closure.closure().manifest_digest()
        || f.source != m.source
        || f.source_length != m.source_length
        || f.policy != m.policy
        || f.consumer != m.consumer
    {
        Err(PolicyError::Crypto(CryptoError::Unauthorized))
    } else {
        Ok(())
    }
}

fn map_policy(e: PolicyError) -> WrapError {
    match e {
        PolicyError::Spent => WrapError::Spent,
        PolicyError::Capacity => WrapError::Capacity,
        PolicyError::Crypto(CryptoError::Closed) | PolicyError::Authority(QdnfError::Closed) => {
            WrapError::Closed
        }
        PolicyError::Crypto(CryptoError::Capacity)
        | PolicyError::Authority(QdnfError::Capacity) => WrapError::Capacity,
        _ => WrapError::Unauthorized,
    }
}
