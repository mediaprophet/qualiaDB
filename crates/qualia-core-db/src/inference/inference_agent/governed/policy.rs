use super::{ActorPolicyScope, ConstraintInstruction};
use crate::NQuin;
use serde::{de::SeqAccess, Deserialize, Deserializer, Serialize};
use std::{fmt, marker::PhantomData};

pub const POLICY_VERSION: u32 = 2;
pub const INFERENCE_POLICY_AUDIENCE: u64 = crate::q_hash("qualia:inference:token-acceptance:v2");
pub const MAX_POLICY_JSON_BYTES: usize = 128 * 1024;
/// Conservative admission reservation for input, bounded decoded collections,
/// and serialization. Allocator bookkeeping and caller-owned inputs are external.
pub const POLICY_COLD_BUDGET_BYTES: usize = 512 * 1024;

/// Bounded cold storage. Deserialization checks the count before retaining each
/// element, rather than accepting an unbounded Vec and checking afterwards.
#[derive(Debug, Clone, Serialize)]
#[serde(transparent)]
pub struct PolicyList<T, const N: usize>(Vec<T>);
impl<T, const N: usize> PolicyList<T, N> {
    pub fn from_vec(values: Vec<T>) -> Result<Self, PolicyError> {
        if values.len() > N || values.capacity() > N {
            return Err(PolicyError::Capacity);
        }
        Ok(Self(values))
    }
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
}
impl<T, const N: usize> Default for PolicyList<T, N> {
    fn default() -> Self {
        Self(Vec::new())
    }
}
impl<'de, T: Deserialize<'de>, const N: usize> Deserialize<'de> for PolicyList<T, N> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct Visitor<T, const N: usize>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>, const N: usize> serde::de::Visitor<'de> for Visitor<T, N> {
            type Value = PolicyList<T, N>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                write!(f, "at most {N} policy elements")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                if seq.size_hint().is_some_and(|n| n > N) {
                    return Err(serde::de::Error::custom(
                        "policy collection exceeds capacity",
                    ));
                }
                let mut values = Vec::with_capacity(N);
                while values.len() < N {
                    match seq.next_element()? {
                        Some(value) => values.push(value),
                        None => return Ok(PolicyList(values)),
                    }
                }
                if seq.next_element::<serde::de::IgnoredAny>()?.is_some() {
                    return Err(serde::de::Error::custom(
                        "policy collection exceeds capacity",
                    ));
                }
                Ok(PolicyList(values))
            }
        }
        d.deserialize_seq(Visitor::<T, N>(PhantomData))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", content = "digest", deny_unknown_fields)]
pub enum DigestScope {
    Any,
    Exact([u8; 32]),
}
impl DigestScope {
    pub(crate) fn allows(self, digest: [u8; 32]) -> bool {
        matches!(self, Self::Any) || self == Self::Exact(digest)
    }
}

/// Explicit policy content selected by the human/host. No Default implementation:
/// even an empty, permissive program must be deliberately installed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyRules {
    /// Explicit actor associations authorized by the independent policy installer.
    pub actors: PolicyList<ActorPolicyScope, 32>,
    pub model: DigestScope,
    pub request: DigestScope,
    pub graph_scopes: PolicyList<u64, 32>,
    pub program: PolicyList<ConstraintInstruction, 64>,
    pub norms: PolicyList<NQuin, 256>,
    pub facts: PolicyList<NQuin, 255>,
    pub epistemic: PolicyList<NQuin, 256>,
    pub trace: PolicyList<NQuin, 255>,
    pub required_claim: u64,
    pub temporal_invariant: u64,
    pub non_derogable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PolicyDocument {
    pub version: u32,
    pub audience: u64,
    pub principal: u64,
    pub controller_generation: u64,
    pub policy_generation: u64,
    pub not_before: u64,
    pub expires_at: u64,
    pub rules: PolicyRules,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    Capacity,
    Budget,
    InvalidProgram,
    InvalidPolicy,
    InvalidSignature,
    WrongAuthority,
    MissingPolicy,
    Expired,
    Revoked,
    StaleGeneration,
    Scope,
    InvalidCandidate,
    Clock,
    CounterExhausted,
    Session,
    Io,
    Encoding,
}
impl fmt::Display for PolicyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sentinel policy: {self:?}")
    }
}
impl std::error::Error for PolicyError {}

impl PolicyDocument {
    pub(crate) fn validate(&self) -> Result<(), PolicyError> {
        if self.version != POLICY_VERSION
            || self.principal == 0
            || self.audience == 0
            || self.controller_generation == 0
            || self.policy_generation == 0
            || self.not_before >= self.expires_at
            || self.expires_at > u32::MAX as u64
        {
            return Err(PolicyError::InvalidPolicy);
        }
        if self.rules.actors.as_slice().is_empty() {
            return Err(PolicyError::Scope);
        }
        for actor in self.rules.actors.as_slice() {
            actor.validate()?;
        }
        for instruction in self.rules.program.as_slice() {
            instruction.lower()?;
        }
        for quins in [
            self.rules.norms.as_slice(),
            self.rules.facts.as_slice(),
            self.rules.epistemic.as_slice(),
            self.rules.trace.as_slice(),
        ] {
            for q in quins {
                if q.parity != q.subject ^ q.predicate ^ q.object ^ q.context {
                    return Err(PolicyError::InvalidPolicy);
                }
            }
        }
        Ok(())
    }
    pub(crate) fn valid_at(&self, now: u64) -> Result<(), PolicyError> {
        if now < self.not_before || now >= self.expires_at {
            Err(PolicyError::Expired)
        } else {
            Ok(())
        }
    }
}
