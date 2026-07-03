//! Zero-Knowledge **predicate (threshold / range) proofs** for the disclosure
//! model's `PropertyProof` modality.
//!
//! This module lets a holder prove that a *private* value satisfies a *public*
//! bound WITHOUT revealing the value — e.g. "age ≥ 18", "balance ≥ min",
//! "score ∈ [lo, hi]". The proofs are **real Groth16 over BLS12-381** (arkworks
//! 0.6), built on genuine R1CS constraints (bit-decomposition range checks), not
//! a hash commitment. It is a sibling of [`crate::crypto::zk_proofs`] and reuses
//! the same curve, RNG ([`crate::zk_proofs::zk_secure_rng`]), and serialization
//! conventions (`CanonicalSerialize` / compressed bytes).
//!
//! # How soundness is enforced (bit-decomposition)
//!
//! To prove `value >= threshold` for `value, threshold` in a fixed bit-width
//! `N = 64`, the circuit introduces a **private** witness `diff = value - threshold`
//! and enforces, over the BLS12-381 scalar field `Fr`:
//!
//! 1. `value == threshold + diff` (the definition of `diff`);
//! 2. `diff == Σ_{i<N} b_i · 2^i`, where each `b_i` is a **boolean** witness
//!    (`b_i · (b_i − 1) == 0`).
//!
//! Constraint (2) proves `diff` is a non-negative `N`-bit integer, i.e.
//! `0 <= diff < 2^N`. Combined with (1) that gives `value = threshold + diff >=
//! threshold`. If instead `value < threshold`, then over the field `diff`
//! evaluates to `value − threshold ≡ p − (threshold − value)` (a number of order
//! the field modulus `p ≈ 2^255`), which **cannot** be written as an `N`-bit sum
//! for `N = 64` — no boolean assignment `b_i` satisfies (2). Hence there is **no
//! satisfying witness** and the honest prover simply cannot produce a proof: the
//! `< threshold` case is *unprovable*, not merely rejected at verify time.
//!
//! The **range** predicate `lo <= value <= hi` composes two such checks in one
//! circuit: `value − lo` is a non-negative `N`-bit integer AND `hi − value` is a
//! non-negative `N`-bit integer.
//!
//! # Trusted setup model (honest limitation)
//!
//! Groth16 requires a per-circuit trusted setup (a structured reference string).
//! Like [`crate::crypto::zk_proofs`] and [`crate::crypto::deontic_circuit`], this
//! module performs a **per-statement `circuit_specific_setup`**: [`prove_threshold`]
//! / [`prove_range`] run setup, prove, and bundle the verifying key *with* the
//! proof ([`PredicateProof`]) so a verifier can check it standalone. The circuit
//! *shape* is fixed (it depends only on `N`, never on the secret value), so the
//! toxic-waste randomness of setup is the only trust assumption; the setup is not
//! specialised to the secret. For a production deployment the VK for each
//! predicate width would be generated once by a ceremony and pinned — factoring
//! that ceremony out is a deployment concern, not a soundness gap in the circuit.
//! The `verify_*` entry points take the public bound plus the [`PredicateProof`]
//! (which carries the VK produced by that statement's setup); they deserialize
//! that bundled VK and check the proof against the supplied public input. Pinning
//! a single ceremony-generated VK per predicate width, and rejecting proofs that
//! ship any other VK, is the production hardening on top of this milestone.

#![cfg(feature = "zk-culling")]

use ark_bls12_381::{Bls12_381, Fr};
use ark_ff::{BigInteger, One, PrimeField, Zero};
use ark_groth16::{Groth16, Proof, ProvingKey, VerifyingKey};
use ark_relations::gr1cs::{
    ConstraintSynthesizer, ConstraintSystem, ConstraintSystemRef, LinearCombination,
    SynthesisError, Variable,
};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;

/// Fixed bit-width for the range / threshold checks. Values and bounds must fit
/// in `u64`; the bit-decomposition proves `0 <= diff < 2^64`.
pub const PREDICATE_BITS: usize = 64;

// ── Public statements ──────────────────────────────────────────────────────

/// Public statement for a threshold predicate: the prover asserts knowledge of a
/// private `value` with `value >= threshold`. `threshold` is the sole public input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThresholdStatement {
    pub threshold: u64,
}

/// Public statement for a range predicate: the prover asserts knowledge of a
/// private `value` with `lo <= value <= hi`. `lo` and `hi` are the public inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RangeStatement {
    pub lo: u64,
    pub hi: u64,
}

// ── Proof container ────────────────────────────────────────────────────────

/// A self-contained predicate proof: the compressed Groth16 proof plus the
/// verifying key it was produced under (per-statement setup, see module docs).
/// A verifier with only this struct and the public statement can check validity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredicateProof {
    /// Compressed arkworks `Proof<Bls12_381>` bytes.
    pub proof: Vec<u8>,
    /// Compressed arkworks `VerifyingKey<Bls12_381>` bytes.
    pub vk: Vec<u8>,
}

impl PredicateProof {
    /// Total serialized size in bytes (proof + VK), useful for telemetry.
    pub fn size_bytes(&self) -> usize {
        self.proof.len() + self.vk.len()
    }
}

// ── Threshold circuit ──────────────────────────────────────────────────────

/// R1CS circuit proving `value >= threshold` via `N`-bit decomposition of
/// `diff = value - threshold`. `value` and the bits are private; `threshold` is
/// the single public input.
#[derive(Clone)]
struct ThresholdCircuit {
    /// Private secret. `None` at setup time (shape only).
    value: Option<u64>,
    /// Public bound. `None` at setup time.
    threshold: Option<u64>,
}

impl ConstraintSynthesizer<Fr> for ThresholdCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Public input: threshold.
        let threshold_var = cs.new_input_variable(|| {
            self.threshold
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        // Private witness: the secret value.
        let value_var = cs.new_witness_variable(|| {
            self.value
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;

        // Enforce value >= threshold, binding the proof to the public threshold.
        enforce_geq(
            cs,
            Operand { val: self.value, var: value_var },
            Operand { val: self.threshold, var: threshold_var },
        )?;
        Ok(())
    }
}

// ── Range circuit ──────────────────────────────────────────────────────────

/// R1CS circuit proving `lo <= value <= hi`. Both bounds are public inputs,
/// allocated in the order `[lo, hi]`; `value` is private. Enforced as two
/// independent non-negative `N`-bit differences: `value - lo` and `hi - value`.
#[derive(Clone)]
struct RangeCircuit {
    value: Option<u64>,
    lo: Option<u64>,
    hi: Option<u64>,
}

impl ConstraintSynthesizer<Fr> for RangeCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        // Public inputs in a fixed order: lo, then hi.
        let lo_var = cs.new_input_variable(|| {
            self.lo
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let hi_var = cs.new_input_variable(|| {
            self.hi
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        // Private witness: the secret value, allocated once and shared by both
        // sub-checks so `hi >= value` refers to the *same* value as `value >= lo`.
        let value_var = cs.new_witness_variable(|| {
            self.value
                .map(Fr::from)
                .ok_or(SynthesisError::AssignmentMissing)
        })?;
        let value = Operand { val: self.value, var: value_var };

        // value >= lo  (diff_lo = value - lo is a non-negative N-bit integer).
        enforce_geq(
            cs.clone(),
            value,
            Operand { val: self.lo, var: lo_var },
        )?;
        // hi >= value  (diff_hi = hi - value is a non-negative N-bit integer).
        enforce_geq(
            cs,
            Operand { val: self.hi, var: hi_var },
            value,
        )?;
        Ok(())
    }
}

// ── Shared constraint gadget ───────────────────────────────────────────────

/// One operand of the `>=` gadget: the (optional) `u64` assignment plus the R1CS
/// `Variable` already allocated for it in the constraint system. `Copy` so the
/// range circuit can pass its single `value` operand to both sub-checks.
#[derive(Clone, Copy)]
struct Operand {
    /// The concrete assignment (`None` at setup / shape-only synthesis).
    val: Option<u64>,
    /// The variable already bound to `val` in the constraint system.
    var: Variable,
}

/// Enforce `big >= small`, where **both** operands are already allocated in the
/// constraint system (each carrying its own assignment). Introduces a private
/// `diff = big - small` witness plus its `N`-bit boolean decomposition and
/// enforces:
///
/// * `big == small + diff`  →  `(small.var + diff_var) * 1 == big.var`;
/// * each bit boolean: `b_i * b_i == b_i`;
/// * recomposition: `(Σ b_i·2^i) * 1 == diff_var`.
///
/// Because the bits are constrained boolean and recompose to `diff`, we have
/// `diff ∈ [0, 2^N)`; combined with `big = small + diff` that gives `big >=
/// small`. A satisfying assignment therefore exists **iff** `big >= small` — if
/// `big < small`, the field element `diff = big − small ≡ p − (small − big)` is a
/// ~255-bit number with no `N`-bit boolean decomposition, so no witness
/// satisfies the constraints (the false statement is *unprovable*).
fn enforce_geq(
    cs: ConstraintSystemRef<Fr>,
    big: Operand,
    small: Operand,
) -> Result<(), SynthesisError> {
    // Witness: diff = big - small, computed in the field so it is consistent with
    // the linear constraint below regardless of sign. If big < small the field
    // subtraction yields a value that cannot be N-bit-decomposed, which the
    // boolean + recomposition constraints then reject (no satisfying assignment).
    let diff_var = cs.new_witness_variable(|| match (big.val, small.val) {
        (Some(b), Some(s)) => Ok(Fr::from(b) - Fr::from(s)),
        _ => Err(SynthesisError::AssignmentMissing),
    })?;

    // Constraint: (small + diff) * 1 == big.
    cs.enforce_r1cs_constraint(
        || LinearCombination::from(small.var) + diff_var,
        || LinearCombination::from(Variable::One),
        || LinearCombination::from(big.var),
    )?;

    // N-bit boolean decomposition of diff, recomposed to equal diff_var.
    let mut recomposition = LinearCombination::<Fr>::zero();
    let mut coeff = Fr::one();
    let two = Fr::from(2u64);
    for i in 0..PREDICATE_BITS {
        // Bit i of diff (from the field element's little-endian bits). Because the
        // recomposition constraint pins Σ b_i·2^i == diff, an honest prover MUST
        // supply the true bits; a dishonest one has no alternative assignment.
        let bit_var = cs.new_witness_variable(|| {
            let d = match (big.val, small.val) {
                (Some(b), Some(s)) => Fr::from(b) - Fr::from(s),
                _ => return Err(SynthesisError::AssignmentMissing),
            };
            let bits = d.into_bigint().to_bits_le();
            let b = if i < bits.len() && bits[i] {
                Fr::one()
            } else {
                Fr::zero()
            };
            Ok(b)
        })?;

        // Booleanity: bit * bit == bit  ⟺  bit ∈ {0, 1}.
        cs.enforce_r1cs_constraint(
            || LinearCombination::from(bit_var),
            || LinearCombination::from(bit_var),
            || LinearCombination::from(bit_var),
        )?;

        recomposition = recomposition + (coeff, bit_var);
        coeff *= two;
    }

    // Recomposition: (Σ b_i·2^i) * 1 == diff.
    cs.enforce_r1cs_constraint(
        || recomposition,
        || LinearCombination::from(Variable::One),
        || LinearCombination::from(diff_var),
    )?;

    Ok(())
}

// ── Satisfiability pre-check ───────────────────────────────────────────────

/// Synthesize `circuit` (with its concrete assignment) into a standalone
/// constraint system and report whether every constraint is satisfied.
///
/// This is the honest gate on "the statement is true": a `false` result means no
/// witness satisfies the circuit (e.g. `value < threshold`), so the prover must
/// refuse rather than emit a proof. Running it BEFORE `Groth16::prove` gives a
/// clean `Err` on every build profile — `prove` itself only `debug_assert!`s
/// satisfiability (it panics in debug, and would emit a non-verifying proof in
/// release), so we never rely on that path for correctness.
fn is_satisfied<C: ConstraintSynthesizer<Fr>>(circuit: C) -> Result<bool, String> {
    let cs = ConstraintSystem::<Fr>::new_ref();
    circuit
        .generate_constraints(cs.clone())
        .map_err(|e| format!("constraint synthesis: {e}"))?;
    cs.is_satisfied()
        .map_err(|e| format!("satisfiability check: {e}"))
}

// ── Serialization helpers ──────────────────────────────────────────────────

fn serialize_proof_vk(
    proof: &Proof<Bls12_381>,
    vk: &VerifyingKey<Bls12_381>,
) -> Result<PredicateProof, String> {
    let mut proof_bytes = Vec::new();
    proof
        .serialize_compressed(&mut proof_bytes)
        .map_err(|e| format!("proof serialize: {e}"))?;
    let mut vk_bytes = Vec::new();
    vk.serialize_compressed(&mut vk_bytes)
        .map_err(|e| format!("vk serialize: {e}"))?;
    Ok(PredicateProof {
        proof: proof_bytes,
        vk: vk_bytes,
    })
}

fn deserialize_proof_vk(
    bundle: &PredicateProof,
) -> Result<(Proof<Bls12_381>, VerifyingKey<Bls12_381>), String> {
    let proof = Proof::<Bls12_381>::deserialize_compressed(&bundle.proof[..])
        .map_err(|e| format!("proof deserialize: {e}"))?;
    let vk = VerifyingKey::<Bls12_381>::deserialize_compressed(&bundle.vk[..])
        .map_err(|e| format!("vk deserialize: {e}"))?;
    Ok((proof, vk))
}

// ── Threshold: public API ──────────────────────────────────────────────────

/// Prove, in zero knowledge, that a private `value` satisfies `value >= threshold`.
///
/// Returns a [`PredicateProof`] (compressed Groth16 proof + verifying key). Fails
/// with an `Err` if `value < threshold` — there is no satisfying witness, so the
/// honest prover *cannot* construct a proof (this is the soundness property: a
/// false statement is unprovable, not merely unverifiable).
pub fn prove_threshold(value: u64, threshold: u64) -> Result<PredicateProof, String> {
    let mut rng = crate::zk_proofs::zk_secure_rng();

    // Per-statement setup on the fixed circuit *shape* (no secret bound).
    let setup_circuit = ThresholdCircuit {
        value: None,
        threshold: None,
    };
    let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(setup_circuit, &mut rng)
        .map_err(|e| format!("threshold setup: {e}"))?;

    let circuit = ThresholdCircuit {
        value: Some(value),
        threshold: Some(threshold),
    };
    // Honest gate: refuse to prove a false statement (value < threshold has no
    // satisfying witness — the range check cannot be met).
    if !is_satisfied(circuit.clone())? {
        return Err(format!(
            "value {value} < threshold {threshold}: statement is false and unprovable"
        ));
    }
    let proof = Groth16::<Bls12_381>::prove(&pk, circuit, &mut rng)
        .map_err(|e| format!("threshold prove: {e}"))?;

    serialize_proof_vk(&proof, &vk)
}

/// Verify a [`PredicateProof`] produced by [`prove_threshold`] against a public
/// `threshold`. Returns `true` iff the proof is valid *for that exact threshold*
/// — a proof made for one threshold does not verify against a different one.
///
/// The proof carries its own verifying key (per-statement setup), so no external
/// VK is needed. `threshold` is supplied here as the public input.
pub fn verify_threshold(proof: &PredicateProof, threshold: u64) -> bool {
    let (ark_proof, vk) = match deserialize_proof_vk(proof) {
        Ok(pv) => pv,
        Err(_) => return false,
    };
    let public_inputs = [Fr::from(threshold)];
    Groth16::<Bls12_381>::verify(&vk, &public_inputs, &ark_proof).unwrap_or(false)
}

// ── Range: public API ──────────────────────────────────────────────────────

/// Prove, in zero knowledge, that a private `value` satisfies `lo <= value <= hi`.
///
/// Fails with `Err` if `value < lo` or `value > hi` (no satisfying witness), and
/// if `lo > hi` (an empty range is unprovable for any value).
pub fn prove_range(value: u64, lo: u64, hi: u64) -> Result<PredicateProof, String> {
    let mut rng = crate::zk_proofs::zk_secure_rng();

    let setup_circuit = RangeCircuit {
        value: None,
        lo: None,
        hi: None,
    };
    let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(setup_circuit, &mut rng)
        .map_err(|e| format!("range setup: {e}"))?;

    let circuit = RangeCircuit {
        value: Some(value),
        lo: Some(lo),
        hi: Some(hi),
    };
    // Honest gate: refuse to prove a false statement. `value < lo`, `value > hi`,
    // or an empty interval `lo > hi` all leave one of the two N-bit range checks
    // unsatisfiable.
    if !is_satisfied(circuit.clone())? {
        return Err(format!(
            "value {value} outside [{lo}, {hi}] (or empty interval): statement is false and unprovable"
        ));
    }
    let proof = Groth16::<Bls12_381>::prove(&pk, circuit, &mut rng)
        .map_err(|e| format!("range prove: {e}"))?;

    serialize_proof_vk(&proof, &vk)
}

/// Verify a [`PredicateProof`] produced by [`prove_range`] against public bounds
/// `lo` and `hi`. Returns `true` iff valid for that exact `[lo, hi]` pair. Public
/// inputs are supplied in the circuit's allocation order `[lo, hi]`.
pub fn verify_range(proof: &PredicateProof, lo: u64, hi: u64) -> bool {
    let (ark_proof, vk) = match deserialize_proof_vk(proof) {
        Ok(pv) => pv,
        Err(_) => return false,
    };
    let public_inputs = [Fr::from(lo), Fr::from(hi)];
    Groth16::<Bls12_381>::verify(&vk, &public_inputs, &ark_proof).unwrap_or(false)
}

// ── Convenience wrappers over the statement structs ────────────────────────

impl ThresholdStatement {
    /// Prove a private `value` satisfies this statement (`value >= self.threshold`).
    pub fn prove(&self, value: u64) -> Result<PredicateProof, String> {
        prove_threshold(value, self.threshold)
    }
    /// Verify a proof against this statement's public threshold.
    pub fn verify(&self, proof: &PredicateProof) -> bool {
        verify_threshold(proof, self.threshold)
    }
}

impl RangeStatement {
    /// Prove a private `value` satisfies this statement (`self.lo <= value <= self.hi`).
    pub fn prove(&self, value: u64) -> Result<PredicateProof, String> {
        prove_range(value, self.lo, self.hi)
    }
    /// Verify a proof against this statement's public bounds.
    pub fn verify(&self, proof: &PredicateProof) -> bool {
        verify_range(proof, self.lo, self.hi)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ---- Threshold: correctness ----

    #[test]
    fn threshold_above_verifies() {
        // 21 >= 18 must verify.
        let proof = prove_threshold(21, 18).expect("21 >= 18 is provable");
        assert!(verify_threshold(&proof, 18), "21 >= 18 must verify");
    }

    #[test]
    fn threshold_boundary_verifies() {
        // Boundary: 18 >= 18 must verify (diff = 0, all bits zero).
        let proof = prove_threshold(18, 18).expect("18 >= 18 is provable");
        assert!(verify_threshold(&proof, 18), "18 >= 18 (boundary) must verify");
    }

    #[test]
    fn threshold_zero_bound_verifies() {
        // Any value >= 0.
        let proof = prove_threshold(0, 0).expect("0 >= 0 is provable");
        assert!(verify_threshold(&proof, 0));
    }

    // ---- Threshold: SOUNDNESS ----

    #[test]
    fn threshold_below_is_unprovable() {
        // SOUNDNESS: 17 >= 18 is FALSE. The honest prover must NOT be able to
        // construct a proof — diff = 17 - 18 wraps to a ~255-bit field element
        // that has no 64-bit boolean decomposition, so no satisfying witness
        // exists and proving returns Err. The false statement is *unprovable*.
        let result = prove_threshold(17, 18);
        assert!(
            result.is_err(),
            "17 >= 18 is false and MUST be unprovable (honest prover cannot cheat), got Ok"
        );
    }

    #[test]
    fn threshold_far_below_is_unprovable() {
        // A larger gap, same property: value strictly below threshold is unprovable.
        assert!(
            prove_threshold(0, 1).is_err(),
            "0 >= 1 is false and MUST be unprovable"
        );
        assert!(
            prove_threshold(100, 1_000_000).is_err(),
            "100 >= 1_000_000 is false and MUST be unprovable"
        );
    }

    #[test]
    fn threshold_proof_does_not_verify_against_different_public_threshold() {
        // SOUNDNESS (public-input binding): a proof made for threshold = 18 must
        // NOT verify when checked against a different public threshold = 50.
        let proof = prove_threshold(21, 18).expect("21 >= 18 is provable");
        assert!(verify_threshold(&proof, 18), "must verify against its own threshold");
        assert!(
            !verify_threshold(&proof, 50),
            "a proof for threshold=18 must NOT verify against threshold=50"
        );
        assert!(
            !verify_threshold(&proof, 17),
            "a proof for threshold=18 must NOT verify against threshold=17"
        );
    }

    // ---- Range: correctness ----

    #[test]
    fn range_in_range_verifies() {
        // 42 ∈ [18, 65].
        let proof = prove_range(42, 18, 65).expect("42 in [18,65] is provable");
        assert!(verify_range(&proof, 18, 65), "42 in [18,65] must verify");
    }

    #[test]
    fn range_boundaries_verify() {
        // Both endpoints are inclusive.
        let lo_proof = prove_range(18, 18, 65).expect("lo boundary provable");
        assert!(verify_range(&lo_proof, 18, 65), "value == lo must verify");
        let hi_proof = prove_range(65, 18, 65).expect("hi boundary provable");
        assert!(verify_range(&hi_proof, 18, 65), "value == hi must verify");
    }

    // ---- Range: SOUNDNESS ----

    #[test]
    fn range_below_lo_is_unprovable() {
        // 17 ∉ [18, 65] (below lo) → unprovable.
        assert!(
            prove_range(17, 18, 65).is_err(),
            "17 < 18 (below lo) MUST be unprovable"
        );
    }

    #[test]
    fn range_above_hi_is_unprovable() {
        // 66 ∉ [18, 65] (above hi) → unprovable.
        assert!(
            prove_range(66, 18, 65).is_err(),
            "66 > 65 (above hi) MUST be unprovable"
        );
    }

    #[test]
    fn range_empty_interval_is_unprovable() {
        // lo > hi is an empty interval; no value can satisfy it.
        assert!(
            prove_range(50, 65, 18).is_err(),
            "empty interval [65,18] MUST be unprovable for any value"
        );
    }

    #[test]
    fn range_proof_does_not_verify_against_wrong_bounds() {
        // SOUNDNESS (public-input binding): a proof for [18,65] must not verify
        // against different public bounds.
        let proof = prove_range(42, 18, 65).expect("42 in [18,65] is provable");
        assert!(verify_range(&proof, 18, 65), "must verify against its own bounds");
        assert!(
            !verify_range(&proof, 43, 65),
            "a proof for [18,65] must NOT verify against [43,65]"
        );
        assert!(
            !verify_range(&proof, 18, 41),
            "a proof for [18,65] must NOT verify against [18,41]"
        );
        assert!(
            !verify_range(&proof, 0, 100),
            "a proof for [18,65] must NOT verify against [0,100]"
        );
    }

    // ---- Statement-struct wrappers ----

    #[test]
    fn statement_wrappers_roundtrip() {
        let ts = ThresholdStatement { threshold: 18 };
        let p = ts.prove(21).expect("provable");
        assert!(ts.verify(&p));
        assert!(!ThresholdStatement { threshold: 99 }.verify(&p));

        let rs = RangeStatement { lo: 18, hi: 65 };
        let p = rs.prove(42).expect("provable");
        assert!(rs.verify(&p));
        assert!(!RangeStatement { lo: 43, hi: 65 }.verify(&p));
    }

    #[test]
    fn proof_has_nontrivial_size() {
        // Sanity: a real Groth16 proof + VK is a few hundred bytes, not empty.
        let proof = prove_threshold(21, 18).unwrap();
        assert!(proof.size_bytes() > 100, "real proof+vk must be non-trivial");
    }

    // ---- Integrator adversarial checks (independent of the authoring agent) ----

    #[test]
    fn tampered_proof_is_rejected() {
        // Flipping a proof byte must fail verification (not silently accept).
        let mut proof = prove_threshold(21, 18).unwrap();
        proof.proof[0] ^= 0xFF;
        assert!(!verify_threshold(&proof, 18), "a tampered proof must not verify");
    }

    #[test]
    fn proof_binds_the_exact_threshold_not_merely_a_satisfiable_one() {
        // 100 clears BOTH >=50 and >=60, but a proof MADE for 50 must not verify
        // against 60 — the proof binds the exact public threshold, not "any bound
        // the secret happens to satisfy". This is the property a discloser relies on.
        let proof = prove_threshold(100, 50).unwrap();
        assert!(verify_threshold(&proof, 50));
        assert!(
            !verify_threshold(&proof, 60),
            "proof for threshold=50 must NOT verify against threshold=60"
        );
    }

    #[test]
    fn max_value_threshold_holds() {
        // Edge: u64::MAX clears any threshold; boundary at the top of the width.
        let proof = prove_threshold(u64::MAX, u64::MAX).unwrap();
        assert!(verify_threshold(&proof, u64::MAX));
    }
}
