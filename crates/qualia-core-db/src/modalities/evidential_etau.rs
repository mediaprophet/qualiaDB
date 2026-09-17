//! Paraconsistent Eτ Evidential Logic & W3C VCs — plan §7.3 A7.
//!
//! Extends the existing paraconsistent logic (`paraconsistent.rs`) with
//! evidential (μ, λ) packing and W3C Verifiable Credential artifact outputs.
//!
//! ## Evidential (μ, λ) packing
//!
//! Per plan review note 1, evidential data is packed as two f16 values into
//! the NQuin metadata modality payload area [0..31] (16 bits each = 32 bits),
//! preserving the Lamport clock [32..60] and routing lane [61..62].
//!
//! - **μ (mu)**: evidential support — degree of positive evidence [0, 1].
//! - **λ (lambda)**: evidential refutation — degree of negative evidence [0, 1].
//! - **G (combined)**: paraconsistent truth value = μ - λ, in [-1, 1].
//!   G ≥ 0.5 → route to quarantine (contradiction), G ≤ -0.5 → reject.
//!
//! ## Opcodes
//!
//! Extends the paraconsistent opcode range (0x30-0x32 in `paraconsistent.rs`):
//! - `OP_EVIDENTIAL_PACK` (0x33): pack (μ, λ) into a Quin's metadata.
//! - `OP_EVIDENTIAL_SCORE` (0x34): compute G from a Quin's packed (μ, λ).
//! - `OP_EVIDENTIAL_VC` (0x35): emit a W3C Verifiable Credential artifact
//!   containing the evidential claim.
//!
//! ## W3C VC integration
//!
//! Evidential claims can be sealed into `crypto::verifiable_credential::Credential`
//! artifacts, providing tamper-evident attestation of evidential assessments.

use crate::modalities::paraconsistent::{global_saturation, is_saturated, Belnap};
use crate::NQuin;

// ── Opcodes ────────────────────────────────────────────────────────────────

/// Pack (μ, λ) evidential values into a Quin's metadata.
pub const OP_EVIDENTIAL_PACK: u8 = 0x33;
/// Compute the evidential score G from packed (μ, λ).
pub const OP_EVIDENTIAL_SCORE: u8 = 0x34;
/// Emit a W3C Verifiable Credential artifact for an evidential claim.
pub const OP_EVIDENTIAL_VC: u8 = 0x35;

// ── f16 conversion ─────────────────────────────────────────────────────────

/// Convert an f32 to a 16-bit half-precision float (IEEE 754 binary16).
/// This is a pure-Rust implementation with no external dependency.
pub fn f32_to_f16(f: f32) -> u16 {
    let bits = f.to_bits();
    let sign = ((bits >> 16) & 0x8000) as u16;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let mant = bits & 0x7FFFFF;

    if exp == 0xFF {
        // Inf or NaN
        if mant == 0 {
            return sign | 0x7C00; // Inf
        }
        return sign | 0x7C00 | ((mant >> 13) as u16 | 0x0200); // NaN
    }

    let new_exp = exp - 127 + 15;
    if new_exp >= 0x1F {
        // Overflow → Inf
        return sign | 0x7C00;
    }
    if new_exp <= 0 {
        // Subnormal or zero
        if new_exp < -10 {
            return sign; // Too small → zero
        }
        let mant = mant | 0x800000;
        let shift = 14 - new_exp;
        let hmant = (mant >> shift) as u16;
        // Round to nearest even
        let rem = mant & ((1 << shift) - 1);
        let half = 1 << (shift - 1);
        if rem > half || (rem == half && hmant & 1 != 0) {
            return sign | (hmant + 1);
        }
        return sign | hmant;
    }

    // Normalized
    let hmant = (mant >> 13) as u16;
    let rem = mant & 0x1FFF;
    let half = 0x1000;
    if rem > half || (rem == half && hmant & 1 != 0) {
        let rounded = hmant + 1;
        if rounded >= 0x400 {
            // Rounding caused mantissa overflow → exp++
            return sign | (((new_exp as u16) + 1) << 10);
        }
        return sign | ((new_exp as u16) << 10) | rounded;
    }
    sign | ((new_exp as u16) << 10) | hmant
}

/// Convert a 16-bit half-precision float to f32.
pub fn f16_to_f32(h: u16) -> f32 {
    let sign = ((h & 0x8000) as u32) << 16;
    let exp = ((h >> 10) & 0x1F) as i32;
    let mant = (h & 0x3FF) as u32;

    if exp == 0 {
        if mant == 0 {
            return f32::from_bits(sign);
        }
        // Subnormal
        let mut e = -1;
        let mut m = mant;
        while m & 0x400 == 0 {
            m <<= 1;
            e -= 1;
        }
        m &= 0x3FF;
        let new_exp = (127 + e - 14) as u32;
        return f32::from_bits(sign | (new_exp << 23) | (m << 13));
    }

    if exp == 0x1F {
        if mant == 0 {
            return f32::from_bits(sign | 0x7F800000); // Inf
        }
        return f32::from_bits(sign | 0x7F800000 | (mant << 13)); // NaN
    }

    let new_exp = (exp + 127 - 15) as u32;
    f32::from_bits(sign | (new_exp << 23) | (mant << 13))
}

// ── Evidential values ──────────────────────────────────────────────────────

/// Evidential (μ, λ) pair — support and refutation degrees.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EvidentialPair {
    /// μ (mu) — degree of positive evidence [0, 1].
    pub mu: f32,
    /// λ (lambda) — degree of negative evidence [0, 1].
    pub lambda: f32,
}

impl EvidentialPair {
    pub fn new(mu: f32, lambda: f32) -> Self {
        Self {
            mu: mu.clamp(0.0, 1.0),
            lambda: lambda.clamp(0.0, 1.0),
        }
    }

    /// Combined evidential score G = μ - λ, in [-1, 1].
    pub fn g(&self) -> f32 {
        self.mu - self.lambda
    }

    /// Is this a strong positive claim? (G ≥ 0.5)
    pub fn is_strong_positive(&self) -> bool {
        self.g() >= 0.5
    }

    /// Is this a strong negative claim? (G ≤ -0.5)
    pub fn is_strong_negative(&self) -> bool {
        self.g() <= -0.5
    }

    /// Is this a contradiction? (both μ ≥ 0.5 and λ ≥ 0.5)
    pub fn is_contradiction(&self) -> bool {
        self.mu >= 0.5 && self.lambda >= 0.5
    }

    /// Convert to Belnap four-valued logic.
    pub fn to_belnap(&self) -> Belnap {
        Belnap::from_evidence(self.mu >= 0.5, self.lambda >= 0.5)
    }

    /// Pack into 32 bits: μ as f16 in [0..15], λ as f16 in [16..31].
    pub fn pack(&self) -> u32 {
        let mu_h = f32_to_f16(self.mu) as u32;
        let lambda_h = f32_to_f16(self.lambda) as u32;
        mu_h | (lambda_h << 16)
    }

    /// Unpack from 32 bits: μ from [0..15], λ from [16..31].
    pub fn unpack(packed: u32) -> Self {
        let mu = f16_to_f32((packed & 0xFFFF) as u16);
        let lambda = f16_to_f32((packed >> 16) as u16);
        Self { mu, lambda }
    }

    /// Pack into an NQuin's metadata modality payload area [0..31],
    /// preserving the Lamport clock [32..60] and routing lane [61..62].
    pub fn pack_into_metadata(&self, metadata: u64) -> u64 {
        let packed = self.pack() as u64;
        (metadata & !0xFFFFFFFF) | packed
    }

    /// Unpack from an NQuin's metadata modality payload area [0..31].
    pub fn unpack_from_metadata(metadata: u64) -> Self {
        Self::unpack((metadata & 0xFFFFFFFF) as u32)
    }
}

impl Default for EvidentialPair {
    fn default() -> Self {
        Self {
            mu: 0.0,
            lambda: 0.0,
        }
    }
}

// ── Evidential Quin operations ─────────────────────────────────────────────

/// Pack (μ, λ) evidential values into a Quin's metadata.
/// The opcode is set in the predicate's low byte.
pub fn pack_evidential(mut quin: NQuin, mu: f32, lambda: f32) -> NQuin {
    let pair = EvidentialPair::new(mu, lambda);
    quin.metadata = pair.pack_into_metadata(quin.metadata);
    // Set the opcode in the predicate's low byte.
    quin.predicate = (quin.predicate & !0xFF) | OP_EVIDENTIAL_PACK as u64;
    // Recompute parity.
    quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context ^ quin.metadata;
    quin
}

/// Extract the evidential (μ, λ) pair from a Quin's metadata.
pub fn extract_evidential(quin: &NQuin) -> EvidentialPair {
    EvidentialPair::unpack_from_metadata(quin.metadata)
}

/// Compute the evidential score G from a Quin's packed (μ, λ).
pub fn evidential_score(quin: &NQuin) -> f32 {
    extract_evidential(quin).g()
}

/// Route an evidential Quin based on its G score:
/// - G ≥ 0.5 → quarantine (strong positive, needs verification)
/// - G ≤ -0.5 → reject (strong negative)
/// - |G| < 0.5 → normal (insufficient evidence either way)
/// - μ ≥ 0.5 and λ ≥ 0.5 → contradiction (route to isolated context)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidentialRoute {
    /// Normal — insufficient evidence either way.
    Normal,
    /// Quarantine — strong positive claim, needs verification.
    Quarantine,
    /// Reject — strong negative claim.
    Reject,
    /// Contradiction — both positive and negative evidence ≥ 0.5.
    Contradiction,
}

/// Determine the routing for an evidential Quin.
pub fn route_evidential(quin: &NQuin) -> EvidentialRoute {
    let pair = extract_evidential(quin);
    if pair.is_contradiction() {
        EvidentialRoute::Contradiction
    } else if pair.is_strong_positive() {
        EvidentialRoute::Quarantine
    } else if pair.is_strong_negative() {
        EvidentialRoute::Reject
    } else {
        EvidentialRoute::Normal
    }
}

/// Route a batch of evidential Quins into normal, quarantine, reject, and
/// contradiction buckets. Returns counts for each.
pub fn route_evidential_batch(
    quins: &[NQuin],
    out_normal: &mut [NQuin],
    out_quarantine: &mut [NQuin],
    out_reject: &mut [NQuin],
    out_contradiction: &mut [NQuin],
) -> Result<(usize, usize, usize, usize), EvidentialError> {
    let mut n = 0;
    let mut q = 0;
    let mut r = 0;
    let mut c = 0;
    for quin in quins {
        match route_evidential(quin) {
            EvidentialRoute::Normal => {
                if n >= out_normal.len() {
                    return Err(EvidentialError::BufferOverflow);
                }
                out_normal[n] = *quin;
                n += 1;
            }
            EvidentialRoute::Quarantine => {
                if q >= out_quarantine.len() {
                    return Err(EvidentialError::BufferOverflow);
                }
                out_quarantine[q] = *quin;
                q += 1;
            }
            EvidentialRoute::Reject => {
                if r >= out_reject.len() {
                    return Err(EvidentialError::BufferOverflow);
                }
                out_reject[r] = *quin;
                r += 1;
            }
            EvidentialRoute::Contradiction => {
                if c >= out_contradiction.len() {
                    return Err(EvidentialError::BufferOverflow);
                }
                out_contradiction[c] = *quin;
                c += 1;
            }
        }
    }
    Ok((n, q, r, c))
}

#[derive(Debug)]
pub enum EvidentialError {
    BufferOverflow,
}

// ── Evidential saturation ──────────────────────────────────────────────────

/// Evidential saturation: the fraction of Quins that are contradictions
/// or in quarantine. Uses the existing `global_saturation` function.
pub fn evidential_saturation(normal: usize, flagged: usize) -> f32 {
    global_saturation(normal, flagged)
}

/// Check if evidential saturation exceeds a threshold.
pub fn is_evidentially_saturated(normal: usize, flagged: usize, threshold: f32) -> bool {
    is_saturated(evidential_saturation(normal, flagged), threshold)
}

// ── W3C VC integration ─────────────────────────────────────────────────────

/// An evidential claim suitable for sealing into a W3C Verifiable Credential.
#[derive(Debug, Clone, PartialEq)]
pub struct EvidentialClaim {
    /// The subject of the claim (agent/entity DID hash).
    pub subject: u64,
    /// The predicate (property) being attested.
    pub predicate: u64,
    /// The object (value) being attested.
    pub object: u64,
    /// The context (graph) of the claim.
    pub context: u64,
    /// The evidential (μ, λ) pair.
    pub evidence: EvidentialPair,
    /// The route determined for this claim.
    pub route: EvidentialRoute,
}

impl EvidentialClaim {
    /// Create an evidential claim from a Quin.
    pub fn from_quin(quin: &NQuin) -> Self {
        let evidence = extract_evidential(quin);
        let route = route_evidential(quin);
        Self {
            subject: quin.subject,
            predicate: quin.predicate >> 8, // Strip opcode
            object: quin.object,
            context: quin.context,
            evidence,
            route,
        }
    }

    /// Convert to an NQuin suitable for inclusion in a Credential's claims.
    pub fn to_claim_quin(&self) -> NQuin {
        let mut q = NQuin {
            subject: self.subject,
            predicate: (self.predicate << 8) | OP_EVIDENTIAL_VC as u64,
            object: self.object,
            context: self.context,
            metadata: self.evidence.pack_into_metadata(0),
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata;
        q
    }

    /// Human-readable description.
    pub fn describe(&self) -> String {
        let route_str = match self.route {
            EvidentialRoute::Normal => "normal",
            EvidentialRoute::Quarantine => "quarantine",
            EvidentialRoute::Reject => "reject",
            EvidentialRoute::Contradiction => "contradiction",
        };
        format!(
            "evidential claim: subject={} pred={} obj={} ctx={} μ={:.3} λ={:.3} G={:.3} route={}",
            self.subject,
            self.predicate,
            self.object,
            self.context,
            self.evidence.mu,
            self.evidence.lambda,
            self.evidence.g(),
            route_str
        )
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f16_roundtrip_zero() {
        assert_eq!(f16_to_f32(f32_to_f16(0.0)), 0.0);
        assert_eq!(f16_to_f32(f32_to_f16(-0.0)), -0.0);
    }

    #[test]
    fn f16_roundtrip_one() {
        let h = f32_to_f16(1.0);
        let f = f16_to_f32(h);
        assert!((f - 1.0).abs() < 1e-6);
    }

    #[test]
    fn f16_roundtrip_half() {
        let h = f32_to_f16(0.5);
        let f = f16_to_f32(h);
        assert!((f - 0.5).abs() < 1e-6);
    }

    #[test]
    fn f16_roundtrip_small() {
        let h = f32_to_f16(0.001);
        let f = f16_to_f32(h);
        assert!((f - 0.001).abs() < 0.0001, "f16 roundtrip 0.001: got {f}");
    }

    #[test]
    fn f16_inf_and_nan() {
        let inf_h = f32_to_f16(f32::INFINITY);
        assert_eq!(f16_to_f32(inf_h), f32::INFINITY);
        let neg_inf_h = f32_to_f16(f32::NEG_INFINITY);
        assert_eq!(f16_to_f32(neg_inf_h), f32::NEG_INFINITY);
        let nan_h = f32_to_f16(f32::NAN);
        assert!(f16_to_f32(nan_h).is_nan());
    }

    #[test]
    fn evidential_pair_basic() {
        let p = EvidentialPair::new(0.8, 0.2);
        assert!((p.g() - 0.6).abs() < 1e-6);
        assert!(p.is_strong_positive());
        assert!(!p.is_strong_negative());
        assert!(!p.is_contradiction());
    }

    #[test]
    fn evidential_pair_contradiction() {
        let p = EvidentialPair::new(0.7, 0.6);
        assert!(p.is_contradiction());
        assert!(p.to_belnap().is_contradiction());
    }

    #[test]
    fn evidential_pair_strong_negative() {
        let p = EvidentialPair::new(0.1, 0.8);
        assert!(p.is_strong_negative());
        assert!(!p.is_strong_positive());
    }

    #[test]
    fn evidential_pair_clamp() {
        let p = EvidentialPair::new(1.5, -0.3);
        assert_eq!(p.mu, 1.0);
        assert_eq!(p.lambda, 0.0);
    }

    #[test]
    fn evidential_pack_unpack() {
        let p = EvidentialPair::new(0.75, 0.25);
        let packed = p.pack();
        let unpacked = EvidentialPair::unpack(packed);
        assert!((unpacked.mu - 0.75).abs() < 0.01);
        assert!((unpacked.lambda - 0.25).abs() < 0.01);
    }

    #[test]
    fn evidential_pack_into_metadata_preserves_lamport() {
        // metadata with Lamport clock in [32..60] = 12345
        let metadata: u64 = (12345u64 << 32) | 0b11 << 61;
        let p = EvidentialPair::new(0.8, 0.3);
        let packed = p.pack_into_metadata(metadata);
        // Lamport clock should be preserved.
        let lamport = (packed >> 32) & 0x7FFFFFF;
        assert_eq!(lamport, 12345);
        // Routing lane should be preserved.
        let lane = (packed >> 61) & 0b11;
        assert_eq!(lane, 0b11);
        // Evidential data should be in [0..31].
        let unpacked = EvidentialPair::unpack_from_metadata(packed);
        assert!((unpacked.mu - 0.8).abs() < 0.01);
        assert!((unpacked.lambda - 0.3).abs() < 0.01);
    }

    #[test]
    fn pack_evidential_into_quin() {
        let q = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 4,
            metadata: 0,
            parity: 0,
        };
        let packed = pack_evidential(q, 0.9, 0.1);
        // Opcode should be set.
        assert_eq!((packed.predicate & 0xFF) as u8, OP_EVIDENTIAL_PACK);
        // Evidential data should be extractable.
        let ev = extract_evidential(&packed);
        assert!((ev.mu - 0.9).abs() < 0.01);
        assert!((ev.lambda - 0.1).abs() < 0.01);
    }

    #[test]
    fn evidential_score_from_quin() {
        let q = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 4,
            metadata: 0,
            parity: 0,
        };
        let packed = pack_evidential(q, 0.8, 0.2);
        let g = evidential_score(&packed);
        assert!((g - 0.6).abs() < 0.01);
    }

    #[test]
    fn route_evidential_normal() {
        let q = pack_evidential(NQuin::default(), 0.3, 0.2);
        assert_eq!(route_evidential(&q), EvidentialRoute::Normal);
    }

    #[test]
    fn route_evidential_quarantine() {
        let q = pack_evidential(NQuin::default(), 0.8, 0.1);
        assert_eq!(route_evidential(&q), EvidentialRoute::Quarantine);
    }

    #[test]
    fn route_evidential_reject() {
        let q = pack_evidential(NQuin::default(), 0.1, 0.9);
        assert_eq!(route_evidential(&q), EvidentialRoute::Reject);
    }

    #[test]
    fn route_evidential_contradiction() {
        let q = pack_evidential(NQuin::default(), 0.7, 0.6);
        assert_eq!(route_evidential(&q), EvidentialRoute::Contradiction);
    }

    #[test]
    fn route_evidential_batch_all_four() {
        let normal = pack_evidential(
            NQuin {
                subject: 1,
                ..Default::default()
            },
            0.3,
            0.2,
        );
        let quarantine = pack_evidential(
            NQuin {
                subject: 2,
                ..Default::default()
            },
            0.9,
            0.1,
        );
        let reject = pack_evidential(
            NQuin {
                subject: 3,
                ..Default::default()
            },
            0.1,
            0.8,
        );
        let contradiction = pack_evidential(
            NQuin {
                subject: 4,
                ..Default::default()
            },
            0.7,
            0.6,
        );
        let quins = vec![normal, quarantine, reject, contradiction];
        let mut out_n = [NQuin::default(); 10];
        let mut out_q = [NQuin::default(); 10];
        let mut out_r = [NQuin::default(); 10];
        let mut out_c = [NQuin::default(); 10];
        let (n, q, r, c) =
            route_evidential_batch(&quins, &mut out_n, &mut out_q, &mut out_r, &mut out_c).unwrap();
        assert_eq!((n, q, r, c), (1, 1, 1, 1));
        assert_eq!(out_n[0].subject, 1);
        assert_eq!(out_q[0].subject, 2);
        assert_eq!(out_r[0].subject, 3);
        assert_eq!(out_c[0].subject, 4);
    }

    #[test]
    fn evidential_saturation_check() {
        // global_saturation(consistent, isolated) = isolated / (consistent + isolated)
        let s = evidential_saturation(80, 20);
        assert!((s - 0.2).abs() < 1e-6);
        assert!(is_evidentially_saturated(80, 20, 0.15));
        assert!(!is_evidentially_saturated(80, 20, 0.5));
    }

    #[test]
    fn evidential_claim_from_quin() {
        // NQuin predicate: [8..62]=property-path hash, [0..7]=opcode.
        // So predicate 100 must be stored as 100 << 8.
        let q = pack_evidential(
            NQuin {
                subject: 42,
                predicate: 100 << 8,
                object: 200,
                context: 300,
                metadata: 0,
                parity: 0,
            },
            0.85,
            0.15,
        );
        let claim = EvidentialClaim::from_quin(&q);
        assert_eq!(claim.subject, 42);
        assert_eq!(claim.predicate, 100); // opcode stripped by >> 8
        assert_eq!(claim.object, 200);
        assert_eq!(claim.context, 300);
        assert!((claim.evidence.mu - 0.85).abs() < 0.01);
        assert_eq!(claim.route, EvidentialRoute::Quarantine);
    }

    #[test]
    fn evidential_claim_to_quin_roundtrip() {
        let original = pack_evidential(
            NQuin {
                subject: 1,
                predicate: 2 << 8,
                object: 3,
                context: 4,
                metadata: 0,
                parity: 0,
            },
            0.6,
            0.4,
        );
        let claim = EvidentialClaim::from_quin(&original);
        let claim_quin = claim.to_claim_quin();
        // The claim quin should have the evidential VC opcode.
        assert_eq!((claim_quin.predicate & 0xFF) as u8, OP_EVIDENTIAL_VC);
        // The evidential data should be preserved.
        let ev = extract_evidential(&claim_quin);
        assert!((ev.mu - 0.6).abs() < 0.01);
        assert!((ev.lambda - 0.4).abs() < 0.01);
    }

    #[test]
    fn evidential_claim_describe() {
        let q = pack_evidential(
            NQuin {
                subject: 1,
                predicate: 2,
                object: 3,
                context: 4,
                metadata: 0,
                parity: 0,
            },
            0.8,
            0.1,
        );
        let claim = EvidentialClaim::from_quin(&q);
        let desc = claim.describe();
        assert!(desc.contains("μ=0.8"));
        assert!(desc.contains("λ=0.1"));
        assert!(desc.contains("route=quarantine"));
    }

    #[test]
    fn evidential_to_belnap() {
        assert_eq!(EvidentialPair::new(0.8, 0.1).to_belnap(), Belnap::True);
        assert_eq!(EvidentialPair::new(0.1, 0.8).to_belnap(), Belnap::False);
        assert_eq!(EvidentialPair::new(0.1, 0.1).to_belnap(), Belnap::Neither);
        assert_eq!(EvidentialPair::new(0.7, 0.6).to_belnap(), Belnap::Both);
    }
}
