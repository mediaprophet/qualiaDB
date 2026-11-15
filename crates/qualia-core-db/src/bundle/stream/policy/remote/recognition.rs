//! Purpose-scoped recognition table. Metadata only; PinAuthority remains the key owner.
use super::codec::DisclosureAction;
use super::PolicyError as E;
use crate::crypto::network::authority::PinHandle;
use crate::crypto::network::entropy::fill_os;
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::object_stream::MAX_OBJECT_BYTES;
use crate::net::qdnf::types::{Generation, OperationId, StrongDigest};

pub const MAX_RECOGNIZED: usize = 8;

/// Crate-local admin token analogous to LocalBootstrap. Not a wire role flag.
pub struct RecognitionAdmin {
    instance: [u8; 16],
}

/// Process-local selection of an enrolled controller row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecognitionHandle {
    instance: [u8; 16],
    slot: u8,
    generation: Generation,
}

impl RecognitionHandle {
    pub const fn generation(&self) -> Generation {
        self.generation
    }
}

/// Allowed ticket actions for one enrollment. Unknown bits refuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllowedOps(u16);

impl AllowedOps {
    pub const METADATA_RELEASE: Self = Self(1);
    pub const PAYLOAD_USE: Self = Self(2);
    pub const BOTH: Self = Self(3);

    pub fn from_bits(bits: u16) -> Result<Self, E> {
        if bits == 0 || bits & !Self::BOTH.0 != 0 {
            return Err(E::Reference);
        }
        Ok(Self(bits))
    }

    pub const fn bits(self) -> u16 {
        self.0
    }

    pub const fn contains(self, action: DisclosureAction) -> bool {
        self.0 & action.bit() != 0
    }
}

/// Trusted host enrollment. Exact source/policy; no wildcard or delegation chain.
#[derive(Clone, Copy, Debug)]
pub struct RecognitionEnrollment {
    pub pin: PinHandle,
    pub principal: StrongDigest,
    pub source: StrongDigest,
    pub source_length: u64,
    pub policy: StrongDigest,
    pub publisher: StrongDigest,
    pub purpose: StrongDigest,
    pub allowed_ops: AllowedOps,
    pub operation: OperationId,
    pub expires_unix: u64,
    pub max_decision_lifetime: u64,
}

/// Snapshot of a currently recognized controller. Not a disclosure grant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecognizedController {
    pin: PinHandle,
    principal: StrongDigest,
    source: StrongDigest,
    source_length: u64,
    policy: StrongDigest,
    publisher: StrongDigest,
    purpose: StrongDigest,
    allowed_ops: AllowedOps,
    operation: OperationId,
    expires_unix: u64,
    max_decision_lifetime: u64,
    controller_generation: Generation,
    decision_id_floor: u64,
}

impl RecognizedController {
    pub const fn pin(&self) -> PinHandle {
        self.pin
    }
    pub const fn principal(&self) -> StrongDigest {
        self.principal
    }
    pub const fn source(&self) -> StrongDigest {
        self.source
    }
    pub const fn source_length(&self) -> u64 {
        self.source_length
    }
    pub const fn policy(&self) -> StrongDigest {
        self.policy
    }
    pub const fn publisher(&self) -> StrongDigest {
        self.publisher
    }
    pub const fn purpose(&self) -> StrongDigest {
        self.purpose
    }
    pub const fn allowed_ops(&self) -> AllowedOps {
        self.allowed_ops
    }
    pub const fn operation(&self) -> OperationId {
        self.operation
    }
    pub const fn expires_unix(&self) -> u64 {
        self.expires_unix
    }
    pub const fn max_decision_lifetime(&self) -> u64 {
        self.max_decision_lifetime
    }
    pub const fn controller_generation(&self) -> Generation {
        self.controller_generation
    }
    pub const fn decision_id_floor(&self) -> u64 {
        self.decision_id_floor
    }

    pub fn current_at(&self, now: u64) -> Result<(), E> {
        if now >= self.expires_unix {
            Err(E::Crypto(CryptoError::Expired))
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy)]
struct Record {
    pin: PinHandle,
    principal: StrongDigest,
    source: StrongDigest,
    source_length: u64,
    policy: StrongDigest,
    publisher: StrongDigest,
    purpose: StrongDigest,
    allowed_ops: AllowedOps,
    operation: OperationId,
    expires_unix: u64,
    max_decision_lifetime: u64,
    generation: Generation,
    controller_generation: Generation,
    next_decision_id: u64,
    decision_id_floor: u64,
}

/// Fixed eight-slot table. Default deny; no implicit eviction; no key material.
pub struct ControllerRecognition {
    instance: [u8; 16],
    records: [Option<Record>; MAX_RECOGNIZED],
    next_generation: Generation,
}

impl ControllerRecognition {
    /// Host authenticates its administrator before invoking this.
    pub fn new_local_admin() -> Result<(Self, RecognitionAdmin), E> {
        let mut instance = [0; 16];
        fill_os(&mut instance).map_err(E::Crypto)?;
        if instance == [0; 16] {
            return Err(E::Crypto(CryptoError::EntropyFailure));
        }
        Ok((
            Self {
                instance,
                records: core::array::from_fn(|_| None),
                next_generation: Generation::ZERO,
            },
            RecognitionAdmin { instance },
        ))
    }

    fn admin(&self, admin: &RecognitionAdmin) -> Result<(), E> {
        if admin.instance != self.instance {
            return Err(E::Crypto(CryptoError::Unauthorized));
        }
        Ok(())
    }

    pub fn install(
        &mut self,
        admin: &RecognitionAdmin,
        enrollment: RecognitionEnrollment,
        now: u64,
    ) -> Result<RecognitionHandle, E> {
        self.admin(admin)?;
        validate_enrollment(enrollment, now)?;
        for row in self.records.iter().flatten() {
            if row.source == enrollment.source && row.policy == enrollment.policy {
                return Err(E::Crypto(CryptoError::Conflict));
            }
        }
        let slot = self
            .records
            .iter()
            .position(Option::is_none)
            .ok_or(E::Capacity)?;
        let generation = self.next_generation.next().map_err(E::Authority)?;
        self.records[slot] = Some(Record {
            pin: enrollment.pin,
            principal: enrollment.principal,
            source: enrollment.source,
            source_length: enrollment.source_length,
            policy: enrollment.policy,
            publisher: enrollment.publisher,
            purpose: enrollment.purpose,
            allowed_ops: enrollment.allowed_ops,
            operation: enrollment.operation,
            expires_unix: enrollment.expires_unix,
            max_decision_lifetime: enrollment.max_decision_lifetime,
            generation,
            controller_generation: generation,
            next_decision_id: 1,
            decision_id_floor: 1,
        });
        self.next_generation = generation;
        Ok(RecognitionHandle {
            instance: self.instance,
            slot: slot as u8,
            generation,
        })
    }

    /// Explicit replacement. Old PinHandle and decision ids do not survive.
    pub fn rotate(
        &mut self,
        admin: &RecognitionAdmin,
        handle: RecognitionHandle,
        enrollment: RecognitionEnrollment,
        now: u64,
    ) -> Result<RecognitionHandle, E> {
        self.admin(admin)?;
        validate_enrollment(enrollment, now)?;
        let old = *self.record(handle)?;
        if enrollment.source != old.source
            || enrollment.policy != old.policy
            || enrollment.source_length != old.source_length
        {
            return Err(E::Reference);
        }
        if enrollment.pin == old.pin {
            return Err(E::Crypto(CryptoError::Conflict));
        }
        let generation = self.next_generation.next().map_err(E::Authority)?;
        let slot = handle.slot as usize;
        self.records[slot] = Some(Record {
            pin: enrollment.pin,
            principal: enrollment.principal,
            publisher: enrollment.publisher,
            purpose: enrollment.purpose,
            allowed_ops: enrollment.allowed_ops,
            operation: enrollment.operation,
            expires_unix: enrollment.expires_unix,
            max_decision_lifetime: enrollment.max_decision_lifetime,
            generation,
            controller_generation: generation,
            decision_id_floor: old.next_decision_id,
            next_decision_id: old.next_decision_id,
            source: old.source,
            source_length: old.source_length,
            policy: old.policy,
        });
        self.next_generation = generation;
        Ok(RecognitionHandle {
            instance: self.instance,
            slot: handle.slot,
            generation,
        })
    }

    /// Default deny. Exact source/policy/principal/publisher/purpose/operation.
    pub fn recognize(
        &self,
        principal: StrongDigest,
        source: StrongDigest,
        source_length: u64,
        policy: StrongDigest,
        publisher: StrongDigest,
        purpose: StrongDigest,
        operation: OperationId,
        action: DisclosureAction,
        now: u64,
    ) -> Result<RecognizedController, E> {
        if source.is_zero() || source_length == 0 || principal.is_zero() || policy.is_zero() {
            return Err(E::Reference);
        }
        let row = self
            .records
            .iter()
            .flatten()
            .find(|r| {
                r.principal == principal
                    && r.source == source
                    && r.source_length == source_length
                    && r.policy == policy
            })
            .ok_or(E::Reference)?;
        if row.publisher != publisher || row.purpose != purpose || row.operation != operation {
            return Err(E::Reference);
        }
        if !row.allowed_ops.contains(action) {
            return Err(E::Reference);
        }
        if now >= row.expires_unix {
            return Err(E::Crypto(CryptoError::Expired));
        }
        Ok(snapshot(row))
    }

    pub fn consume_decision_id(
        &mut self,
        admin: &RecognitionAdmin,
        handle: RecognitionHandle,
    ) -> Result<u64, E> {
        self.admin(admin)?;
        let rec = self.record_mut(handle)?;
        let id = rec.next_decision_id;
        rec.next_decision_id = id.checked_add(1).ok_or(E::Limit)?;
        Ok(id)
    }

    pub fn snapshot(&self, handle: RecognitionHandle) -> Result<RecognizedController, E> {
        Ok(snapshot(self.record(handle)?))
    }

    fn record(&self, handle: RecognitionHandle) -> Result<&Record, E> {
        if handle.instance != self.instance {
            return Err(E::Crypto(CryptoError::Unauthorized));
        }
        let row = self
            .records
            .get(handle.slot as usize)
            .and_then(Option::as_ref)
            .ok_or(E::Crypto(CryptoError::Unauthorized))?;
        if row.generation != handle.generation {
            return Err(E::Crypto(CryptoError::StaleGeneration));
        }
        Ok(row)
    }

    fn record_mut(&mut self, handle: RecognitionHandle) -> Result<&mut Record, E> {
        if handle.instance != self.instance {
            return Err(E::Crypto(CryptoError::Unauthorized));
        }
        let row = self
            .records
            .get_mut(handle.slot as usize)
            .and_then(Option::as_mut)
            .ok_or(E::Crypto(CryptoError::Unauthorized))?;
        if row.generation != handle.generation {
            return Err(E::Crypto(CryptoError::StaleGeneration));
        }
        Ok(row)
    }
}

fn snapshot(row: &Record) -> RecognizedController {
    RecognizedController {
        pin: row.pin,
        principal: row.principal,
        source: row.source,
        source_length: row.source_length,
        policy: row.policy,
        publisher: row.publisher,
        purpose: row.purpose,
        allowed_ops: row.allowed_ops,
        operation: row.operation,
        expires_unix: row.expires_unix,
        max_decision_lifetime: row.max_decision_lifetime,
        controller_generation: row.controller_generation,
        decision_id_floor: row.decision_id_floor,
    }
}

fn validate_enrollment(e: RecognitionEnrollment, now: u64) -> Result<(), E> {
    if e.principal.is_zero()
        || e.source.is_zero()
        || e.policy.is_zero()
        || e.publisher.is_zero()
        || e.purpose.is_zero()
        || e.operation.is_zero()
    {
        return Err(E::Reference);
    }
    if e.source_length < 64 || e.source_length > MAX_OBJECT_BYTES {
        return Err(E::Reference);
    }
    if e.max_decision_lifetime == 0 || e.max_decision_lifetime > super::MAX_DECISION_LEASE_SECS {
        return Err(E::Limit);
    }
    if now >= e.expires_unix {
        return Err(E::Crypto(CryptoError::Expired));
    }
    Ok(())
}
