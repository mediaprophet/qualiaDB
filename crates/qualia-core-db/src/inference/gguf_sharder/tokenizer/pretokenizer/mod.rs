//! Allocation-free SmolLM/GPT-2 pretokenization.

mod scalar;
mod simd;

pub use scalar::{scan_unicode, scan_unicode_qwen35, PretokenError, PretokenSpan};

/// Split into byte spans using AVX2 for ASCII input when available and the scalar Unicode
/// implementation otherwise.
pub fn pretokenize_into(text: &str, out: &mut [PretokenSpan]) -> Result<usize, PretokenError> {
    if text.is_ascii() && simd::avx2_available() {
        // SAFETY: runtime feature detection gates the AVX2 implementation.
        return unsafe { simd::scan_ascii_avx2(text.as_bytes(), out) };
    }
    scan_unicode(text, out)
}

/// Dispatch on the GGUF `tokenizer.ggml.pre` type.  `qwen35` has its own
/// scalar scanner because its split rule differs structurally from the
/// GPT-2/SmolLM family (single-digit spans, letter runs that absorb a
/// leading non-letter byte and combining marks, punctuation runs that
/// absorb trailing newlines).  Unknown types keep the GPT-2 path.
pub fn pretokenize_for(
    pre_type: &str,
    text: &str,
    out: &mut [PretokenSpan],
) -> Result<usize, PretokenError> {
    if pre_type.eq_ignore_ascii_case("qwen35") {
        return scan_unicode_qwen35(text, out);
    }
    pretokenize_into(text, out)
}

#[cfg(test)]
mod tests;
