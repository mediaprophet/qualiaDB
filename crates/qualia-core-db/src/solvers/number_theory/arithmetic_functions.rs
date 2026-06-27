//! The classical multiplicative arithmetic functions, computed from the prime
//! factorization: Euler's totient `φ`, the Möbius function `μ`, divisor count `d(n)`
//! and divisor sum `σ(n)`.

use super::primes::prime_factors;

/// Euler's totient `φ(n)` — the count of integers in `[1, n]` coprime to `n`.
/// `φ(0) = 0`, `φ(1) = 1`. Computed as `n · ∏_{p|n} (1 − 1/p)`.
pub fn euler_totient(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut result = n;
    for (p, _) in prime_factors(n) {
        result = result / p * (p - 1);
    }
    result
}

/// The Möbius function `μ(n)`: `0` if `n` is divisible by a square > 1, else `(−1)^k`
/// where `k` is the number of distinct prime factors. `μ(1) = 1`, `μ(0) = 0`.
pub fn mobius(n: u64) -> i8 {
    if n == 0 {
        return 0;
    }
    if n == 1 {
        return 1;
    }
    let factors = prime_factors(n);
    if factors.iter().any(|&(_, e)| e > 1) {
        return 0; // squareful
    }
    if factors.len() % 2 == 0 {
        1
    } else {
        -1
    }
}

/// Number of positive divisors `d(n) = ∏ (e_i + 1)`. `d(1) = 1`, `d(0) = 0`.
pub fn divisor_count(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    prime_factors(n).iter().map(|&(_, e)| (e + 1) as u64).product::<u64>().max(1)
}

/// Sum of positive divisors `σ(n) = ∏ (p^{e+1} − 1)/(p − 1)`. `σ(1) = 1`, `σ(0) = 0`.
pub fn divisor_sum(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut sum = 1u64;
    for (p, e) in prime_factors(n) {
        // (p^{e+1} − 1)/(p − 1) = 1 + p + … + p^e
        let mut term = 1u64;
        let mut pk = 1u64;
        for _ in 0..e {
            pk *= p;
            term += pk;
        }
        sum *= term;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn totient_known_values() {
        assert_eq!(euler_totient(1), 1);
        assert_eq!(euler_totient(9), 6); // 1,2,4,5,7,8
        assert_eq!(euler_totient(10), 4); // 1,3,7,9
        assert_eq!(euler_totient(12), 4);
        // φ(p) = p − 1 for prime p.
        assert_eq!(euler_totient(17), 16);
        // φ is what the regular-polygon (Gauss–Wantzel) decision rests on.
        assert_eq!(euler_totient(15), 8); // 3·5 → (3−1)(5−1)
    }

    #[test]
    fn mobius_known_values() {
        assert_eq!(mobius(1), 1);
        assert_eq!(mobius(2), -1); // one prime
        assert_eq!(mobius(6), 1); // two primes (2·3)
        assert_eq!(mobius(30), -1); // three primes (2·3·5)
        assert_eq!(mobius(4), 0); // 2² squareful
        assert_eq!(mobius(12), 0); // 2²·3
    }

    #[test]
    fn divisor_functions() {
        // 28 = 2²·7 → d = 3·2 = 6 ; σ = 7·8 = 56
        assert_eq!(divisor_count(28), 6);
        assert_eq!(divisor_sum(28), 56);
        // perfect number: σ(n) = 2n.
        assert_eq!(divisor_sum(6), 12);
        assert_eq!(divisor_count(1), 1);
        assert_eq!(divisor_sum(1), 1);
    }
}
