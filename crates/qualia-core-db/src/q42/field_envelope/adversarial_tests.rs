//! V01 adversarial & golden structural envelope test entry point.
//!
//! Wire into `q42/field_envelope/mod.rs` with:
//! `#[cfg(test)] mod adversarial_tests;`

#[path = "adversarial_vectors/helpers.rs"]
mod helpers;

#[path = "adversarial_vectors/golden_and_modes.rs"]
mod golden_and_modes;

#[path = "adversarial_vectors/header_directory_attacks.rs"]
mod header_directory_attacks;

#[path = "adversarial_vectors/metadata_numeric_limits_t01.rs"]
mod metadata_numeric_limits_t01;

#[path = "adversarial_vectors/v01_corpus_replay.rs"]
mod v01_corpus_replay;
