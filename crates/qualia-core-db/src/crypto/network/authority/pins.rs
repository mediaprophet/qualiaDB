use super::operations::PendingSlot;
use super::types::{Pin, ReceiptStamp, Scope};
use super::{AuthorityReceipt, AuthoritySource, LocalBootstrap, PinConfig, PinHandle, MAX_PINS};
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::rotation::RotationState;
use crate::crypto::network::{cose, digest::sha384, entropy::fill_os};
use crate::net::peer::runtime::leases::{LeaseTable, LEASE_SLOTS};
use crate::net::qdnf::authority::TemporalGrant;
use crate::net::qdnf::types::{Generation, ProfileId};
use ed25519_dalek::VerifyingKey;
use fips204::{ml_dsa_65, traits::SerDes};

/// Owns full paired pins, not semantic grants or keys. No persistence is implied.
pub struct PinAuthority {
    pub(super) instance: [u8; 16],
    pins: [Option<Pin>; MAX_PINS],
    next_generation: Generation,
    pub(super) operations: LeaseTable,
    pub(super) pending: [Option<PendingSlot>; LEASE_SLOTS],
}
impl PinAuthority {
    /// Explicit local trust-store bootstrap. The host authenticates its admin
    /// before invoking this and never exposes the returned capability to wire input.
    pub fn new_local_bootstrap() -> Result<(Self, LocalBootstrap), CryptoError> {
        let mut instance = [0; 16];
        fill_os(&mut instance)?;
        if instance == [0; 16] {
            return Err(CryptoError::EntropyFailure);
        }
        Ok((
            Self {
                instance,
                pins: core::array::from_fn(|_| None),
                next_generation: Generation::ZERO,
                operations: LeaseTable::new(),
                pending: [None; LEASE_SLOTS],
            },
            LocalBootstrap { instance },
        ))
    }

    fn admin(&self, admin: &LocalBootstrap) -> Result<(), CryptoError> {
        if admin.instance != self.instance {
            return Err(CryptoError::Unauthorized);
        }
        Ok(())
    }

    pub fn install_pin(
        &mut self,
        admin: &LocalBootstrap,
        config: PinConfig<'_>,
        now: u64,
    ) -> Result<PinHandle, CryptoError> {
        self.admin(admin)?;
        for pin in self.pins.iter().flatten() {
            if pin.principal == config.principal
                && pin.scope.matches(config.purpose, config.audience)
            {
                return Err(CryptoError::Conflict);
            }
        }
        let slot = self
            .pins
            .iter()
            .position(Option::is_none)
            .ok_or(CryptoError::Capacity)?;
        let generation = self.next_generation.next()?;
        let pin = validated_pin(config, generation, now)?;
        self.pins[slot] = Some(pin);
        self.next_generation = generation;
        Ok(PinHandle {
            instance: self.instance,
            slot: slot as u8,
            generation,
        })
    }

    /// Explicit prospective full-pair replacement; no overlapping old epoch.
    /// Both public keys change, and scope/principal remain exact.
    pub fn rotate_pin(
        &mut self,
        admin: &LocalBootstrap,
        old_handle: PinHandle,
        config: PinConfig<'_>,
        now: u64,
    ) -> Result<PinHandle, CryptoError> {
        self.admin(admin)?;
        let old = self.record(old_handle)?;
        if old.principal != config.principal || !old.scope.matches(config.purpose, config.audience)
        {
            return Err(CryptoError::Unauthorized);
        }
        RotationState::paired(old.epoch, config.provider_epoch)?;
        let generation = self.next_generation.next()?;
        let new = validated_pin(config, generation, now)?;
        if old.ids.ml_dsa_65 == new.ids.ml_dsa_65 || old.ids.ed25519 == new.ids.ed25519 {
            return Err(CryptoError::Conflict);
        }
        // Invalidation is bounded and internally infallible under LeaseTable ownership.
        self.invalidate_operations(old_handle)?;
        self.pins[old_handle.slot as usize] = Some(new);
        self.next_generation = generation;
        Ok(PinHandle {
            generation,
            ..old_handle
        })
    }

    pub fn revoke_pin(
        &mut self,
        admin: &LocalBootstrap,
        handle: PinHandle,
    ) -> Result<(), CryptoError> {
        self.admin(admin)?;
        self.record(handle)?;
        self.pins[handle.slot as usize]
            .as_mut()
            .ok_or(CryptoError::Incomplete)?
            .revoked = true;
        self.invalidate_operations(handle)
    }

    pub fn verify_source(
        &self,
        source: AuthoritySource<'_>,
        envelope: &[u8],
        expected: &cose::Binding<'_>,
        now: u64,
        workspace: &mut cose::Workspace,
    ) -> Result<AuthorityReceipt, CryptoError> {
        let handle = match source {
            AuthoritySource::LocalPin(handle) => handle,
            AuthoritySource::SignedControllerChain(_) => return Err(CryptoError::Unsupported),
            AuthoritySource::ClassicalDid(_) => return Err(CryptoError::Unauthorized),
        };
        let pin = self.current_pin(handle, now)?;
        if expected.key_ids != pin.ids
            || expected.controller_epoch != pin.epoch.epoch
            || !pin.scope.matches(expected.purpose, expected.audience)
        {
            return Err(CryptoError::Unauthorized);
        }
        let verified = cose::verify_pair(envelope, expected, &pin.keys(), workspace)?;
        Ok(AuthorityReceipt {
            stamp: ReceiptStamp {
                pin: handle,
                principal: pin.principal,
                descriptor: *verified.binding().descriptor_digest,
            },
        })
    }

    /// Recheck before dependent use. Historical receipt possession is insufficient.
    pub fn recheck(
        &self,
        receipt: &AuthorityReceipt,
        descriptor: &[u8; 48],
        now: u64,
    ) -> Result<(), CryptoError> {
        self.check_stamp(&receipt.stamp, descriptor, now)
            .map(|_| ())
    }

    pub(super) fn check_stamp(
        &self,
        stamp: &ReceiptStamp,
        descriptor: &[u8; 48],
        now: u64,
    ) -> Result<&Pin, CryptoError> {
        let pin = self.current_pin(stamp.pin, now)?;
        if stamp.descriptor != *descriptor || stamp.principal != pin.principal {
            return Err(CryptoError::Unauthorized);
        }
        Ok(pin)
    }
    pub(super) fn current_pin(&self, handle: PinHandle, now: u64) -> Result<&Pin, CryptoError> {
        let pin = self.record(handle)?;
        pin.current_at(now)?;
        Ok(pin)
    }
    fn record(&self, handle: PinHandle) -> Result<&Pin, CryptoError> {
        if handle.instance != self.instance {
            return Err(CryptoError::Unauthorized);
        }
        let pin = self
            .pins
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .ok_or(CryptoError::Unauthorized)?;
        if pin.generation != handle.generation {
            return Err(CryptoError::StaleGeneration);
        }
        Ok(pin)
    }
}

fn validated_pin(
    config: PinConfig<'_>,
    generation: Generation,
    now: u64,
) -> Result<Pin, CryptoError> {
    if config.principal.is_zero() || config.provider_epoch.controller == 0 {
        return Err(CryptoError::Unauthorized);
    }
    if config.not_before_unix >= config.expires_unix {
        return Err(CryptoError::Malformed);
    }
    if now >= config.expires_unix {
        return Err(CryptoError::Expired);
    }
    let scope = Scope::new(config.purpose, config.audience)?;
    ml_dsa_65::PublicKey::try_from_bytes(*config.keys.ml_dsa_65)
        .map_err(|_| CryptoError::Unauthorized)?;
    let ed =
        VerifyingKey::from_bytes(config.keys.ed25519).map_err(|_| CryptoError::Unauthorized)?;
    if ed.is_weak() {
        return Err(CryptoError::Unauthorized);
    }
    let ids = config.keys.key_ids();
    let validity = TemporalGrant {
        purpose_digest: sha384(config.purpose),
        audience_digest: sha384(config.audience),
        authority_generation: generation.0,
        not_before_unix: config.not_before_unix,
        expires_unix: config.expires_unix,
        profile: ProfileId::QPR_PQ_1,
    };
    Ok(Pin {
        principal: config.principal,
        epoch: config.provider_epoch,
        ml_pk: *config.keys.ml_dsa_65,
        ed_pk: *config.keys.ed25519,
        ids,
        scope,
        validity,
        generation,
        revoked: false,
    })
}
