//! Bounded coreference: exact-string grouping plus an experimental pronoun sieve.
//!
//! NLP-001 is a resource/contract repair, not a coreference-quality claim.
//! Exact surface grouping is not identity. The name-to-gender list is
//! experimental and must not emit personal gender or identity facts.
//! Substrate mention generation is still one-word-per-mention; NLP-500/NLP-501
//! replace the detector and heuristic. Empty mention lists skip detection.

mod error;
mod limits;
mod resolve;
mod validate;

pub use error::CorefError;
pub use limits::{
    fnv1a64, workspace_bytes_required, CancellationToken, CorefLimits, CorefWorkStats,
    DEFAULT_MAX_ANTECEDENTS_PER_PRONOUN, DEFAULT_MAX_ANTECEDENT_CHECKS, DEFAULT_MAX_CHAINS,
    DEFAULT_MAX_MENTIONS, DEFAULT_MAX_MENTION_TEXT_BYTES, DEFAULT_MAX_SOURCE_BYTES,
    DEFAULT_MAX_WORKSPACE_BYTES, SENTINEL_BYTES,
};
pub use resolve::resolve_coreferences_into;
pub use validate::{
    aggregate_mention_text_bytes, mention_kind_from_label, offset_from_i64, offset_from_u64,
    preflight_counts, validate_mention, validate_mentions,
};

use crate::nlp::span::DocSpan;

/// Coarse mention kind, used by the pronoun sieve for agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MentionKind {
    Pronoun = 0,
    Proper = 1,
    Common = 2,
}

impl MentionKind {
    pub fn as_label(self) -> &'static str {
        match self {
            Self::Pronoun => "pronoun",
            Self::Proper => "proper",
            Self::Common => "common",
        }
    }
}

/// One coreference mention with byte-span provenance. Cold/host form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorefMention {
    pub span: DocSpan,
    pub text: String,
    pub kind: MentionKind,
}

/// A resolved coreference chain: all mentions judged co-referent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorefChain {
    pub id: u32,
    pub mentions: Vec<CorefMention>,
}

/// Validated mention used on the hot path. Text is the source slice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct ValidatedMention {
    pub start: u32,
    pub end: u32,
    pub kind: MentionKind,
}

impl ValidatedMention {
    pub fn span(self) -> DocSpan {
        DocSpan::new(self.start, self.end)
    }
}

/// Per-mention scratch record. Caller-owned; zero-initialized then filled.
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct MentionWork {
    pub parent: u32,
    pub hash: u64,
    pub norm_off: u32,
    pub norm_len: u32,
    pub scratch: u32,
}

/// Caller-owned hot-path workspace.
pub struct CorefScratch<'a> {
    pub work: &'a mut [MentionWork],
    pub order: &'a mut [u32],
    pub gather: &'a mut [u32],
    pub norm: &'a mut [u8],
}

impl CorefScratch<'_> {
    pub fn provided_bytes(&self) -> usize {
        core::mem::size_of_val(self.work)
            .saturating_add(core::mem::size_of_val(self.order))
            .saturating_add(core::mem::size_of_val(self.gather))
            .saturating_add(self.norm.len())
    }
}

/// Chain header written into a caller buffer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(C)]
pub struct CorefChainHead {
    pub id: u32,
    pub mention_start: u32,
    pub mention_count: u32,
}

/// Cold adapter: validate, allocate scratch, resolve, materialize chains.
///
/// Errors are never converted into an empty success. Empty `mentions` is a
/// documented no-detection mode and returns zero chains.
pub fn resolve_coreferences(
    source: &str,
    mentions: &[CorefMention],
) -> Result<Vec<CorefChain>, CorefError> {
    resolve_coreferences_counted(
        source,
        mentions,
        &CorefLimits::DEFAULT,
        &CancellationToken::new(),
    )
    .map(|(chains, _)| chains)
}

/// Cold adapter that also returns work counters.
pub fn resolve_coreferences_counted(
    source: &str,
    mentions: &[CorefMention],
    limits: &CorefLimits,
    cancel: &CancellationToken,
) -> Result<(Vec<CorefChain>, CorefWorkStats), CorefError> {
    let text_bytes = aggregate_mention_text_bytes(mentions, limits.max_mention_text_bytes)?;
    preflight_counts(source, mentions.len(), text_bytes, limits)?;
    if mentions.is_empty() {
        return Ok((Vec::new(), CorefWorkStats::default()));
    }
    let mut validated = vec![
        ValidatedMention {
            start: 0,
            end: 0,
            kind: MentionKind::Common
        };
        mentions.len()
    ];
    validate_mentions(source, mentions, &mut validated)?;
    let norm_cap = text_bytes;
    let required = workspace_bytes_required(mentions.len(), norm_cap);
    if required > limits.max_workspace_bytes {
        return Err(CorefError::WorkspaceTooSmall {
            required,
            provided: limits.max_workspace_bytes,
        });
    }
    let mut work = vec![MentionWork::default(); mentions.len()];
    let mut order = vec![0u32; mentions.len()];
    let mut gather = vec![0u32; mentions.len()];
    let mut norm = vec![0u8; norm_cap];
    let mut scratch = CorefScratch {
        work: &mut work,
        order: &mut order,
        gather: &mut gather,
        norm: &mut norm,
    };
    let mut heads =
        vec![CorefChainHead::default(); core::cmp::min(mentions.len(), limits.max_chains)];
    let mut ids = vec![0u32; mentions.len()];
    let mut stats = CorefWorkStats::default();
    let chain_count = resolve_coreferences_into(
        source,
        &validated,
        limits,
        &mut scratch,
        &mut heads,
        &mut ids,
        cancel,
        &mut stats,
    )?;
    let mut chains = Vec::with_capacity(chain_count);
    for head in heads.iter().take(chain_count) {
        let start = head.mention_start as usize;
        let count = head.mention_count as usize;
        let mut chain_mentions = Vec::with_capacity(count);
        for &mention_index in ids[start..start + count].iter() {
            chain_mentions.push(mentions[mention_index as usize].clone());
        }
        chains.push(CorefChain {
            id: head.id,
            mentions: chain_mentions,
        });
    }
    Ok((chains, stats))
}

#[cfg(test)]
mod tests;
