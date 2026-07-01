//! Exact packed homomorphic arithmetic using the BFV Ring-LWE scheme.
//!
//! This is a composition boundary, not a Webizen evaluator hot path. The upstream
//! cryptographic backend owns bounded polynomial/key buffers. Serialized ciphertexts
//! are copied into caller-owned storage and represented elsewhere by [`HeCiphertextRef`].

use std::sync::Arc;

#[cfg(test)]
use fhe::bfv::BfvParametersBuilder;
use fhe::bfv::{
    BfvParameters, Ciphertext, Encoding, EvaluationKey, EvaluationKeyBuilder, Plaintext, PublicKey,
    RelinearizationKey, SecretKey,
};
use fhe_traits::{
    DeserializeParametrized, FheDecoder, FheDecrypter, FheEncoder, FheEncrypter,
    Serialize as FheSerialize,
};

pub const MAX_BFV_PACKED_SLOTS: usize = 4096;
pub const MAX_SERIALIZED_HE_CONTEXT_BYTES: usize = 42 * 1024 * 1024;
const PLAINTEXT_MODULUS_BITS: usize = 20;
const PARAMETER_SET_INDEX: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HeScheme {
    Bfv = 1,
}

/// Fixed-size ABI reference to a ciphertext held in an external bounded store.
///
/// This is exactly 48 bytes (six `u64`s), like `NQuin`, but it is not semantic RDF
/// data and does not overload any Quin bit fields.
#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct HeCiphertextRef {
    pub store_id: u64,
    pub key_id_hash: u64,
    pub parameter_fingerprint: u64,
    pub commitment_lo: u64,
    pub commitment_hi: u64,
    packed: u64,
}

impl HeCiphertextRef {
    pub const fn slot_count(self) -> u32 {
        self.packed as u32
    }

    pub const fn scheme(self) -> Option<HeScheme> {
        match ((self.packed >> 32) & 0xff) as u8 {
            value if value == HeScheme::Bfv as u8 => Some(HeScheme::Bfv),
            _ => None,
        }
    }

    pub const fn level(self) -> u8 {
        ((self.packed >> 40) & 0xff) as u8
    }

    const fn new(
        store_id: u64,
        key_id_hash: u64,
        parameter_fingerprint: u64,
        commitment_lo: u64,
        commitment_hi: u64,
        slot_count: u32,
        level: u8,
    ) -> Self {
        Self {
            store_id,
            key_id_hash,
            parameter_fingerprint,
            commitment_lo,
            commitment_hi,
            packed: slot_count as u64 | ((HeScheme::Bfv as u64) << 32) | ((level as u64) << 40),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeError {
    Backend,
    ParameterSetUnavailable,
    EmptyInput,
    TooManySlots,
    PlaintextOutOfRange,
    OutputBufferTooSmall,
    CiphertextMismatch,
    InvalidScale,
    NonFiniteInput,
    MemoryLimitExceeded,
    CommitmentMismatch,
}

impl HeError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Backend => "BFV backend operation failed",
            Self::ParameterSetUnavailable => "128-bit BFV parameter set unavailable",
            Self::EmptyInput => "at least one BFV slot is required",
            Self::TooManySlots => "input exceeds the configured BFV packing capacity",
            Self::PlaintextOutOfRange => "value exceeds the signed BFV plaintext range",
            Self::OutputBufferTooSmall => "caller output buffer is too small",
            Self::CiphertextMismatch => {
                "ciphertexts use different keys, parameters, or slot counts"
            }
            Self::InvalidScale => "fixed-point scale must be positive",
            Self::NonFiniteInput => "fixed-point input contains a non-finite value",
            Self::MemoryLimitExceeded => "serialized BFV context exceeds the 42 MiB boundary",
            Self::CommitmentMismatch => "ciphertext bytes do not match the ABI reference",
        }
    }
}

impl core::fmt::Display for HeError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl std::error::Error for HeError {}

/// Ciphertext object owned at the crypto boundary.
#[derive(Debug, Clone)]
pub struct BfvCiphertext {
    inner: Ciphertext,
    reference: HeCiphertextRef,
}

impl BfvCiphertext {
    pub const fn reference(&self) -> HeCiphertextRef {
        self.reference
    }
}

/// One BFV key/evaluation context using parameters selected for ~128-bit security.
pub struct BfvEngine {
    parameters: Arc<BfvParameters>,
    secret_key: SecretKey,
    public_key: PublicKey,
    relinearization_key: RelinearizationKey,
    evaluation_key: EvaluationKey,
    key_id_hash: u64,
    parameter_fingerprint: u64,
    serialized_context_bytes: usize,
}

impl BfvEngine {
    /// Generate a new BFV context. This is intentionally explicit and potentially
    /// expensive; `PrivacyEngine::new()` never generates cryptographic keys.
    pub fn generate_128_bit(key_id_hash: u64) -> Result<Self, HeError> {
        let parameters = BfvParameters::default_parameters_128(PLAINTEXT_MODULUS_BITS)
            .map_err(|_| HeError::Backend)?
            .nth(PARAMETER_SET_INDEX)
            .ok_or(HeError::ParameterSetUnavailable)?;
        if parameters.degree() != MAX_BFV_PACKED_SLOTS {
            return Err(HeError::ParameterSetUnavailable);
        }
        Self::from_parameters(key_id_hash, parameters)
    }

    fn from_parameters(key_id_hash: u64, parameters: Arc<BfvParameters>) -> Result<Self, HeError> {
        let mut random = rand09::rng();
        let secret_key = SecretKey::random(&parameters, &mut random);
        let public_key = PublicKey::new(&secret_key, &mut random);
        let relinearization_key =
            RelinearizationKey::new(&secret_key, &mut random).map_err(|_| HeError::Backend)?;
        let evaluation_key = EvaluationKeyBuilder::new_leveled(&secret_key, 0, 0)
            .map_err(|_| HeError::Backend)?
            .enable_inner_sum()
            .map_err(|_| HeError::Backend)?
            .build(&mut random)
            .map_err(|_| HeError::Backend)?;
        // Keep each serialization temporary in a separate statement so setup never
        // retains all five byte vectors at once.
        let mut serialized_context_bytes = parameters.to_bytes().len();
        serialized_context_bytes = serialized_context_bytes
            .checked_add(secret_key.to_bytes().len())
            .ok_or(HeError::MemoryLimitExceeded)?;
        serialized_context_bytes = serialized_context_bytes
            .checked_add(public_key.to_bytes().len())
            .ok_or(HeError::MemoryLimitExceeded)?;
        serialized_context_bytes = serialized_context_bytes
            .checked_add(relinearization_key.to_bytes().len())
            .ok_or(HeError::MemoryLimitExceeded)?;
        serialized_context_bytes = serialized_context_bytes
            .checked_add(evaluation_key.to_bytes().len())
            .ok_or(HeError::MemoryLimitExceeded)?;
        if serialized_context_bytes > MAX_SERIALIZED_HE_CONTEXT_BYTES {
            return Err(HeError::MemoryLimitExceeded);
        }
        let parameter_fingerprint = fingerprint_parameters(&parameters);

        Ok(Self {
            parameters,
            secret_key,
            public_key,
            relinearization_key,
            evaluation_key,
            key_id_hash,
            parameter_fingerprint,
            serialized_context_bytes,
        })
    }

    #[cfg(test)]
    fn generate_test_context(key_id_hash: u64) -> Result<Self, HeError> {
        // Small algebraically equivalent parameters keep unit tests fast. They make
        // no security claim; production can only enter through `generate_128_bit`.
        let parameters = BfvParametersBuilder::new()
            .set_degree(16)
            .set_plaintext_modulus(1153)
            .set_moduli_sizes(&[50, 50, 50])
            .build_arc()
            .map_err(|_| HeError::Backend)?;
        Self::from_parameters(key_id_hash, parameters)
    }

    pub const fn key_id_hash(&self) -> u64 {
        self.key_id_hash
    }

    pub const fn parameter_fingerprint(&self) -> u64 {
        self.parameter_fingerprint
    }

    pub const fn serialized_context_bytes(&self) -> usize {
        self.serialized_context_bytes
    }

    pub fn plaintext_modulus(&self) -> u64 {
        self.parameters.plaintext()
    }

    pub fn signed_plaintext_limit(&self) -> i64 {
        (self.parameters.plaintext() / 2) as i64
    }

    /// Encrypt signed packed integers with the public key.
    pub fn encrypt_i64(&self, store_id: u64, values: &[i64]) -> Result<BfvCiphertext, HeError> {
        self.validate_values(values)?;
        let plaintext = Plaintext::try_encode(values, Encoding::simd(), &self.parameters)
            .map_err(|_| HeError::Backend)?;
        let mut random = rand09::rng();
        let inner = self
            .public_key
            .try_encrypt(&plaintext, &mut random)
            .map_err(|_| HeError::Backend)?;
        self.wrap(store_id, values.len(), inner)
    }

    /// Homomorphic packed addition.
    pub fn add(
        &self,
        store_id: u64,
        left: &BfvCiphertext,
        right: &BfvCiphertext,
    ) -> Result<BfvCiphertext, HeError> {
        self.validate_pair(left, right)?;
        self.wrap(
            store_id,
            left.reference.slot_count() as usize,
            &left.inner + &right.inner,
        )
    }

    /// Homomorphic packed element-wise multiplication followed by relinearization.
    pub fn multiply(
        &self,
        store_id: u64,
        left: &BfvCiphertext,
        right: &BfvCiphertext,
    ) -> Result<BfvCiphertext, HeError> {
        self.validate_pair(left, right)?;
        let mut product = &left.inner * &right.inner;
        self.relinearization_key
            .relinearizes(&mut product)
            .map_err(|_| HeError::Backend)?;
        self.wrap(store_id, left.reference.slot_count() as usize, product)
    }

    /// Homomorphic packed dot product. The result is repeated by the BFV inner-sum
    /// rotation circuit; callers read slot zero.
    pub fn dot_product(
        &self,
        store_id: u64,
        left: &BfvCiphertext,
        right: &BfvCiphertext,
    ) -> Result<BfvCiphertext, HeError> {
        self.validate_pair(left, right)?;
        let mut product = &left.inner * &right.inner;
        self.relinearization_key
            .relinearizes(&mut product)
            .map_err(|_| HeError::Backend)?;
        let sum = self
            .evaluation_key
            .computes_inner_sum(&product)
            .map_err(|_| HeError::Backend)?;
        self.wrap(store_id, 1, sum)
    }

    /// Decrypt into a caller-owned buffer. Only the originally packed slots are copied.
    pub fn decrypt_i64_into(
        &self,
        ciphertext: &BfvCiphertext,
        out: &mut [i64],
    ) -> Result<usize, HeError> {
        self.validate_ciphertext(ciphertext)?;
        let count = ciphertext.reference.slot_count() as usize;
        if out.len() < count {
            return Err(HeError::OutputBufferTooSmall);
        }
        let plaintext = self
            .secret_key
            .try_decrypt(&ciphertext.inner)
            .map_err(|_| HeError::Backend)?;
        let decoded =
            Vec::<i64>::try_decode(&plaintext, Encoding::simd()).map_err(|_| HeError::Backend)?;
        out[..count].copy_from_slice(&decoded[..count]);
        Ok(count)
    }

    /// Serialize into a caller-owned external ciphertext store.
    pub fn serialize_into(
        &self,
        ciphertext: &BfvCiphertext,
        out: &mut [u8],
    ) -> Result<usize, HeError> {
        self.validate_ciphertext(ciphertext)?;
        let serialized = ciphertext.inner.to_bytes();
        if out.len() < serialized.len() {
            return Err(HeError::OutputBufferTooSmall);
        }
        out[..serialized.len()].copy_from_slice(&serialized);
        Ok(serialized.len())
    }

    /// Restore a ciphertext from external storage and recompute its commitment.
    pub fn deserialize(
        &self,
        store_id: u64,
        slot_count: usize,
        bytes: &[u8],
    ) -> Result<BfvCiphertext, HeError> {
        validate_slot_count(slot_count)?;
        let inner =
            Ciphertext::from_bytes(bytes, &self.parameters).map_err(|_| HeError::Backend)?;
        self.wrap(store_id, slot_count, inner)
    }

    /// Restore bytes only when they match a previously issued ABI reference.
    pub fn deserialize_verified(
        &self,
        reference: HeCiphertextRef,
        bytes: &[u8],
    ) -> Result<BfvCiphertext, HeError> {
        if reference.key_id_hash != self.key_id_hash
            || reference.parameter_fingerprint != self.parameter_fingerprint
            || reference.scheme() != Some(HeScheme::Bfv)
        {
            return Err(HeError::CiphertextMismatch);
        }
        let digest = blake3::hash(bytes);
        let digest_bytes = digest.as_bytes();
        let commitment_lo = u64::from_le_bytes(digest_bytes[0..8].try_into().unwrap());
        let commitment_hi = u64::from_le_bytes(digest_bytes[8..16].try_into().unwrap());
        if commitment_lo != reference.commitment_lo || commitment_hi != reference.commitment_hi {
            return Err(HeError::CommitmentMismatch);
        }
        let inner =
            Ciphertext::from_bytes(bytes, &self.parameters).map_err(|_| HeError::Backend)?;
        Ok(BfvCiphertext { inner, reference })
    }

    fn wrap(
        &self,
        store_id: u64,
        slot_count: usize,
        inner: Ciphertext,
    ) -> Result<BfvCiphertext, HeError> {
        validate_slot_count(slot_count)?;
        let serialized = inner.to_bytes();
        let digest = blake3::hash(&serialized);
        let digest_bytes = digest.as_bytes();
        let commitment_lo = u64::from_le_bytes(digest_bytes[0..8].try_into().unwrap());
        let commitment_hi = u64::from_le_bytes(digest_bytes[8..16].try_into().unwrap());
        Ok(BfvCiphertext {
            inner,
            reference: HeCiphertextRef::new(
                store_id,
                self.key_id_hash,
                self.parameter_fingerprint,
                commitment_lo,
                commitment_hi,
                slot_count as u32,
                0,
            ),
        })
    }

    fn validate_values(&self, values: &[i64]) -> Result<(), HeError> {
        validate_slot_count(values.len())?;
        let limit = self.signed_plaintext_limit();
        if values
            .iter()
            .any(|value| *value <= -limit || *value >= limit)
        {
            return Err(HeError::PlaintextOutOfRange);
        }
        Ok(())
    }

    fn validate_ciphertext(&self, ciphertext: &BfvCiphertext) -> Result<(), HeError> {
        let reference = ciphertext.reference;
        if reference.key_id_hash != self.key_id_hash
            || reference.parameter_fingerprint != self.parameter_fingerprint
        {
            return Err(HeError::CiphertextMismatch);
        }
        Ok(())
    }

    fn validate_pair(&self, left: &BfvCiphertext, right: &BfvCiphertext) -> Result<(), HeError> {
        self.validate_ciphertext(left)?;
        self.validate_ciphertext(right)?;
        if left.reference.slot_count() != right.reference.slot_count() {
            return Err(HeError::CiphertextMismatch);
        }
        Ok(())
    }
}

/// Quantize finite floating-point values into a caller-owned fixed-point buffer.
pub fn encode_fixed_point_into(
    input: &[f64],
    scale: i64,
    signed_plaintext_limit: i64,
    out: &mut [i64],
) -> Result<usize, HeError> {
    if scale <= 0 {
        return Err(HeError::InvalidScale);
    }
    if out.len() < input.len() {
        return Err(HeError::OutputBufferTooSmall);
    }
    for (destination, value) in out.iter_mut().zip(input) {
        if !value.is_finite() {
            return Err(HeError::NonFiniteInput);
        }
        let scaled = (*value * scale as f64).round();
        if scaled <= -(signed_plaintext_limit as f64) || scaled >= signed_plaintext_limit as f64 {
            return Err(HeError::PlaintextOutOfRange);
        }
        *destination = scaled as i64;
    }
    Ok(input.len())
}

/// Decode fixed-point integers into a caller-owned floating-point buffer.
pub fn decode_fixed_point_into(
    input: &[i64],
    scale: i64,
    out: &mut [f64],
) -> Result<usize, HeError> {
    if scale <= 0 {
        return Err(HeError::InvalidScale);
    }
    if out.len() < input.len() {
        return Err(HeError::OutputBufferTooSmall);
    }
    for (destination, value) in out.iter_mut().zip(input) {
        *destination = *value as f64 / scale as f64;
    }
    Ok(input.len())
}

fn validate_slot_count(slot_count: usize) -> Result<(), HeError> {
    if slot_count == 0 {
        Err(HeError::EmptyInput)
    } else if slot_count > MAX_BFV_PACKED_SLOTS {
        Err(HeError::TooManySlots)
    } else {
        Ok(())
    }
}

fn fingerprint_parameters(parameters: &BfvParameters) -> u64 {
    let mut hash = blake3::Hasher::new();
    hash.update(&(parameters.degree() as u64).to_le_bytes());
    hash.update(&parameters.plaintext().to_le_bytes());
    for modulus in parameters.moduli() {
        hash.update(&modulus.to_le_bytes());
    }
    u64::from_le_bytes(hash.finalize().as_bytes()[..8].try_into().unwrap())
}

#[cfg(test)]
mod tests;
