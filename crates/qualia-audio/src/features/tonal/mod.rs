//! Tonal — real HPCP, tuning, key/scale, chord (declared assumptions, abstain).
//!
//! This module supersedes the fake `bin % 12` "chroma" that lived in `music.rs`:
//! [`hpcp`] performs a proper `log2`-based frequency → pitch-class mapping from
//! spectral peaks. Every function that assumes 12-TET takes an explicit assumption
//! flag and **abstains** rather than force a label — Qualia is modality-first and
//! does not treat 12-TET as universal truth. Tuning (`ref_freq_hz` / `n_pc`) is a
//! parameter throughout. Re-exports only (AU-TONAL).

pub mod chord;
pub mod hpcp;
pub mod key;
pub mod scale;
pub mod tuning;

pub use chord::{estimate_chord, ChordEstimate, ChordQuality};
pub use hpcp::hpcp;
pub use key::{estimate_key, KeyEstimate};
pub use scale::{estimate_scale, ScaleEstimate, ScaleProposal};
pub use tuning::{estimate_tuning, TuningEstimate};
