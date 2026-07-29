//! Mel / MFCC / DCT / Bark / ERB psychoacoustic features.
//!
//! The **real** triangular filterbanks (this module) replace the fake linear-band
//! "log_mel" averaging in `stft_stream.rs`. Banks are precomputed once into a
//! caller-owned weight table ([`build_mel_bank`] / [`build_bark_bank`] /
//! [`build_erb_bank`]) and applied allocation-free per frame ([`mel_bands`],
//! [`mfcc`], [`bfcc`], [`gfcc`]). Re-exports only.

pub mod bark_bank;
pub mod bfcc;
pub mod dct;
pub mod erb_bank;
pub mod gfcc;
pub mod hz_mel;
pub mod mel_bands;
pub mod mel_bank;
pub mod mel_to_hz;
pub mod mfcc;

pub use bark_bank::build_bark_bank;
pub use bfcc::bfcc;
pub use dct::dct2;
pub use erb_bank::build_erb_bank;
pub use gfcc::gfcc;
pub use hz_mel::hz_to_mel;
pub use mel_bands::mel_bands;
pub use mel_bank::build_mel_bank;
pub use mel_to_hz::mel_to_hz;
pub use mfcc::mfcc;
