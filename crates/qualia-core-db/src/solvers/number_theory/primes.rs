//! Primality and factorization. `is_prime` is a **deterministic** Miller–Rabin (the
//! witness set `{2,3,5,…,37}` is proven correct for all of `u64`), and `prime_factors`
//! uses trial division for small factors then **Pollard's rho** (Brent's variant) for
//! the rest, so factorization is correct across the full `u64` range — not just up to
//! `√n`.

use super::modular::mod_pow;

#[inline]
fn mulmod(a: u64, b: u64, m: u64) -> u64 {
    ((a as u128 * b as u128) % m as u128) as u64
}

/// Deterministic Miller–Rabin primality test, exact for all `u64`.
pub fn is_prime(n: u64) -> bool {
    if n < 2 {
        return false;
    }
    for &p in &[2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        if n == p {
            return true;
        }
        if n % p == 0 {
            return false;
        }
    }
    // n − 1 = d · 2^r
    let mut d = n - 1;
    let mut r = 0u32;
    while d & 1 == 0 {
        d >>= 1;
        r += 1;
    }
    'witness: for &a in &[2u64, 3, 5, 7, 11, 13, 17, 19, 23, 29, 31, 37] {
        let mut x = mod_pow(a, d, n);
        if x == 1 || x == n - 1 {
            continue;
        }
        for _ in 0..r - 1 {
            x = mulmod(x, x, n);
            if x == n - 1 {
                continue 'witness;
            }
        }
        return false; // composite
    }
    true
}

/// The smallest prime strictly greater than `n`.
pub fn next_prime(n: u64) -> u64 {
    let mut c = n.saturating_add(1);
    if c <= 2 {
        return 2;
    }
    if c % 2 == 0 {
        c += 1;
    }
    loop {
        if is_prime(c) {
            return c;
        }
        c = c.saturating_add(2);
    }
}

/// Pollard's rho (Brent) — returns a non-trivial factor of a composite `n`.
fn pollard_rho(n: u64) -> u64 {
    if n % 2 == 0 {
        return 2;
    }
    let mut c = 1u64;
    loop {
        let f = |x: u64| (mulmod(x, x, n) + c) % n;
        let mut x = 2u64;
        let mut y = 2u64;
        let mut d = 1u64;
        while d == 1 {
            x = f(x);
            y = f(f(y));
            d = super::modular::gcd(x.abs_diff(y), n);
        }
        if d != n {
            return d;
        }
        c += 1; // cycle hit n itself; retry with a different constant
    }
}

fn factor_into(n: u64, out: &mut Vec<u64>) {
    if n == 1 {
        return;
    }
    if is_prime(n) {
        out.push(n);
        return;
    }
    let d = pollard_rho(n);
    factor_into(d, out);
    factor_into(n / d, out);
}

/// Prime factorization as `(prime, exponent)` pairs, ascending by prime. Empty for
/// `n < 2` (0 and 1 have no prime factorization).
pub fn prime_factors(n: u64) -> Vec<(u64, u32)> {
    if n < 2 {
        return Vec::new();
    }
    let mut flat = Vec::new();
    factor_into(n, &mut flat);
    flat.sort_unstable();
    let mut out: Vec<(u64, u32)> = Vec::new();
    for p in flat {
        if let Some(last) = out.last_mut() {
            if last.0 == p {
                last.1 += 1;
                continue;
            }
        }
        out.push((p, 1));
    }
    out
}

/// All positive divisors of `n`, ascending. `[1]` for `n = 1`; empty for `n = 0`.
pub fn divisors(n: u64) -> Vec<u64> {
    if n == 0 {
        return Vec::new();
    }
    let mut divs = vec![1u64];
    for (p, e) in prime_factors(n) {
        let mut pk = 1u64;
        let base = divs.clone();
        for _ in 0..e {
            pk *= p;
            for &d in &base {
                divs.push(d * pk);
            }
        }
    }
    divs.sort_unstable();
    divs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primality_known_cases() {
        for p in [2u64, 3, 5, 7, 13, 97, 7919, 104729, 1_000_000_007] {
            assert!(is_prime(p), "{p} should be prime");
        }
        for c in [0u64, 1, 4, 9, 15, 100, 7917, 1_000_000_011] {
            assert!(!is_prime(c), "{c} should be composite");
        }
        // A large Carmichael number (561 = 3·11·17) fools Fermat but not Miller–Rabin.
        assert!(!is_prime(561));
        // A 64-bit semiprime is composite.
        assert!(!is_prime(10_000_000_000_000_061 * 3));
    }

    #[test]
    fn factorization_is_correct_and_reconstructs() {
        assert_eq!(prime_factors(360), vec![(2, 3), (3, 2), (5, 1)]); // 2³·3²·5
        assert_eq!(prime_factors(97), vec![(97, 1)]);
        assert_eq!(prime_factors(1), vec![]);
        // A hard semiprime that needs Pollard's rho (beyond √n trial division).
        let n = 1_000_000_007u64 * 1_000_000_009u64;
        let f = prime_factors(n);
        assert_eq!(f, vec![(1_000_000_007, 1), (1_000_000_009, 1)]);
        // Product of (prime^exp) reconstructs n.
        let prod: u64 = f.iter().map(|&(p, e)| p.pow(e)).product();
        assert_eq!(prod, n);
    }

    #[test]
    fn divisors_of_28_are_perfect() {
        let d = divisors(28);
        assert_eq!(d, vec![1, 2, 4, 7, 14, 28]);
        // 28 is perfect: its proper divisors sum to itself.
        let proper: u64 = d.iter().filter(|&&x| x != 28).sum();
        assert_eq!(proper, 28);
    }

    #[test]
    fn next_prime_walks_forward() {
        assert_eq!(next_prime(13), 17);
        assert_eq!(next_prime(0), 2);
        assert_eq!(next_prime(1), 2);
        assert_eq!(next_prime(89), 97);
    }
}
