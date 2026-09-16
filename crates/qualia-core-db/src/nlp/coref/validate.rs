//! Preflight mention validation. Fail before grouping or antecedent search.

use super::error::CorefError;
use super::limits::CorefLimits;
use super::{CorefMention, MentionKind, ValidatedMention};

/// Checked conversion from a host `u64` offset.
pub fn offset_from_u64(value: u64) -> Result<u32, CorefError> {
    u32::try_from(value).map_err(|_| CorefError::OffsetOverflow)
}

/// Checked conversion from a host `i64` offset. Negative values never wrap.
pub fn offset_from_i64(value: i64) -> Result<u32, CorefError> {
    if value < 0 {
        return Err(CorefError::NegativeOffset);
    }
    u32::try_from(value).map_err(|_| CorefError::OffsetOverflow)
}

/// Parse a supported mention kind. Unknown strings fail; they are not coerced.
pub fn mention_kind_from_label(label: &str) -> Result<MentionKind, CorefError> {
    match label {
        "pronoun" => Ok(MentionKind::Pronoun),
        "proper" => Ok(MentionKind::Proper),
        "common" => Ok(MentionKind::Common),
        _ => Err(CorefError::InvalidKind),
    }
}

/// Reject oversize source/list/text before any grouping work.
pub fn preflight_counts(
    source: &str,
    mention_count: usize,
    mention_text_bytes: usize,
    limits: &CorefLimits,
) -> Result<(), CorefError> {
    if source.len() > limits.max_source_bytes {
        return Err(limits.source_too_large(source.len()));
    }
    if mention_count > limits.max_mentions {
        return Err(CorefError::TooManyMentions {
            count: mention_count,
            max: limits.max_mentions,
        });
    }
    if mention_text_bytes > limits.max_mention_text_bytes {
        return Err(CorefError::MentionTextTooLarge {
            bytes: mention_text_bytes,
            max: limits.max_mention_text_bytes,
        });
    }
    Ok(())
}

/// Sum mention-text bytes with saturating arithmetic, failing at the cap.
pub fn aggregate_mention_text_bytes(
    mentions: &[CorefMention],
    max_bytes: usize,
) -> Result<usize, CorefError> {
    let mut total = 0usize;
    for mention in mentions {
        total = total.saturating_add(mention.text.len());
        if total > max_bytes {
            return Err(CorefError::MentionTextTooLarge {
                bytes: total,
                max: max_bytes,
            });
        }
    }
    Ok(total)
}

/// Validate one mention against the immutable source. `prev` enforces document order.
pub fn validate_mention(
    source: &str,
    mention: &CorefMention,
    prev: Option<&ValidatedMention>,
) -> Result<ValidatedMention, CorefError> {
    let start = mention.span.start_utf8 as usize;
    let end = mention.span.end_utf8 as usize;
    if start >= end {
        return Err(CorefError::SpanEmpty);
    }
    if end > source.len() {
        return Err(CorefError::SpanOutOfRange);
    }
    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return Err(CorefError::SpanNotUtf8Boundary);
    }
    if &source[start..end] != mention.text.as_str() {
        return Err(CorefError::TextMismatch);
    }
    if let Some(prev) = prev {
        if mention.span.start_utf8 < prev.start
            || (mention.span.start_utf8 == prev.start && mention.span.end_utf8 < prev.end)
        {
            return Err(CorefError::UnsortedMentions);
        }
    }
    Ok(ValidatedMention {
        start: mention.span.start_utf8,
        end: mention.span.end_utf8,
        kind: mention.kind,
    })
}

/// Validate every mention after the count/byte preflight has passed.
pub fn validate_mentions(
    source: &str,
    mentions: &[CorefMention],
    out: &mut [ValidatedMention],
) -> Result<usize, CorefError> {
    if out.len() < mentions.len() {
        return Err(CorefError::OutputBufferFull);
    }
    let mut prev = None;
    for (index, mention) in mentions.iter().enumerate() {
        let validated = validate_mention(source, mention, prev.as_ref())?;
        out[index] = validated;
        prev = Some(validated);
    }
    Ok(mentions.len())
}
