//! Explicit native host grant boundary for pulse, HID and private-key effects.
//! Uses the existing PoetSnapshot state owner; no source request grants itself
//! permission, and no principal/project authentication is inferred here.
mod host;
pub use host::ScopedEffects;

#[cfg(test)]
mod tests;
