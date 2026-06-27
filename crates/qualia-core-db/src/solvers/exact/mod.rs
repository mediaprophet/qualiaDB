//! Exact / arbitrary-precision arithmetic — the §3.1 "exact computation"
//! foundation.
//!
//! - [`bigint::BigInt`] — arbitrary-precision signed integer (sign + little-
//!   endian `u32` magnitude, schoolbook multiply, Knuth long division).
//! - [`rational::BigRational`] — exact rational over `BigInt`, always reduced
//!   and sign-normalised.
//!
//! These are heap-backed and live deliberately **off** the zero-heap hot path.
//! Fallible operations (zero divisor / zero denominator) fail closed with
//! `Option`/`Result` and never fabricate a value.

pub mod bigint;
pub mod rational;

pub use bigint::BigInt;
pub use rational::BigRational;
