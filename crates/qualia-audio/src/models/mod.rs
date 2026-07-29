//! Learned audio heads — loaded via a common P64 loader that **fails closed** (abstains /
//! `NeedsWeights`) when weights are absent. No Candle/Burn runtime, no Python. Real weights are
//! gated on the principal (HA1 corpus, HA6 speech model). See ADR 007 + delivery plan Waves 7–8.

pub mod aed_p64;
pub mod audio_tokens;
pub mod crepe;
pub mod loader;
pub mod separation;
pub mod speech_p64;
