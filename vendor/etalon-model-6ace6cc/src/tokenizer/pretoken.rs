use super::{budget::Work, PretokenSpan, TokenizerError};

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

// Exact scalar GPT-2/SmolLM split used by the legacy scanner. Unicode property
// queries count as fixed scalar operations; every peek/contraction is metered.
pub(super) fn scan(
    text: &str,
    out: &mut [PretokenSpan],
    work: &mut Work,
) -> Result<usize, TokenizerError> {
    if text.len() > u32::MAX as usize {
        return Err(TokenizerError::ArithmeticOverflow);
    }
    let bytes = text.as_bytes();
    let (mut index, mut written) = (0, 0);
    while index < bytes.len() {
        work.charge(1)?;
        let mut contraction = 0;
        for suffix in [
            b"'s".as_slice(),
            b"'t",
            b"'re",
            b"'ve",
            b"'m",
            b"'ll",
            b"'d",
        ] {
            work.charge(1)?;
            if bytes.len() - index >= suffix.len()
                && work.equal(&bytes[index..index + suffix.len()], suffix)?
            {
                contraction = suffix.len();
                break;
            }
        }
        let start = index;
        if contraction != 0 {
            index += contraction;
        } else {
            let mut class_start = index;
            if bytes[index] == b' ' && index + 1 < bytes.len() {
                work.charge(1)?;
                let next = text[index + 1..]
                    .chars()
                    .next()
                    .ok_or(TokenizerError::Metadata)?;
                if !next.is_whitespace() {
                    class_start += 1;
                }
            }
            work.charge(1)?;
            let first = text[class_start..]
                .chars()
                .next()
                .ok_or(TokenizerError::Metadata)?;
            let class = classify(first);
            index = class_start + first.len_utf8();
            while index < bytes.len() {
                work.charge(1)?;
                let ch = text[index..]
                    .chars()
                    .next()
                    .ok_or(TokenizerError::Metadata)?;
                if classify(ch) != class {
                    break;
                }
                index += ch.len_utf8();
            }
        }
        work.charge(1)?;
        *out.get_mut(written).ok_or(TokenizerError::Capacity)? = PretokenSpan { start, end: index };
        written += 1;
    }
    Ok(written)
}
