//! Audio Quality Assessment — intrusive (pure-Rust, no licence) + non-intrusive MOS (NeedsWeights).
//! Never emit a fabricated MOS number: learned MOS fails closed until weights are supplied.

pub mod intrusive;
pub mod mos;
