use super::{
    CLOSURE_PURPOSE, ManifestView, PUBLICATION_PURPOSE, PolicyError as E, PublicationProof,
    decode_manifest,
};
use crate::bundle::stream::descriptor::{
    CandidateIdentity, DigestDomain, decode_body, domain_digest,
};
use crate::crypto::network::{
    authority::{AuthorityReceipt, AuthoritySource, PinAuthority, PinHandle},
    cose,
};
use crate::net::qdnf::types::StrongDigest;
use crate::semantic_instruments::resolve::bounded_closure::ClosureCandidate;

/// Recognized paired issuer's exact declared graph, not semantic policy admission.
/// Receipt retains exact pin-owner incarnation; cached closure does not block revocation.
pub struct AuthenticatedClosure<'a> {
    candidate: ClosureCandidate<'a, 'a>,
    manifest: ManifestView<'a>,
    receipt: AuthorityReceipt,
    digest: StrongDigest,
}
/// Signed manifest plus a separately recognized paired publisher signature over
/// the exact descriptor (including entry roots), associated with a full candidate.
/// This does not prove payload download, plaintext disclosure rights or durability.
pub struct BoundClosure<'a> {
    closure: AuthenticatedClosure<'a>,
    identity: CandidateIdentity,
    publisher_receipt: AuthorityReceipt,
}
/// Expected key identifiers/epoch/audience are checked against the selected local
/// pin. No wire key imports or caller-provided verified/Allow booleans exist.
pub fn authenticate_closure<'a>(
    candidate: ClosureCandidate<'a, 'a>,
    manifest: &'a [u8],
    pins: &PinAuthority,
    pin: PinHandle,
    key_ids: cose::KeyIds,
    epoch: u64,
    audience: &[u8],
    envelope: &[u8],
    now: u64,
    workspace: &mut cose::Workspace,
) -> Result<AuthenticatedClosure<'a>, E> {
    let manifest = decode_manifest(manifest)?;
    manifest.matches(&candidate)?;
    let digest = manifest.digest();
    let binding = cose::Binding {
        descriptor_digest: &digest.0,
        key_ids,
        controller_epoch: epoch,
        purpose: CLOSURE_PURPOSE,
        audience,
    };
    let receipt = pins.verify_source(
        AuthoritySource::LocalPin(pin),
        envelope,
        &binding,
        now,
        workspace,
    )?;
    if receipt.principal() != manifest.fields().issuer {
        return Err(E::Reference);
    }
    Ok(AuthenticatedClosure {
        candidate,
        manifest,
        receipt,
        digest,
    })
}
impl<'a> AuthenticatedClosure<'a> {
    pub const fn manifest(&self) -> ManifestView<'a> {
        self.manifest
    }
    pub const fn manifest_digest(&self) -> StrongDigest {
        self.digest
    }
    pub fn candidate(&self) -> &ClosureCandidate<'a, 'a> {
        &self.candidate
    }
    pub fn recheck(&self, pins: &PinAuthority, now: u64) -> Result<(), E> {
        pins.recheck(&self.receipt, &self.digest.0, now)
            .map_err(E::from)
    }
    pub fn bind_descriptor(
        self,
        identity: CandidateIdentity,
        body: &[u8],
        scratch: &mut [u8],
        pins: &PinAuthority,
        proof: PublicationProof<'_>,
        now: u64,
        workspace: &mut cose::Workspace,
    ) -> Result<BoundClosure<'a>, E> {
        let body = decode_body(body).map_err(|_| E::Descriptor)?;
        let body_id = domain_digest(DigestDomain::Body, body.encoded_bytes(), scratch)
            .map_err(|_| E::Descriptor)?;
        let context = body.context();
        let context_id = domain_digest(DigestDomain::Context, context.encoded_bytes(), scratch)
            .map_err(|_| E::Descriptor)?;
        let parameters = context.parameters();
        let fields = self.manifest.fields();
        if body_id != identity.descriptor_id()
            || context_id != identity.context_id()
            || body.context_id() != context_id
            || parameters.generation != identity.generation()
            || context.entry_count() != identity.entry_count() as usize
            || parameters.semantic_contract != self.digest
            || parameters.policy != fields.policy
            || parameters.consumer_profile != fields.consumer
            || context.source().source_digest != fields.source
            || context.source().length != fields.source_length
        {
            return Err(E::Descriptor);
        }
        self.recheck(pins, now)?;
        let binding = cose::Binding {
            descriptor_digest: &body_id.0,
            key_ids: proof.key_ids,
            controller_epoch: proof.epoch,
            purpose: PUBLICATION_PURPOSE,
            audience: proof.audience,
        };
        let publisher_receipt = pins.verify_source(
            AuthoritySource::LocalPin(proof.pin),
            proof.envelope,
            &binding,
            now,
            workspace,
        )?;
        Ok(BoundClosure {
            closure: self,
            identity,
            publisher_receipt,
        })
    }
}
impl<'a> BoundClosure<'a> {
    pub const fn identity(&self) -> CandidateIdentity {
        self.identity
    }
    pub fn closure(&self) -> &AuthenticatedClosure<'a> {
        &self.closure
    }
    pub fn recheck(&self, pins: &PinAuthority, now: u64) -> Result<(), E> {
        self.closure.recheck(pins, now)?;
        pins.recheck(
            &self.publisher_receipt,
            &self.identity.descriptor_id().0,
            now,
        )
        .map_err(E::from)
    }
}
