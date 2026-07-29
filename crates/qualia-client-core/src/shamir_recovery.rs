//! **Shamir Secret Sharing over GF(2⁸)** — the primitive for *social recovery* of a key without the owner.
//!
//! A secret (e.g. a payload's data-encryption key, or a recovery key) is split into `n` shares such that any
//! `k` of them reconstruct it and any `k-1` reveal **nothing** (information-theoretic). The shares are handed
//! to chosen friends/trustees; after death or incapacity, a quorum of `k` of them combine their shares and
//! recover the key — **the owner's key is never needed** (this is what the dead-man / incapacity switches
//! need for true friend-side enactment, which key-release-on-enact could not do while it depended on the
//! owner's derived key).
//!
//! This is not a cipher and not a simulation: it is the standard Shamir scheme — a degree-`(k-1)` polynomial
//! per secret byte over the AES field GF(2⁸) (modulus `x⁸+x⁴+x³+x+1` = `0x11b`), evaluated at `x = 1..=n` for
//! the shares and Lagrange-interpolated at `x = 0` to recover the constant term (the secret byte). Field
//! multiplication uses carry-less multiply with reduction; the inverse is `a^254` (Fermat in GF(2⁸)). Fully
//! deterministic and testable.

use serde::{Deserialize, Serialize};

/// GF(2⁸) multiplication (AES field, modulus `0x11b`).
fn gf_mul(mut a: u8, mut b: u8) -> u8 {
    let mut p = 0u8;
    for _ in 0..8 {
        if b & 1 != 0 {
            p ^= a;
        }
        let hi = a & 0x80;
        a <<= 1;
        if hi != 0 {
            a ^= 0x1b; // reduce by the low bits of 0x11b (the x⁸ term is the shifted-out bit)
        }
        b >>= 1;
    }
    p
}

/// GF(2⁸) multiplicative inverse via Fermat: `a^(2⁸-2) = a^254 = a⁻¹` (for `a != 0`). `inv(0)` is defined as
/// `0` (never used — division only ever divides by a nonzero `x_i ^ x_j`).
fn gf_inv(a: u8) -> u8 {
    if a == 0 {
        return 0;
    }
    let mut result = 1u8;
    let mut base = a;
    let mut exp = 254u32;
    while exp > 0 {
        if exp & 1 == 1 {
            result = gf_mul(result, base);
        }
        base = gf_mul(base, base);
        exp >>= 1;
    }
    result
}

fn gf_div(a: u8, b: u8) -> u8 {
    gf_mul(a, gf_inv(b))
}

/// A single Shamir share: the evaluation point `x` (`1..=n`, distinct, nonzero) and the per-byte evaluations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Share {
    /// The evaluation abscissa (never 0 — 0 is the secret).
    pub x: u8,
    /// One field evaluation per secret byte.
    pub y: Vec<u8>,
}

/// A random field element (four is overkill for one byte, but keep it simple — one draw per coefficient).
fn random_byte() -> u8 {
    rand::random::<u8>()
}

/// **Split** `secret` into `n` shares, any `k` of which reconstruct it. `k` in `1..=n`, `n` in `1..=255`.
/// Each secret byte gets an independent random degree-`(k-1)` polynomial with that byte as the constant term.
pub fn split(secret: &[u8], k: usize, n: usize) -> Result<Vec<Share>, String> {
    if k == 0 || n == 0 {
        return Err("k and n must be >= 1".into());
    }
    if k > n {
        return Err(format!("threshold k={k} cannot exceed shares n={n}"));
    }
    if n > 255 {
        return Err("n must be <= 255 (distinct nonzero abscissae in GF(2^8))".into());
    }
    // Per-byte coefficients: coeff[byte][0] = secret byte; coeff[byte][1..k] = random.
    let coeffs: Vec<Vec<u8>> = secret
        .iter()
        .map(|&s| {
            let mut c = Vec::with_capacity(k);
            c.push(s);
            for _ in 1..k {
                c.push(random_byte());
            }
            c
        })
        .collect();

    let mut shares = Vec::with_capacity(n);
    for x in 1..=(n as u8) {
        let y: Vec<u8> = coeffs.iter().map(|c| eval_poly(c, x)).collect();
        shares.push(Share { x, y });
    }
    Ok(shares)
}

/// Evaluate a polynomial (Horner) at `x` in GF(2⁸).
fn eval_poly(coeffs: &[u8], x: u8) -> u8 {
    let mut acc = 0u8;
    for &c in coeffs.iter().rev() {
        acc = gf_mul(acc, x) ^ c;
    }
    acc
}

/// **Reconstruct** the secret from a set of shares via Lagrange interpolation at `x = 0`. Requires at least
/// the original `k` shares (fewer under-determines the polynomial and yields a wrong secret); all shares must
/// have equal-length `y` and distinct `x`. Providing more than `k` is fine (consistent, overdetermined).
pub fn reconstruct(shares: &[Share]) -> Result<Vec<u8>, String> {
    if shares.is_empty() {
        return Err("no shares".into());
    }
    let len = shares[0].y.len();
    if shares.iter().any(|s| s.y.len() != len) {
        return Err("shares have differing secret lengths".into());
    }
    // Distinct abscissae check.
    for i in 0..shares.len() {
        if shares[i].x == 0 {
            return Err("share abscissa 0 is invalid".into());
        }
        for j in (i + 1)..shares.len() {
            if shares[i].x == shares[j].x {
                return Err("duplicate share abscissa".into());
            }
        }
    }

    let mut secret = vec![0u8; len];
    for byte in 0..len {
        let mut acc = 0u8;
        for i in 0..shares.len() {
            // Lagrange basis L_i(0) = prod_{j!=i} (0 - x_j) / (x_i - x_j) = prod x_j / (x_i ^ x_j).
            let xi = shares[i].x;
            let mut num = 1u8;
            let mut den = 1u8;
            for j in 0..shares.len() {
                if i == j {
                    continue;
                }
                let xj = shares[j].x;
                num = gf_mul(num, xj);
                den = gf_mul(den, xi ^ xj);
            }
            let l0 = gf_div(num, den);
            acc ^= gf_mul(shares[i].y[byte], l0);
        }
        secret[byte] = acc;
    }
    Ok(secret)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gf_field_axioms() {
        // 1 is the identity; every nonzero element has an inverse; multiplication is associative-ish spot check.
        for a in 1u8..=255 {
            assert_eq!(gf_mul(a, 1), a);
            assert_eq!(gf_mul(a, gf_inv(a)), 1, "a * a^-1 == 1 for a={a}");
        }
        assert_eq!(gf_mul(0, 5), 0);
        // Distributivity spot-check: a*(b^c) == a*b ^ a*c.
        assert_eq!(gf_mul(7, 9 ^ 13), gf_mul(7, 9) ^ gf_mul(7, 13));
    }

    #[test]
    fn any_k_of_n_reconstructs_the_secret() {
        let secret = b"a 32-byte data-encryption key!!!".to_vec();
        let shares = split(&secret, 3, 5).unwrap();
        assert_eq!(shares.len(), 5);
        // Several distinct 3-subsets all recover the exact secret.
        for subset in [[0usize, 1, 2], [0, 2, 4], [1, 3, 4], [2, 3, 4]] {
            let chosen: Vec<Share> = subset.iter().map(|&i| shares[i].clone()).collect();
            assert_eq!(
                reconstruct(&chosen).unwrap(),
                secret,
                "subset {subset:?} recovers"
            );
        }
        // More than k (all 5) also recovers.
        assert_eq!(reconstruct(&shares).unwrap(), secret);
    }

    #[test]
    fn fewer_than_k_shares_do_not_recover_the_secret() {
        let secret = b"top secret payload key".to_vec();
        let shares = split(&secret, 3, 5).unwrap();
        // With only 2 of the 3 required shares, the interpolated value is not the secret.
        let two: Vec<Share> = vec![shares[0].clone(), shares[1].clone()];
        assert_ne!(
            reconstruct(&two).unwrap(),
            secret,
            "k-1 shares must not reveal the secret"
        );
    }

    #[test]
    fn threshold_equals_n_and_threshold_one() {
        let secret = b"edge".to_vec();
        // k == n: every share needed.
        let all = split(&secret, 4, 4).unwrap();
        assert_eq!(reconstruct(&all).unwrap(), secret);
        assert_ne!(reconstruct(&all[..3]).unwrap(), secret);
        // k == 1: any single share is the secret (degree-0 polynomial).
        let ones = split(&secret, 1, 3).unwrap();
        assert_eq!(reconstruct(&ones[1..2]).unwrap(), secret);
    }

    #[test]
    fn bad_parameters_are_rejected() {
        assert!(split(b"x", 0, 3).is_err());
        assert!(split(b"x", 4, 3).is_err(), "k > n");
        assert!(split(b"x", 1, 300).is_err(), "n > 255");
        assert!(reconstruct(&[]).is_err());
        let s = split(b"xy", 2, 3).unwrap();
        // Duplicate abscissa rejected.
        let dup = vec![s[0].clone(), s[0].clone()];
        assert!(reconstruct(&dup).is_err());
    }

    #[test]
    fn serde_round_trips() {
        let shares = split(b"round trip", 2, 3).unwrap();
        let json = serde_json::to_string(&shares).unwrap();
        let back: Vec<Share> = serde_json::from_str(&json).unwrap();
        assert_eq!(shares, back);
        assert_eq!(reconstruct(&back[..2]).unwrap(), b"round trip");
    }
}
