//! **Number theory & combinatorics** (Gap analysis §3.2-NT).
//!
//! Primality, factorization, modular arithmetic, the classic arithmetic functions
//! (Euler totient, Möbius, divisor sums) and combinatorics (factorials, binomials,
//! partitions, Stirling, Catalan). Underpins the CAS, crypto, and **constructibility**
//! (the regular-polygon decision needs Fermat-prime factorization and `φ(n)`), and is
//! one of the bounded, high-demand gaps the computational-engine gap analysis surfaced.
//!
//! Exact integer arithmetic over `u64`/`i64` (with `u128` intermediates to avoid
//! overflow in modular multiply); fail-closed via `Option`/[`NumberTheoryError`] on
//! degenerate input. Kernel-class `Divergent` (branch-heavy) with a pure CPU path.
//!
//! Layers (§11): [`modular`], [`primes`], [`arithmetic_functions`], [`combinatorics`].

pub mod arithmetic_functions;
pub mod combinatorics;
pub mod modular;
pub mod primes;

pub use arithmetic_functions::{divisor_count, divisor_sum, euler_totient, mobius};
pub use combinatorics::{
    binomial, catalan, factorial, partitions, stirling_first, stirling_second,
};
pub use modular::{extended_gcd, gcd, lcm, mod_inverse, mod_pow};
pub use primes::{divisors, is_prime, next_prime, prime_factors};

/// Fail-closed errors for number-theoretic operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberTheoryError {
    /// Input outside the function's domain (e.g. factoring 0, totient of 0).
    OutOfDomain,
    /// No result exists (e.g. modular inverse when gcd ≠ 1).
    NoSolution,
}

impl core::fmt::Display for NumberTheoryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            NumberTheoryError::OutOfDomain => write!(f, "argument out of domain"),
            NumberTheoryError::NoSolution => write!(f, "no solution exists"),
        }
    }
}
impl std::error::Error for NumberTheoryError {}
