---
created: 2026-06-30
updated: 2026-06-30
update_scope: Comprehensive
---

# number_theory Index

## Functionality Overview
Comprehensive index of functionality for `number_theory`. This document serves as the ground truth for bots regarding implemented components and dependencies.

## File & Subdirectory Manifest
### Files & Exported Functionality
- 📄 `arithmetic_functions.rs`
  - `fn euler_totient`
  - `fn mobius`
  - `fn divisor_count`
  - `fn divisor_sum`
  - `fn totient_known_values`
  - `fn mobius_known_values`
  - `fn divisor_functions`
- 📄 `combinatorics.rs`
  - `fn factorial`
  - `fn binomial`
  - `fn partitions`
  - `fn stirling_second`
  - `fn stirling_first`
  - `fn catalan`
  - `fn factorial_and_overflow_guard`
  - `fn binomial_known_values`
  - `fn partition_counts`
  - `fn stirling_numbers`
  - `fn catalan_numbers`
- 📄 `mod.rs`
  - `enum NumberTheoryError`
  - `impl core`
  - `fn fmt`
  - `impl std`
- 📄 `modular.rs`
  - `fn gcd`
  - `fn lcm`
  - `fn extended_gcd`
  - `fn mod_pow`
  - `fn mod_inverse`
  - `fn crt`
  - `fn gcd_lcm_basics`
  - `fn extended_gcd_satisfies_bezout`
  - `fn mod_pow_matches_known_values`
  - `fn mod_inverse_exists_iff_coprime`
  - `fn crt_combines_congruences`
- 📄 `primes.rs`
  - `fn mulmod`
  - `fn is_prime`
  - `fn next_prime`
  - `fn pollard_rho`
  - `fn factor_into`
  - `fn prime_factors`
  - `fn divisors`
  - `fn primality_known_cases`
  - `fn factorization_is_correct_and_reconstructs`
  - `fn divisors_of_28_are_perfect`
  - `fn next_prime_walks_forward`

## Changelog
- **2026-06-30**: Automated full index generation, extracting code definitions.
