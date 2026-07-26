//! Allocation-free SmolLM/GPT-2 pretokenization.

mod scalar;
mod simd;

pub use scalar::{scan_unicode, PretokenError, PretokenSpan};

/// Split into byte spans using AVX2 for ASCII input when available and the scalar Unicode
/// implementation otherwise.
pub fn pretokenize_into(text: &str, out: &mut [PretokenSpan]) -> Result<usize, PretokenError> {
    if text.is_ascii() && simd::avx2_available() {
        // SAFETY: runtime feature detection gates the AVX2 implementation.
        return unsafe { simd::scan_ascii_avx2(text.as_bytes(), out) };
    }
    scan_unicode(text, out)
}

#[cfg(test)]
mod tests;
