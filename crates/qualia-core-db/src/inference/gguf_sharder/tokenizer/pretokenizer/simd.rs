#[cfg(not(target_arch = "x86_64"))]
use super::scalar::scan_ascii_scalar;
use super::scalar::{PretokenError, PretokenSpan};

#[inline]
pub fn avx2_available() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        std::arch::is_x86_feature_detected!("avx2")
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

#[cfg(target_arch = "x86_64")]
#[derive(Clone, Copy)]
enum AsciiClass {
    Letter,
    Number,
    Punctuation,
    Whitespace,
}

#[cfg(target_arch = "x86_64")]
fn scalar_class(byte: u8) -> AsciiClass {
    if byte.is_ascii_whitespace() {
        AsciiClass::Whitespace
    } else if byte.is_ascii_alphabetic() {
        AsciiClass::Letter
    } else if byte.is_ascii_digit() {
        AsciiClass::Number
    } else {
        AsciiClass::Punctuation
    }
}

#[cfg(target_arch = "x86_64")]
fn contraction_len(bytes: &[u8]) -> usize {
    const SUFFIXES: [&[u8]; 7] = [b"'s", b"'t", b"'re", b"'ve", b"'m", b"'ll", b"'d"];
    SUFFIXES
        .iter()
        .find_map(|suffix| bytes.starts_with(suffix).then_some(suffix.len()))
        .unwrap_or(0)
}

#[cfg(target_arch = "x86_64")]
fn emit(
    out: &mut [PretokenSpan],
    written: &mut usize,
    start: usize,
    end: usize,
) -> Result<(), PretokenError> {
    let span = out.get_mut(*written).ok_or(PretokenError::OutputTooSmall)?;
    *span = PretokenSpan {
        start: u32::try_from(start).map_err(|_| PretokenError::InputTooLarge)?,
        end: u32::try_from(end).map_err(|_| PretokenError::InputTooLarge)?,
    };
    *written += 1;
    Ok(())
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn class_mask(block: *const u8, class: AsciiClass) -> u32 {
    use std::arch::x86_64::*;

    let bytes = unsafe { _mm256_loadu_si256(block.cast::<__m256i>()) };
    let gt = |value: i8| _mm256_cmpgt_epi8(bytes, _mm256_set1_epi8(value));
    let lt = |value: i8| _mm256_cmpgt_epi8(_mm256_set1_epi8(value), bytes);
    let upper = _mm256_and_si256(gt((b'A' - 1) as i8), lt((b'Z' + 1) as i8));
    let lower = _mm256_and_si256(gt((b'a' - 1) as i8), lt((b'z' + 1) as i8));
    let letters = _mm256_or_si256(upper, lower);
    let numbers = _mm256_and_si256(gt((b'0' - 1) as i8), lt((b'9' + 1) as i8));
    let mut whitespace = _mm256_cmpeq_epi8(bytes, _mm256_set1_epi8(b' ' as i8));
    for byte in [b'\t', b'\n', 0x0b, 0x0c, b'\r'] {
        whitespace = _mm256_or_si256(
            whitespace,
            _mm256_cmpeq_epi8(bytes, _mm256_set1_epi8(byte as i8)),
        );
    }
    let selected = match class {
        AsciiClass::Letter => letters,
        AsciiClass::Number => numbers,
        AsciiClass::Whitespace => whitespace,
        AsciiClass::Punctuation => _mm256_xor_si256(
            _mm256_or_si256(_mm256_or_si256(letters, numbers), whitespace),
            _mm256_set1_epi8(-1),
        ),
    };
    _mm256_movemask_epi8(selected) as u32
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn run_end(bytes: &[u8], mut index: usize, class: AsciiClass) -> usize {
    while index + 32 <= bytes.len() {
        let mask = unsafe { class_mask(bytes.as_ptr().add(index), class) };
        if mask != u32::MAX {
            return index + (!mask).trailing_zeros() as usize;
        }
        index += 32;
    }
    while index < bytes.len() {
        let same = matches!(
            (scalar_class(bytes[index]), class),
            (AsciiClass::Letter, AsciiClass::Letter)
                | (AsciiClass::Number, AsciiClass::Number)
                | (AsciiClass::Punctuation, AsciiClass::Punctuation)
                | (AsciiClass::Whitespace, AsciiClass::Whitespace)
        );
        if !same {
            break;
        }
        index += 1;
    }
    index
}

/// AVX2 SmolLM/GPT-2 splitter for ASCII prompt spans.
///
/// Vector masks advance through 32 equal-category bytes at a time; contractions and the optional
/// leading ASCII space retain the scalar regex ordering. No temporary strings or heap storage.
#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
pub unsafe fn scan_ascii_avx2(
    bytes: &[u8],
    out: &mut [PretokenSpan],
) -> Result<usize, PretokenError> {
    if bytes.len() > u32::MAX as usize {
        return Err(PretokenError::InputTooLarge);
    }
    let mut index = 0usize;
    let mut written = 0usize;
    while index < bytes.len() {
        let contraction = contraction_len(&bytes[index..]);
        if contraction != 0 {
            emit(out, &mut written, index, index + contraction)?;
            index += contraction;
            continue;
        }
        let start = index;
        let mut class_start = index;
        if bytes[index] == b' '
            && index + 1 < bytes.len()
            && !bytes[index + 1].is_ascii_whitespace()
        {
            class_start += 1;
        }
        let class = scalar_class(bytes[class_start]);
        index = unsafe { run_end(bytes, class_start, class) };
        emit(out, &mut written, start, index)?;
    }
    Ok(written)
}

#[cfg(not(target_arch = "x86_64"))]
pub unsafe fn scan_ascii_avx2(
    bytes: &[u8],
    out: &mut [PretokenSpan],
) -> Result<usize, PretokenError> {
    scan_ascii_scalar(bytes, out)
}
