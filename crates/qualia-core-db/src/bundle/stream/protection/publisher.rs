//! One private candidate attempt. Provider policy is not application authority.
use super::geometry::{EMPTY_REF, EMPTY_ROOT, PreparedGeometry};
use crate::bundle::stream::proof::{OutboardNode, TreeSpec};
use crate::crypto::network::object_stream::{Context, ObjectScope, PageRef, RootRow};
use crate::net::qdnf::types::StrongDigest;
use crate::specialized_libs::cryptographic_library::ManagedObjectPublication;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PublisherError {
    Capacity,
    State,
    Source,
    Entropy,
    Crypto,
    Codec,
    Proof,
    Sink,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Artifact {
    Context,
    Geometry,
    Roots,
    Descriptor,
    PublisherProof,
}
/// Bounded synchronous candidate staging only. Implementations must not reenter
/// managed control or await. Actual application guards and durable acceptance
/// must be supplied by the next policy/storage composition; success is not either.
pub(crate) trait CandidateSink {
    fn artifact(&mut self, kind: Artifact, page: u16, bytes: &[u8]) -> Result<(), ()>;
    fn ciphertext(&mut self, scope: ObjectScope, leaf: u64, bytes: &[u8]) -> Result<(), ()>;
    fn outboard(&mut self, spec: &TreeSpec, node: OutboardNode) -> Result<(), ()>;
}
pub(crate) struct ProtectedCandidate {
    pub(super) descriptor: StrongDigest,
    pub(super) context: StrongDigest,
}
impl ProtectedCandidate {
    pub(crate) fn descriptor_id(&self) -> StrongDigest {
        self.descriptor
    }
    pub(crate) fn context_id(&self) -> StrongDigest {
        self.context
    }
}
pub(crate) struct ProtectedPublisher<'a, 's, 'm, 'r> {
    pub(super) geometry: PreparedGeometry<'a, 's>,
    pub(super) publication: ManagedObjectPublication<'m>,
    pub(super) context: Context<'m>,
    pub(super) roots: &'r mut [RootRow],
    pub(super) root_refs: &'r mut [PageRef],
    pub(super) next: usize,
    pub(super) metadata_root: Option<StrongDigest>,
    pub(super) candidate: Option<StrongDigest>,
    pub(super) proof_staged: bool,
    pub(super) header_staged: bool,
    poisoned: bool,
}
impl<'a, 's, 'm, 'r> ProtectedPublisher<'a, 's, 'm, 'r> {
    /// Context is borrowed from separate frozen caller storage, never from a
    /// movable buffer inside this owner. No application authority is admitted.
    pub(crate) fn bind(
        geometry: PreparedGeometry<'a, 's>,
        mut publication: ManagedObjectPublication<'m>,
        context: Context<'m>,
        roots: &'r mut [RootRow],
        refs: &'r mut [PageRef],
    ) -> Result<Self, PublisherError> {
        geometry.check_context(context)?;
        let roots = roots
            .get_mut(..geometry.object_count())
            .ok_or(PublisherError::Capacity)?;
        let refs = refs
            .get_mut(..context.page_count())
            .ok_or(PublisherError::Capacity)?;
        publication
            .bind_context(context)
            .map_err(|_| PublisherError::Crypto)?;
        roots.fill(EMPTY_ROOT);
        refs.fill(EMPTY_REF);
        Ok(Self {
            geometry,
            publication,
            context,
            roots,
            root_refs: refs,
            next: 0,
            metadata_root: None,
            candidate: None,
            proof_staged: false,
            header_staged: false,
            poisoned: false,
        })
    }
    pub(crate) fn is_poisoned(&self) -> bool {
        self.poisoned || self.publication.is_poisoned()
    }
    pub(super) fn attempt<T>(
        &mut self,
        work: impl FnOnce(&mut Self) -> Result<T, PublisherError>,
    ) -> Result<T, PublisherError> {
        if self.is_poisoned() {
            return Err(PublisherError::State);
        }
        self.poisoned = true;
        struct Guard<'p, 'a, 's, 'm, 'r> {
            p: &'p mut ProtectedPublisher<'a, 's, 'm, 'r>,
            armed: bool,
        }
        impl Drop for Guard<'_, '_, '_, '_, '_> {
            fn drop(&mut self) {
                if self.armed {
                    self.p.publication.cancel();
                }
            }
        }
        let mut guard = Guard {
            p: self,
            armed: true,
        };
        let result = work(guard.p);
        if result.is_ok() {
            guard.p.poisoned = false;
            guard.armed = false;
        }
        result
    }
    pub(crate) fn stage_header(
        &mut self,
        sink: &mut impl CandidateSink,
    ) -> Result<(), PublisherError> {
        if self.header_staged {
            return Err(PublisherError::State);
        }
        self.attempt(|p| {
            let bytes = p.context.encoded();
            p.publication
                .with_current_effect(|| {
                    sink.artifact(Artifact::Context, 0, bytes)
                        .map_err(|_| crate::crypto::network::errors::CryptoError::CryptoFailure)
                })
                .map_err(|_| PublisherError::Sink)?;
            let mut scratch = [0; 1560];
            for page in 0..p.context.page_count() {
                let r = p.geometry.encode_page(page, &mut scratch)?;
                p.publication
                    .with_current_effect(|| {
                        sink.artifact(
                            Artifact::Geometry,
                            page as u16,
                            &scratch[..r.length as usize],
                        )
                        .map_err(|_| crate::crypto::network::errors::CryptoError::CryptoFailure)
                    })
                    .map_err(|_| PublisherError::Sink)?;
            }
            p.header_staged = true;
            Ok(())
        })
    }
}
impl Drop for ProtectedPublisher<'_, '_, '_, '_> {
    fn drop(&mut self) {
        self.publication.cancel();
    }
}
