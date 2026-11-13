use super::{StorageError as E, wire::*};
use crate::bundle::stream::proof::{Representation, TreeSpec};
use crate::net::qdnf::types::StrongDigest;
pub const IDENTITY_BYTES: usize = 256;
const MAGIC: &[u8; 8] = b"QBSID01\0";
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageClass {
    PublicPlaintextCandidate = 0,
    SharedCiphertextCandidate = 1,
    PrivatePlaintext = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentityFields {
    pub descriptor: StrongDigest,
    pub context: StrongDigest,
    pub entry: u32,
    pub generation: u64,
    pub representation: Representation,
    pub storage_class: StorageClass,
    pub key_epoch: u32,
    pub logical_length: u64,
    pub root: StrongDigest,
    pub privacy_partition: StrongDigest,
    pub policy_generation: u64,
    pub storage_generation: u64,
}
/// Structurally supported identity. Its fields and roots are not trusted authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StorageIdentity {
    fields: IdentityFields,
    tree: TreeSpec,
}
impl StorageIdentity {
    pub fn new(fields: IdentityFields) -> Result<Self, E> {
        match (fields.storage_class, fields.representation) {
            (StorageClass::PublicPlaintextCandidate, Representation::Plaintext)
            | (StorageClass::SharedCiphertextCandidate, Representation::Ciphertext) => {}
            _ => return Err(E::Unsupported),
        }
        let tree = TreeSpec::new(
            fields.context,
            fields.entry,
            fields.generation,
            fields.representation,
            fields.key_epoch,
            fields.logical_length,
        )
        .map_err(E::Proof)?;
        Ok(Self { fields, tree })
    }
    pub const fn fields(&self) -> &IdentityFields {
        &self.fields
    }
    pub const fn tree_spec(&self) -> TreeSpec {
        self.tree
    }
    pub const fn leaf_count(&self) -> u64 {
        self.tree.leaf_count()
    }
}
pub fn encode_identity(identity: &StorageIdentity, out: &mut [u8]) -> Result<usize, E> {
    let out = out.get_mut(..IDENTITY_BYTES).ok_or(E::Capacity)?;
    out.fill(0);
    let f = identity.fields();
    out[..8].copy_from_slice(MAGIC);
    out[8..10].copy_from_slice(&1u16.to_be_bytes());
    out[16..64].copy_from_slice(f.descriptor.as_bytes());
    out[64..112].copy_from_slice(f.context.as_bytes());
    out[112..116].copy_from_slice(&f.entry.to_be_bytes());
    out[116..124].copy_from_slice(&f.generation.to_be_bytes());
    out[124] = f.representation as u8;
    out[125] = f.storage_class as u8;
    out[128..132].copy_from_slice(&f.key_epoch.to_be_bytes());
    out[132..140].copy_from_slice(&f.logical_length.to_be_bytes());
    out[140..188].copy_from_slice(f.root.as_bytes());
    out[188..236].copy_from_slice(f.privacy_partition.as_bytes());
    out[236..244].copy_from_slice(&f.policy_generation.to_be_bytes());
    out[244..252].copy_from_slice(&f.storage_generation.to_be_bytes());
    Ok(IDENTITY_BYTES)
}
pub fn decode_identity(bytes: &[u8]) -> Result<StorageIdentity, E> {
    if bytes.len() != IDENTITY_BYTES {
        return Err(E::Length);
    }
    if &bytes[..8] != MAGIC {
        return Err(E::Magic);
    }
    if u16_at(bytes, 8)? != 1 {
        return Err(E::Version);
    }
    if u16_at(bytes, 10)? != 0 {
        return Err(E::Flags);
    }
    zero(&bytes[12..16])?;
    zero(&bytes[126..128])?;
    zero(&bytes[252..256])?;
    let representation = match bytes[124] {
        0 => Representation::Plaintext,
        1 => Representation::Ciphertext,
        _ => return Err(E::Unsupported),
    };
    let storage_class = match bytes[125] {
        0 => StorageClass::PublicPlaintextCandidate,
        1 => StorageClass::SharedCiphertextCandidate,
        _ => return Err(E::Unsupported),
    };
    StorageIdentity::new(IdentityFields {
        descriptor: digest_at(bytes, 16)?,
        context: digest_at(bytes, 64)?,
        entry: u32_at(bytes, 112)?,
        generation: u64_at(bytes, 116)?,
        representation,
        storage_class,
        key_epoch: u32_at(bytes, 128)?,
        logical_length: u64_at(bytes, 132)?,
        root: digest_at(bytes, 140)?,
        privacy_partition: digest_at(bytes, 188)?,
        policy_generation: u64_at(bytes, 236)?,
        storage_generation: u64_at(bytes, 244)?,
    })
}
