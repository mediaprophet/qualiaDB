use super::super::managed_control::ControlState;
use super::super::{Key, KeyAlgorithm, KeyManager, KeyOperation, KeyType, SigningAccess};
use crate::crypto::network::{
    errors::CryptoError as E, key_provider::ProviderGrant, types::KeyPurpose,
};
pub(super) fn grant(
    g: ProviderGrant,
    state: &ControlState,
    purpose: KeyPurpose,
    now: u64,
) -> Result<(), E> {
    if state.cancelled {
        return Err(E::Cancelled);
    }
    if g.epoch.controller == 0 || g.epoch.epoch == 0 || g.cell_epoch == 0 {
        return Err(E::Unauthorized);
    }
    g.authorize(purpose, g.epoch.controller, now, state.generation)
}
pub(super) fn policy(
    owner: &KeyManager,
    policy: &str,
    key: &str,
    op: KeyOperation,
    access: SigningAccess<'_>,
    now: u64,
) -> Result<(), E> {
    if !owner
        .key_storage
        .access_control
        .permits_managed_operation(policy, key, op, access, now)
    {
        Err(E::Unauthorized)
    } else {
        Ok(())
    }
}
pub(super) fn private<'a>(
    owner: &'a KeyManager,
    id: &str,
    alg: KeyAlgorithm,
    now: u64,
) -> Result<&'a Key, E> {
    let k = owner
        .key_storage
        .borrow_managed_key(id)
        .ok_or(E::Unauthorized)?;
    if k.key_id != id
        || k.metadata.key_id != id
        || k.key_type != KeyType::Private
        || k.metadata.key_type != KeyType::Private
        || k.key_algorithm != alg
        || k.metadata.key_algorithm != alg
        || now < k.metadata.created_at
        || (k.metadata.expires_at != 0 && now >= k.metadata.expires_at)
    {
        return Err(E::Unauthorized);
    }
    Ok(k)
}
