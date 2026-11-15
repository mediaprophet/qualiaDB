use super::*;
use crate::bundle::stream::proof::{
    LEAF_BYTES, MAX_LOGICAL_BYTES, ProofError, Representation, TreeBuilder, TreeSpec, leaf_hash,
};
use crate::net::qdnf::types::StrongDigest;
fn fields(length: u64, representation: Representation) -> IdentityFields {
    IdentityFields {
        descriptor: StrongDigest([0x11; 48]),
        context: StrongDigest([0x22; 48]),
        entry: 0x01020304,
        generation: 0x0102030405060708,
        representation,
        storage_class: if representation == Representation::Plaintext {
            StorageClass::PublicPlaintextCandidate
        } else {
            StorageClass::SharedCiphertextCandidate
        },
        key_epoch: 0,
        logical_length: length,
        root: StrongDigest([0x33; 48]),
        privacy_partition: StrongDigest([0x44; 48]),
        policy_generation: 0x1112131415161718,
        storage_generation: 0x2122232425262728,
    }
}
fn one_leaf(bytes: &[u8], representation: Representation, epoch: u32) -> StorageIdentity {
    let len = bytes.len()
        - if representation == Representation::Ciphertext {
            16
        } else {
            0
        };
    let mut f = fields(len as u64, representation);
    f.key_epoch = epoch;
    let tree = TreeSpec::new(
        f.context,
        f.entry,
        f.generation,
        representation,
        epoch,
        len as u64,
    )
    .unwrap();
    f.root = leaf_hash(&tree, 0, bytes).unwrap();
    StorageIdentity::new(f).unwrap()
}
fn checksum_record(bytes: &mut [u8; CHECKPOINT_BYTES]) {
    let digest = crate::crypto::network::digest::sha384(&bytes[..352]);
    bytes[352..400].copy_from_slice(digest.as_bytes());
}
#[path = "tests/checkpoint.rs"]
mod checkpoint;
#[path = "tests/identity.rs"]
mod identity;
#[path = "tests/leaf_bitmap.rs"]
mod leaf_bitmap;
