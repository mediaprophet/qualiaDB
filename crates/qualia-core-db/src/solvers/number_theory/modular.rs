//! Modular arithmetic and the Euclidean algorithms — the backbone the rest of the
//! library (primality, totient, CRT) is built on. Overflow-safe via `u128`/`i128`
//! intermediates.

/// Greatest common divisor (binary/Euclid). `gcd(0, n) = n`.
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Least common multiple. `0` if either argument is `0`.
pub fn lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        return 0;
    }
    a / gcd(a, b) * b
}

/// Extended Euclid: returns `(g, x, y)` with `a·x + b·y = g = gcd(a, b)`.
pub fn extended_gcd(a: i64, b: i64) -> (i64, i64, i64) {
    if b == 0 {
        return (a.abs(), if a < 0 { -1 } else { 1 }, 0);
    }
    let (g, x, y) = extended_gcd(b, a % b);
    (g, y, x - (a / b) * y)
}

/// `(base^exp) mod modulus` by repeated squaring. `modulus = 0` → `0` (degenerate).
pub fn mod_pow(mut base: u64, mut exp: u64, modulus: u64) -> u64 {
    if modulus <= 1 {
        return 0;
    }
    let m = modulus as u128;
    let mut result: u128 = 1;
    base %= modulus;
    let mut b = base as u128;
    while exp > 0 {
        if exp & 1 == 1 {
            result = result * b % m;
        }
        b = b * b % m;
        exp >>= 1;
    }
    result as u64
}

/// Modular multiplicative inverse of `a` mod `m`: the `x` with `a·x ≡ 1 (mod m)`.
/// `None` when `gcd(a, m) ≠ 1` (no inverse exists) — fail closed.
pub fn mod_inverse(a: u64, m: u64) -> Option<u64> {
    if m == 0 {
        return None;
    }
    let (g, x, _) = extended_gcd((a % m) as i64, m as i64);
    if g != 1 {
        return None;
    }
    Some(((x % m as i64 + m as i64) % m as i64) as u64)
}

/// Chinese Remainder: solve `x ≡ r1 (mod m1)`, `x ≡ r2 (mod m2)` for **coprime**
/// moduli, returning `(x, m1·m2)`. `None` if the moduli are not coprime.
pub fn crt(r1: u64, m1: u64, r2: u64, m2: u64) -> Option<(u64, u64)> {
    let inv = mod_inverse(m1 % m2, m2)?;
    let m = m1.checked_mul(m2)?;
    // x = r1 + m1 * ((r2 - r1) * inv mod m2)
    let diff = (r2 as i128 - r1 as i128).rem_euclid(m2 as i128) as u128;
    let t = diff * inv as u128 % m2 as u128;
    let x = (r1 as u128 + m1 as u128 * t) % m as u128;
    Some((x as u64, m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gcd_lcm_basics() {
        assert_eq!(gcd(54, 24), 6);
        assert_eq!(gcd(17, 5), 1);
        assert_eq!(gcd(0, 9), 9);
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(lcm(0, 5), 0);
    }

    #[test]
    fn extended_gcd_satisfies_bezout() {
        let (g, x, y) = extended_gcd(240, 46);
        assert_eq!(g, 2);
        assert_eq!(240 * x + 46 * y, g);
    }

    #[test]
    fn mod_pow_matches_known_values() {
        assert_eq!(mod_pow(2, 10, 1000), 24); // 1024 mod 1000
        assert_eq!(mod_pow(3, 0, 7), 1);
        // Fermat: a^(p-1) ≡ 1 (mod p) for prime p ∤ a.
        assert_eq!(mod_pow(2, 12, 13), 1);
    }

    #[test]
    fn mod_inverse_exists_iff_coprime() {
        // 3·4 = 12 ≡ 1 (mod 11)
        assert_eq!(mod_inverse(3, 11), Some(4));
        assert_eq!((3 * 4) % 11, 1);
        // No inverse for 4 mod 8 (gcd 4).
        assert_eq!(mod_inverse(4, 8), None);
    }

    #[test]
    fn crt_combines_congruences() {
        // x ≡ 2 (mod 3), x ≡ 3 (mod 5) → x = 8 (mod 15).
        let (x, m) = crt(2, 3, 3, 5).unwrap();
        assert_eq!(m, 15);
        assert_eq!(x, 8);
        assert_eq!(x % 3, 2);
        assert_eq!(x % 5, 3);
        // Non-coprime moduli → None.
        assert!(crt(1, 4, 2, 6).is_none());
    }
}
