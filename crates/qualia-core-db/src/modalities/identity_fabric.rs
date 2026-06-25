//! Resilient relational identity (§27, legal_logic.md) — fabric resolution.
//!
//! The strict axiom: an **identifier is not an identity**. A key, DID, or name is a pointer;
//! identity is the dynamically-computed result of a *fabric* of anchors (identifiers,
//! attestations, relations). So losing a primary key must NOT collapse identity — it is
//! re-computed from the surviving fabric (the refugee / device-loss / theft resilience case),
//! provided a quorum of anchors survives (k-of-n social/relational recovery).
//!
//! See [[principle-identifiers-not-identity]]. This complements the foundational
//! `modal_kind`/`resolve` layer with the *resilience* primitive. Zero-heap.

/// Identity survives the loss of anchors iff a **quorum** of the fabric remains. `quorum` must
/// be ≥1 (an identity anchored by nothing is not an identity). k-of-n recovery.
#[inline]
pub fn identity_survives_loss(total_anchors: usize, lost_anchors: usize, quorum: usize) -> bool {
    quorum > 0 && total_anchors.saturating_sub(lost_anchors) >= quorum
}

/// The surviving anchor count after a loss (saturating).
#[inline]
pub fn surviving_anchors(total_anchors: usize, lost_anchors: usize) -> usize {
    total_anchors.saturating_sub(lost_anchors)
}

/// Re-compute the active anchor set from `all_anchors`, excluding any in `lost`, into `out`.
/// Returns the count — the surviving fabric an identity is reconstructed from. Zero-heap.
pub fn recompute_fabric(all_anchors: &[u64], lost: &[u64], out: &mut [u64]) -> usize {
    let mut n = 0usize;
    for &a in all_anchors {
        if !lost.contains(&a) {
            if n >= out.len() {
                break;
            }
            out[n] = a;
            n += 1;
        }
    }
    n
}

/// The axiom, made explicit: an identifier is never, by itself, the identity.
#[inline]
pub const fn identifier_is_not_identity() -> bool {
    true
}

/// Identity as an **enumerated state** ([[principle-identifiers-not-identity]]): its confidence is
/// the share of its `total` cryptographic anchors (identifiers + related datasets) currently
/// `present`. One identifier of many → low confidence; the full enumerated fabric → high. `0.0`
/// if `total == 0` (an enumeration of nothing is not an identity).
pub fn enumerated_identity_confidence(present: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        present as f32 / total as f32
    }
}

// ─── Shamir's Secret Sharing (k-of-n quorum key recovery) ─────────────────────────
//
// A real threshold scheme over the prime field GF(2^61−1): a secret is the constant term of a
// random degree-(k−1) polynomial; shares are evaluations; any k shares reconstruct it by Lagrange
// interpolation at x=0, fewer than k reveal nothing. Zero-heap (bounded arrays, u128 intermediates).

/// The Mersenne prime field modulus `2^61 − 1`.
pub const SHAMIR_PRIME: u64 = (1u64 << 61) - 1;

#[inline]
fn m_add(a: u64, b: u64) -> u64 {
    ((a as u128 + b as u128) % SHAMIR_PRIME as u128) as u64
}
#[inline]
fn m_sub(a: u64, b: u64) -> u64 {
    ((a as u128 + SHAMIR_PRIME as u128 - (b % SHAMIR_PRIME) as u128) % SHAMIR_PRIME as u128) as u64
}
#[inline]
fn m_mul(a: u64, b: u64) -> u64 {
    ((a as u128 * b as u128) % SHAMIR_PRIME as u128) as u64
}
fn m_pow(mut base: u64, mut exp: u64) -> u64 {
    base %= SHAMIR_PRIME;
    let mut r = 1u64;
    while exp > 0 {
        if exp & 1 == 1 {
            r = m_mul(r, base);
        }
        base = m_mul(base, base);
        exp >>= 1;
    }
    r
}
#[inline]
fn m_inv(a: u64) -> u64 {
    m_pow(a, SHAMIR_PRIME - 2) // Fermat: a^(p-2) ≡ a⁻¹
}

/// Evaluate the sharing polynomial `secret + Σ coeffs[i]·xⁱ⁺¹` at `x`, mod the prime (Horner).
fn poly_eval(secret: u64, coeffs: &[u64], x: u64) -> u64 {
    let mut y = 0u64;
    for &c in coeffs.iter().rev() {
        y = m_add(m_mul(y, x), c);
    }
    m_add(m_mul(y, x), secret)
}

/// Split `secret` (reduced mod the prime) into `n` shares with threshold `k = coeffs.len()+1`:
/// share i (x-coord `i+1`) y-value is written to `ys[i]`. `coeffs` are the `k−1` polynomial
/// coefficients (from a CSPRNG in production; caller-supplied here for determinism). Returns `n`.
pub fn shamir_split(secret: u64, coeffs: &[u64], n: usize, ys: &mut [u64]) -> usize {
    let s = secret % SHAMIR_PRIME;
    let mut written = 0usize;
    for i in 0..n {
        if i >= ys.len() {
            break;
        }
        ys[i] = poly_eval(s, coeffs, (i + 1) as u64);
        written += 1;
    }
    written
}

/// Reconstruct the secret from `k` shares `(xs[i], ys[i])` by Lagrange interpolation at `x = 0`.
/// Any `k` of the `n` shares recover the secret; fewer reveal nothing. Zero-heap.
pub fn shamir_reconstruct(xs: &[u64], ys: &[u64], k: usize) -> u64 {
    let k = k.min(xs.len()).min(ys.len());
    let mut secret = 0u64;
    for i in 0..k {
        let mut num = 1u64; // Π_{j≠i} (0 − x_j) = Π (−x_j)
        let mut den = 1u64; // Π_{j≠i} (x_i − x_j)
        for j in 0..k {
            if j == i {
                continue;
            }
            num = m_mul(num, m_sub(0, xs[j]));
            den = m_mul(den, m_sub(xs[i], xs[j]));
        }
        let term = m_mul(ys[i], m_mul(num, m_inv(den)));
        secret = m_add(secret, term);
    }
    secret
}

// ─── ZKP capability derivation & recursive web-of-trust ───────────────────────────

/// **ZKP capability derivation**: a capability is granted by PROVING an identity trait (a zk proof)
/// WITHOUT revealing the core identifier. Granted iff `trait_proven` AND the identifier was NOT
/// revealed — the proof carries the trait, not the id (privacy-preserving derivation).
#[inline]
pub fn zkp_capability_granted(trait_proven: bool, identifier_revealed: bool) -> bool {
    trait_proven && !identifier_revealed
}

/// **Recursive identity anchoring with web-of-trust decay**: an identity asserted through a chain
/// of `depth` intermediary identities has confidence `base · decay^depth` — trust attenuates with
/// each hop. `decay ∈ [0,1]`; `depth = 0` is a direct anchor (full `base`).
pub fn web_of_trust_confidence(base: f32, depth: u32, decay: f32) -> f32 {
    base * decay.powi(depth as i32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    #[test]
    fn shamir_k_of_n_recovers_the_secret() {
        let secret = 0x0BADC0DE_1234u64;
        let coeffs = [98765u64, 4242u64]; // k = 3 (degree 2)
        let mut ys = [0u64; 5];
        let n = shamir_split(secret, &coeffs, 5, &mut ys); // 5 shares
        assert_eq!(n, 5);
        // Any 3 shares reconstruct the secret.
        assert_eq!(shamir_reconstruct(&[1, 2, 3], &[ys[0], ys[1], ys[2]], 3), secret);
        assert_eq!(shamir_reconstruct(&[2, 4, 5], &[ys[1], ys[3], ys[4]], 3), secret);
        // Fewer than k shares do NOT yield the secret.
        assert_ne!(shamir_reconstruct(&[1, 2], &[ys[0], ys[1]], 2), secret);
    }

    #[test]
    fn enumerated_identity_zkp_and_web_of_trust() {
        // Enumerated identity: 3 of 4 anchors present → 0.75 confidence.
        assert!((enumerated_identity_confidence(3, 4) - 0.75).abs() < 1e-6);
        assert_eq!(enumerated_identity_confidence(1, 0), 0.0);
        // ZKP capability: granted only when the trait is proven AND the id stays hidden.
        assert!(zkp_capability_granted(true, false));
        assert!(!zkp_capability_granted(true, true), "revealing the identifier defeats the point");
        assert!(!zkp_capability_granted(false, false));
        // Web-of-trust decay: 0.9 base, decay 0.5 → depth 0 = 0.9, depth 2 = 0.225.
        assert!((web_of_trust_confidence(0.9, 0, 0.5) - 0.9).abs() < 1e-6);
        assert!((web_of_trust_confidence(0.9, 2, 0.5) - 0.225).abs() < 1e-6);
    }

    #[test]
    fn identity_survives_key_loss_with_quorum() {
        // 5 anchors, lose the primary key (1), quorum of 3 → survives.
        assert!(identity_survives_loss(5, 1, 3));
        // Lose 3 of 5, quorum 3 → exactly meets → survives.
        assert!(identity_survives_loss(5, 2, 3));
        // Lose too many → identity cannot be reconstructed.
        assert!(!identity_survives_loss(5, 3, 3));
        // Quorum 0 is invalid (nothing anchors nothing).
        assert!(!identity_survives_loss(5, 0, 0));
    }

    #[test]
    fn fabric_recomputes_from_survivors() {
        let key = q_hash("anchor:primaryKey");
        let social = q_hash("anchor:socialAttestation");
        let bio = q_hash("anchor:biometric");
        let device = q_hash("anchor:device");
        let all = [key, social, bio, device];
        let lost = [key, device]; // stolen phone + its key
        let mut out = [0u64; 8];
        let n = recompute_fabric(&all, &lost, &mut out);
        assert_eq!(n, 2, "identity re-computes from the surviving relational fabric");
        assert!(out[..n].contains(&social) && out[..n].contains(&bio));
        assert!(!out[..n].contains(&key));
        assert!(identifier_is_not_identity());
    }
}
