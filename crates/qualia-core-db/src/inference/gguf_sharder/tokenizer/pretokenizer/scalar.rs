#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PretokenSpan {
    pub start: u32,
    pub end: u32,
}

impl PretokenSpan {
    pub fn get<'a>(self, text: &'a str) -> Option<&'a str> {
        text.get(self.start as usize..self.end as usize)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PretokenError {
    OutputTooSmall,
    InputTooLarge,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Letter,
    Number,
    Punctuation,
    Whitespace,
}

fn classify(ch: char) -> Class {
    if ch.is_whitespace() {
        Class::Whitespace
    } else if ch.is_alphabetic() {
        Class::Letter
    } else if ch.is_numeric() {
        Class::Number
    } else {
        Class::Punctuation
    }
}

fn contraction_len(bytes: &[u8]) -> usize {
    const SUFFIXES: [&[u8]; 7] = [b"'s", b"'t", b"'re", b"'ve", b"'m", b"'ll", b"'d"];
    SUFFIXES
        .iter()
        .find_map(|suffix| bytes.starts_with(suffix).then_some(suffix.len()))
        .unwrap_or(0)
}

fn emit(
    out: &mut [PretokenSpan],
    written: &mut usize,
    start: usize,
    end: usize,
) -> Result<(), PretokenError> {
    let slot = out.get_mut(*written).ok_or(PretokenError::OutputTooSmall)?;
    *slot = PretokenSpan {
        start: u32::try_from(start).map_err(|_| PretokenError::InputTooLarge)?,
        end: u32::try_from(end).map_err(|_| PretokenError::InputTooLarge)?,
    };
    *written += 1;
    Ok(())
}

/// Scalar reference matching the SmolLM/GPT-2 split expression while returning borrowed spans.
pub fn scan_unicode(text: &str, out: &mut [PretokenSpan]) -> Result<usize, PretokenError> {
    if text.len() > u32::MAX as usize {
        return Err(PretokenError::InputTooLarge);
    }
    let bytes = text.as_bytes();
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
        if bytes[index] == b' ' && index + 1 < bytes.len() {
            let next = text[index + 1..].chars().next().unwrap();
            if !next.is_whitespace() {
                class_start += 1;
            }
        }
        let first = text[class_start..].chars().next().unwrap();
        let class = classify(first);
        index = class_start + first.len_utf8();
        while index < bytes.len() {
            let ch = text[index..].chars().next().unwrap();
            if classify(ch) != class {
                break;
            }
            index += ch.len_utf8();
        }
        emit(out, &mut written, start, index)?;
    }
    Ok(written)
}

#[cfg(not(target_arch = "x86_64"))]
pub(super) fn scan_ascii_scalar(
    bytes: &[u8],
    out: &mut [PretokenSpan],
) -> Result<usize, PretokenError> {
    // ASCII is valid UTF-8, and this is the exact reference path shared with SIMD tests.
    let text = unsafe { core::str::from_utf8_unchecked(bytes) };
    scan_unicode(text, out)
}
