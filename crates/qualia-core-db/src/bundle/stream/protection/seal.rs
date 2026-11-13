//! Ciphertext and proof construction from actual managed monotonic sealers.
use super::{
    metadata::{self, Erase, PrivateReferences},
    publisher::{CandidateSink, ProtectedPublisher, PublisherError as E},
};
use crate::bundle::stream::proof::{ProofError, Representation, TreeBuilder, TreeSpec};
use crate::crypto::network::{errors::CryptoError, object_stream::*};
use crate::net::qdnf::types::StrongDigest;
use crate::specialized_libs::cryptographic_library::ManagedObjectSealer;
use sha2::{Digest, Sha384};
impl ProtectedPublisher<'_, '_, '_, '_> {
    pub(crate) fn seal_next_object(
        &mut self,
        scratch: &mut [u8],
        sink: &mut impl CandidateSink,
    ) -> Result<(), E> {
        let erased_leaf = Erase { bytes: scratch };
        if !self.header_staged
            || self.next >= self.geometry.object_count()
            || self.metadata_root.is_some()
        {
            return Err(E::State);
        }
        self.attempt(|p| {
            let sorted = p.next;
            let mapping = p.geometry.mappings[sorted];
            let bytes = p
                .geometry
                .source
                .object_bytes(mapping.source_slot)
                .map_err(|_| E::Source)?;
            let mut page = [0; 1560];
            let r = p.geometry.encode_page(sorted / PAGE_ROWS, &mut page)?;
            let geometry = GeometryPage::parse(&page[..r.length as usize]).map_err(|_| E::Codec)?;
            let mut sealer = p
                .publication
                .begin_object(geometry, sorted % PAGE_ROWS)
                .map_err(|_| E::Crypto)?;
            let root = seal_bytes(&mut sealer, bytes, mapping.digest, erased_leaf.bytes, sink)?;
            sealer.finish().map_err(|_| E::Crypto)?;
            p.roots[sorted] = RootRow {
                id: mapping.id,
                root,
            };
            p.next += 1;
            Ok(())
        })
    }
    pub(crate) fn seal_metadata(
        &mut self,
        refs: PrivateReferences,
        private_scratch: &mut [u8],
        leaf_scratch: &mut [u8],
        sink: &mut impl CandidateSink,
    ) -> Result<(), E> {
        let erased_leaf = Erase {
            bytes: leaf_scratch,
        };
        // Erase the entire caller-provided private buffer on every return/unwind.
        let erased = Erase {
            bytes: private_scratch,
        };
        if !self.header_staged
            || self.next != self.geometry.object_count()
            || self.metadata_root.is_some()
        {
            return Err(E::State);
        }
        self.attempt(|p| {
            p.publication
                .with_current_effect(|| Ok(()))
                .map_err(|_| E::Crypto)?;
            metadata::write(&p.geometry, &p.publication, p.context, refs, erased.bytes)?;
            let expected = crate::crypto::network::digest::sha384(erased.bytes);
            let mut sealer = p.publication.begin_metadata().map_err(|_| E::Crypto)?;
            let root = seal_bytes(&mut sealer, erased.bytes, expected, erased_leaf.bytes, sink)?;
            sealer.finish().map_err(|_| E::Crypto)?;
            p.metadata_root = Some(root);
            Ok(())
        })
    }
}
fn seal_bytes(
    sealer: &mut ManagedObjectSealer<'_, '_>,
    bytes: &[u8],
    expected: StrongDigest,
    scratch: &mut [u8],
    sink: &mut impl CandidateSink,
) -> Result<StrongDigest, E> {
    let erased = Erase { bytes: scratch };
    if erased.bytes.len() != LEAF_BYTES as usize + 16 {
        return Err(E::Capacity);
    }
    let scope = sealer.scope();
    if scope.length() != bytes.len() as u64 {
        return Err(E::Source);
    }
    let spec = TreeSpec::new(
        scope.context(),
        scope.id(),
        scope.generation(),
        Representation::Ciphertext,
        scope.epoch(),
        scope.length(),
    )
    .map_err(|_| E::Proof)?;
    let mut tree = TreeBuilder::new(spec);
    let mut hash = Sha384::new();
    for (index, plain) in bytes.chunks(LEAF_BYTES as usize).enumerate() {
        let n = plain.len();
        erased.bytes[..n].copy_from_slice(plain);
        hash.update(&erased.bytes[..n]);
        let (cipher, tail) = erased.bytes.split_at_mut(n);
        let tag: &mut [u8; 16] = (&mut tail[..16]).try_into().map_err(|_| E::Capacity)?;
        if sealer.seal_next(cipher, tag).map_err(|_| E::Crypto)? != index as u64 {
            return Err(E::State);
        }
        tree.append(&erased.bytes[..n + 16], |node| {
            sealer
                .with_current_effect(|| {
                    sink.outboard(&spec, node)
                        .map_err(|_| CryptoError::CryptoFailure)
                })
                .map_err(|_| ProofError::Sink)
        })
        .map_err(|_| E::Sink)?;
        sealer
            .with_current_effect(|| {
                sink.ciphertext(scope, index as u64, &erased.bytes[..n + 16])
                    .map_err(|_| CryptoError::CryptoFailure)
            })
            .map_err(|_| E::Sink)?;
    }
    if hash.finalize().as_slice() != expected.0.as_slice() {
        return Err(E::Source);
    }
    tree.finish(|node| {
        sealer
            .with_current_effect(|| {
                sink.outboard(&spec, node)
                    .map_err(|_| CryptoError::CryptoFailure)
            })
            .map_err(|_| ProofError::Sink)
    })
    .map_err(|_| E::Sink)
}
