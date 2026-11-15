//! Current-format Hann-windowed STFT provider and typed candidate producer.
mod provider;
mod writer;
pub use provider::*;
pub use writer::*;

#[cfg(test)]
mod tests;
