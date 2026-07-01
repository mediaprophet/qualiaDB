# Linear-Algebra Privacy Engine

The linear-algebra privacy engine combines exact packed BFV homomorphic arithmetic
with calibrated differential-privacy releases. It replaces the former capability-only
stub in `specialized_libs/linear_algebra/privacy.rs`.

## Homomorphic encryption

The `privacy-he` feature (enabled by default on native builds) uses the pure-Rust
`fhe` crate's leveled BFV Ring-LWE implementation.

- Production construction uses `BfvEngine::generate_128_bit`.
- The selected standard parameter set has polynomial degree 4096 and a roughly
  20-bit SIMD-compatible plaintext modulus.
- Implemented encrypted operations are packed addition, packed element-wise
  multiplication with relinearization, and a rotation-based packed dot product.
- Signed integer arithmetic is exact modulo the plaintext modulus. Callers must use
  `signed_plaintext_limit()` to prevent wraparound.
- `encode_fixed_point_into` and `decode_fixed_point_into` provide allocation-free
  fixed-point conversion. Addition preserves the scale; multiplication produces
  scale squared.
- Serialized parameter and key material is rejected if it exceeds 42 MiB.

The upstream `fhe` crate states that it has not been independently audited. This is a
real BFV implementation, not a simulation, but it must not be represented as audited,
FIPS-validated, or ready for high-risk production deployment without an independent
cryptographic review.

## 48-byte ABI boundary

BFV ciphertexts are much larger than an `NQuin` and are never embedded in one.
`HeCiphertextRef` is a separate 48-byte, eight-byte-aligned reference containing:

1. external store identifier;
2. hashed key identifier;
3. parameter fingerprint;
4. a 128-bit BLAKE3 ciphertext commitment;
5. packed slot count, scheme, and level.

The commitment detects accidental substitution; it is not an authorization token.
Ciphertext bytes live in caller-owned bounded storage through `serialize_into`.
`deserialize_verified` checks the commitment before parsing stored bytes.

## Differential privacy

`DifferentialPrivacy` implements:

- Laplace noise with scale `L1 sensitivity / epsilon` for pure epsilon-DP;
- the classic Gaussian calibration
  `sigma = L2 sensitivity * sqrt(2 ln(1.25 / delta)) / epsilon`, restricted to
  `0 < epsilon <= 1`;
- basic sequential composition, generalized advanced composition, and fixed-order
  Rényi-DP accounting for Gaussian releases;
- fail-closed budget checks before a result is released;
- operating-system cryptographic entropy on the normal public release methods.

All release loops write into caller-provided `&mut [f64]` buffers. A single charge
covers the entire vector, so the caller must supply the sensitivity of the
vector-valued query, not the sensitivity of one coordinate. Custom `NoiseSource`
implementations are intended for deterministic testing; a non-CSPRNG source voids
the privacy guarantee.

## Feature boundary

DP and the fixed-capacity key metadata registry compile without `privacy-he`. The
large BFV dependency is feature-gated so slim/no-default WASM profiles can omit it.
Key generation is explicit and is never performed by `PrivacyEngine::new()`.
