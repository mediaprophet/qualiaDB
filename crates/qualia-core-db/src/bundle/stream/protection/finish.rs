//! Final structural inventory and same-owner candidate signing; no rights minting.
use super::publisher::{
    Artifact, CandidateSink, ProtectedCandidate, ProtectedPublisher, PublisherError as E,
};
use crate::bundle::stream::proof::{Representation, TreeSpec, empty_root};
use crate::crypto::network::{cose, digest::sha384, errors::CryptoError, object_stream::*};
use crate::specialized_libs::cryptographic_library::PairedSigningRequest;
use zeroize::Zeroizing;
impl ProtectedPublisher<'_, '_, '_, '_> {
    pub(crate) fn finish(
        &mut self,
        out: &mut [u8],
        sink: &mut impl CandidateSink,
    ) -> Result<ProtectedCandidate, E> {
        if self.candidate.is_some() || self.next != self.geometry.object_count() {
            return Err(E::State);
        }
        self.attempt(|p| {
            let metadata_root = p.metadata_root.ok_or(E::State)?;
            if sha384(p.geometry.source.source_bytes()) != p.geometry.source_digest {
                return Err(E::Source);
            }
            let fields = p.context.fields();
            // The crypto Inventory intentionally does not validate empty roots.
            for (row, m) in p.roots.iter().zip(p.geometry.mappings) {
                let length = p.geometry.source.objects()[m.source_slot].length();
                if row.id != m.id {
                    return Err(E::Source);
                }
                if length == 0 {
                    let spec = TreeSpec::new(
                        p.context.id(),
                        m.id,
                        fields.generation,
                        Representation::Ciphertext,
                        fields.key_epoch,
                        0,
                    )
                    .map_err(|_| E::Proof)?;
                    if row.root != empty_root(&spec).map_err(|_| E::Proof)? {
                        return Err(E::Proof);
                    }
                }
            }
            let mut roots = [0; 6728];
            for page in 0..p.context.page_count() {
                let start = page * PAGE_ROWS;
                let end = (start + PAGE_ROWS).min(p.roots.len());
                let n = encode_roots(
                    p.context.id(),
                    page as u16,
                    &p.roots[start..end],
                    &mut roots,
                )
                .map_err(|_| E::Codec)?;
                p.root_refs[page] = PageRef {
                    page: page as u16,
                    rows: (end - start) as u16,
                    length: n as u32,
                    digest: sha384(&roots[..n]),
                };
            }
            let n = encode_descriptor(p.context, metadata_root, p.root_refs, out)
                .map_err(|_| E::Codec)?;
            let descriptor = Descriptor::parse(&out[..n]).map_err(|_| E::Codec)?;
            let mut inventory = Inventory::new(descriptor);
            let mut geometry = [0; 1560];
            for page in 0..p.context.page_count() {
                let g = p.geometry.encode_page(page, &mut geometry)?;
                let start = page * PAGE_ROWS;
                let end = (start + PAGE_ROWS).min(p.roots.len());
                let rn = encode_roots(
                    p.context.id(),
                    page as u16,
                    &p.roots[start..end],
                    &mut roots,
                )
                .map_err(|_| E::Codec)?;
                inventory
                    .accept(
                        GeometryPage::parse(&geometry[..g.length as usize])
                            .map_err(|_| E::Codec)?,
                        RootPage::parse(&roots[..rn]).map_err(|_| E::Codec)?,
                    )
                    .map_err(|_| E::Codec)?;
                p.publication
                    .with_current_effect(|| {
                        sink.artifact(Artifact::Roots, page as u16, &roots[..rn])
                            .map_err(|_| CryptoError::CryptoFailure)
                    })
                    .map_err(|_| E::Sink)?;
            }
            inventory.complete().map_err(|_| E::Codec)?;
            p.publication
                .with_current_effect(|| {
                    sink.artifact(Artifact::Descriptor, 0, descriptor.encoded())
                        .map_err(|_| CryptoError::CryptoFailure)
                })
                .map_err(|_| E::Sink)?;
            p.candidate = Some(descriptor.id());
            Ok(ProtectedCandidate {
                descriptor: descriptor.id(),
                context: p.context.id(),
            })
        })
    }
    /// A signed candidate remains unrecognized until actual publisher pin and
    /// application-policy verification; this is not durable/public promotion.
    pub(crate) fn sign_candidate(
        &mut self,
        descriptor: &[u8],
        request: PairedSigningRequest<'_>,
        workspace: &mut cose::Workspace,
        sink: &mut impl CandidateSink,
    ) -> Result<(), E> {
        if self.proof_staged {
            return Err(E::State);
        }
        self.attempt(|p| {
            let descriptor = Descriptor::parse(descriptor).map_err(|_| E::Codec)?;
            if p.candidate != Some(descriptor.id()) {
                return Err(E::Source);
            }
            let mut proof = Zeroizing::new([0; cose::MAX_ENVELOPE_BYTES]);
            let n = p
                .publication
                .sign_descriptor(descriptor, request, workspace, &mut proof[..])
                .map_err(|_| E::Crypto)?;
            p.publication
                .with_current_effect(|| {
                    sink.artifact(Artifact::PublisherProof, 0, &proof[..n])
                        .map_err(|_| CryptoError::CryptoFailure)
                })
                .map_err(|_| E::Sink)?;
            p.proof_staged = true;
            Ok(())
        })
    }
}
