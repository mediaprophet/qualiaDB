//! Bounded exact-match grouping and pronoun sieve. Hot path: no heap.

use super::error::CorefError;
use super::limits::{workspace_bytes_required, CancellationToken, CorefLimits, CorefWorkStats};
use super::{CorefChainHead, CorefScratch, MentionKind, MentionWork, ValidatedMention};
use crate::nlp::span::DocSpan;
use core::cmp::Ordering;

/// Experimental surface-gender hint used only by the legacy pronoun sieve.
/// This is not a personal gender or identity assertion. NLP-501 replaces it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gender {
    Male,
    Female,
    Neutral,
}

pub fn resolve_coreferences_into(
    source: &str,
    mentions: &[ValidatedMention],
    limits: &CorefLimits,
    scratch: &mut CorefScratch<'_>,
    out_chains: &mut [CorefChainHead],
    out_mention_ids: &mut [u32],
    cancel: &CancellationToken,
    stats: &mut CorefWorkStats,
) -> Result<usize, CorefError> {
    *stats = CorefWorkStats::default();
    if cancel.is_cancelled() {
        return Err(CorefError::Cancelled);
    }
    let n = mentions.len();
    stats.mention_count = n as u32;
    if n > limits.max_mentions {
        return Err(CorefError::TooManyMentions {
            count: n,
            max: limits.max_mentions,
        });
    }
    if n == 0 {
        return Ok(0);
    }
    if scratch.work.len() < n || scratch.order.len() < n || scratch.gather.len() < n {
        return Err(CorefError::WorkspaceTooSmall {
            required: workspace_bytes_required(n, 0),
            provided: scratch.provided_bytes(),
        });
    }

    let (work, order, gather, norm) = (
        &mut scratch.work[..n],
        &mut scratch.order[..n],
        &mut scratch.gather[..n],
        &mut *scratch.norm,
    );

    let folded = fold_mentions(source, mentions, limits, work, norm, stats, cancel)?;
    stats.workspace_bytes = workspace_bytes_required(n, folded);
    if stats.workspace_bytes > limits.max_workspace_bytes {
        return Err(CorefError::WorkspaceTooSmall {
            required: stats.workspace_bytes,
            provided: limits.max_workspace_bytes,
        });
    }

    for (index, slot) in work.iter_mut().enumerate() {
        slot.parent = index as u32;
    }
    for (index, slot) in order.iter_mut().enumerate() {
        *slot = index as u32;
    }

    sort_keys(work, order, norm, stats, limits.max_comparison_bytes)?;
    if cancel.is_cancelled() {
        return Err(CorefError::Cancelled);
    }
    union_equal_keys(mentions, work, order, norm, stats, limits)?;
    resolve_pronouns(source, mentions, work, limits, cancel, stats)?;
    emit_chains(
        work,
        order,
        gather,
        n,
        limits.max_chains,
        out_chains,
        out_mention_ids,
        stats,
    )
}

fn fold_mentions(
    source: &str,
    mentions: &[ValidatedMention],
    limits: &CorefLimits,
    work: &mut [MentionWork],
    norm: &mut [u8],
    stats: &mut CorefWorkStats,
    cancel: &CancellationToken,
) -> Result<usize, CorefError> {
    let mut cursor = 0usize;
    for (index, mention) in mentions.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(CorefError::Cancelled);
        }
        let slice = span_slice(source, mention)?;
        let need = fold_needed_bytes(slice);
        let used = stats.normalization_bytes.saturating_add(need as u64);
        if used > limits.max_normalization_bytes {
            return Err(CorefError::NormalizationBudget {
                used,
                max: limits.max_normalization_bytes,
            });
        }
        if cursor.saturating_add(need) > norm.len() {
            return Err(CorefError::WorkspaceTooSmall {
                required: cursor.saturating_add(need),
                provided: norm.len(),
            });
        }
        let wrote = fold_into(slice, &mut norm[cursor..])?;
        debug_assert_eq!(wrote, need);
        stats.normalization_bytes = used;
        work[index].norm_off = cursor as u32;
        work[index].norm_len = wrote as u32;
        work[index].hash = (limits.key_hash)(&norm[cursor..cursor + wrote]);
        cursor += wrote;
    }
    Ok(cursor)
}

fn fold_needed_bytes(src: &str) -> usize {
    let mut n = 0usize;
    for ch in src.chars() {
        if !ch.is_alphanumeric() {
            continue;
        }
        n = n.saturating_add(ch.to_ascii_lowercase().len_utf8());
    }
    n
}

fn fold_into(src: &str, dest: &mut [u8]) -> Result<usize, CorefError> {
    let mut n = 0usize;
    for ch in src.chars() {
        if !ch.is_alphanumeric() {
            continue;
        }
        let lower = ch.to_ascii_lowercase();
        let mut buf = [0u8; 4];
        let encoded = lower.encode_utf8(&mut buf);
        let next = n.saturating_add(encoded.len());
        if next > dest.len() {
            return Err(CorefError::WorkspaceTooSmall {
                required: next,
                provided: dest.len(),
            });
        }
        dest[n..next].copy_from_slice(encoded.as_bytes());
        n = next;
    }
    Ok(n)
}

fn span_slice<'a>(source: &'a str, mention: &ValidatedMention) -> Result<&'a str, CorefError> {
    DocSpan::new(mention.start, mention.end)
        .slice(source)
        .ok_or(CorefError::SpanOutOfRange)
}

fn sort_keys(
    work: &[MentionWork],
    order: &mut [u32],
    norm: &[u8],
    stats: &mut CorefWorkStats,
    max_bytes: u64,
) -> Result<(), CorefError> {
    // Comparator must remain a total order even after the comparison budget
    // is exhausted; otherwise `sort_unstable_by` may panic. Charge bytes,
    // then fail closed after the sort.
    order.sort_unstable_by(|a, b| {
        stats.key_comparisons = stats.key_comparisons.saturating_add(1);
        let ia = *a as usize;
        let ib = *b as usize;
        work[ia]
            .hash
            .cmp(&work[ib].hash)
            .then_with(|| {
                count_and_cmp_bytes(
                    norm_slice(work, norm, ia),
                    norm_slice(work, norm, ib),
                    stats,
                )
            })
            .then(a.cmp(b))
    });
    if stats.comparison_bytes > max_bytes {
        return Err(CorefError::ComparisonBudget {
            used: stats.comparison_bytes,
            max: max_bytes,
        });
    }
    Ok(())
}

fn norm_slice<'a>(work: &[MentionWork], norm: &'a [u8], index: usize) -> &'a [u8] {
    let off = work[index].norm_off as usize;
    let len = work[index].norm_len as usize;
    &norm[off..off + len]
}

fn count_and_cmp_bytes(left: &[u8], right: &[u8], stats: &mut CorefWorkStats) -> Ordering {
    let shared = left.len().min(right.len());
    for i in 0..shared {
        stats.comparison_bytes = stats.comparison_bytes.saturating_add(1);
        if left[i] != right[i] {
            return left[i].cmp(&right[i]);
        }
    }
    left.len().cmp(&right.len())
}

fn byte_ord(
    left: &[u8],
    right: &[u8],
    stats: &mut CorefWorkStats,
    max_bytes: u64,
) -> Result<Ordering, CorefError> {
    let ord = count_and_cmp_bytes(left, right, stats);
    if stats.comparison_bytes > max_bytes {
        return Err(CorefError::ComparisonBudget {
            used: stats.comparison_bytes,
            max: max_bytes,
        });
    }
    Ok(ord)
}

fn keys_equal(
    a: usize,
    b: usize,
    work: &[MentionWork],
    norm: &[u8],
    stats: &mut CorefWorkStats,
    max_bytes: u64,
) -> Result<bool, CorefError> {
    stats.key_comparisons = stats.key_comparisons.saturating_add(1);
    if work[a].hash != work[b].hash {
        return Ok(false);
    }
    Ok(byte_ord(
        norm_slice(work, norm, a),
        norm_slice(work, norm, b),
        stats,
        max_bytes,
    )? == Ordering::Equal)
}

fn union_equal_keys(
    mentions: &[ValidatedMention],
    work: &mut [MentionWork],
    order: &[u32],
    norm: &[u8],
    stats: &mut CorefWorkStats,
    limits: &CorefLimits,
) -> Result<(), CorefError> {
    // Walk equal-key runs. Adjacent-only union would split a proper-noun run
    // when a pronoun-kind mention with the same folded key sits between them.
    let mut i = 0usize;
    while i < order.len() {
        let start = i;
        i += 1;
        while i < order.len()
            && keys_equal(
                order[start] as usize,
                order[i] as usize,
                work,
                norm,
                stats,
                limits.max_comparison_bytes,
            )?
        {
            i += 1;
        }
        let mut head: Option<usize> = None;
        for slot in &order[start..i] {
            let idx = *slot as usize;
            if mentions[idx].kind == MentionKind::Pronoun {
                continue;
            }
            match head {
                None => head = Some(idx),
                Some(h) => union(work, h, idx),
            }
        }
    }
    Ok(())
}

fn resolve_pronouns(
    source: &str,
    mentions: &[ValidatedMention],
    work: &mut [MentionWork],
    limits: &CorefLimits,
    cancel: &CancellationToken,
    stats: &mut CorefWorkStats,
) -> Result<(), CorefError> {
    for i in 0..mentions.len() {
        if cancel.is_cancelled() {
            return Err(CorefError::Cancelled);
        }
        if mentions[i].kind != MentionKind::Pronoun {
            continue;
        }
        let slice = span_slice(source, &mentions[i])?;
        let Some(gender) = pronoun_gender(slice) else {
            continue;
        };
        let mut candidates = 0usize;
        let mut j = i;
        while j > 0 {
            j -= 1;
            if mentions[j].kind != MentionKind::Proper {
                continue;
            }
            if candidates >= limits.max_antecedents_per_pronoun {
                // Search window exhausted: unresolved, not a budget error.
                break;
            }
            stats.antecedent_checks = stats.antecedent_checks.saturating_add(1);
            if stats.antecedent_checks > limits.max_antecedent_checks {
                return Err(CorefError::AntecedentBudget {
                    used: stats.antecedent_checks,
                    max: limits.max_antecedent_checks,
                });
            }
            candidates += 1;
            stats.antecedent_candidates = stats.antecedent_candidates.saturating_add(1);
            let antecedent = span_slice(source, &mentions[j])?;
            if gender_of(antecedent) == Some(gender) {
                union(work, i, j);
                break;
            }
        }
    }
    Ok(())
}

fn emit_chains(
    work: &mut [MentionWork],
    order: &mut [u32],
    gather: &mut [u32],
    n: usize,
    max_chains: usize,
    out_chains: &mut [CorefChainHead],
    out_mention_ids: &mut [u32],
    stats: &mut CorefWorkStats,
) -> Result<usize, CorefError> {
    for slot in work.iter_mut() {
        slot.scratch = u32::MAX;
        slot.hash = 0;
    }
    for i in 0..n {
        let root = find(work, i);
        work[i].parent = root as u32;
        work[root].hash = work[root].hash.saturating_add(1);
        if (i as u32) < work[root].scratch {
            work[root].scratch = i as u32;
        }
    }
    let mut n_roots = 0usize;
    for i in 0..n {
        if work[i].parent == i as u32 {
            order[n_roots] = i as u32;
            n_roots += 1;
        }
    }
    if n_roots > max_chains {
        return Err(CorefError::TooManyChains {
            count: n_roots,
            max: max_chains,
        });
    }
    if n_roots > out_chains.len() || n > out_mention_ids.len() {
        return Err(CorefError::OutputBufferFull);
    }
    order[..n_roots].sort_unstable_by_key(|&root| work[root as usize].scratch);

    let mut cursor = 0u32;
    for (id, &root) in order[..n_roots].iter().enumerate() {
        let count = work[root as usize].hash as u32;
        out_chains[id] = CorefChainHead {
            id: id as u32,
            mention_start: cursor,
            mention_count: count,
        };
        gather[root as usize] = cursor;
        cursor = cursor.saturating_add(count);
    }
    if cursor as usize > out_mention_ids.len() {
        return Err(CorefError::OutputBufferFull);
    }
    for i in 0..n {
        let root = work[i].parent as usize;
        let dest = gather[root] as usize;
        out_mention_ids[dest] = i as u32;
        gather[root] = gather[root].saturating_add(1);
    }
    stats.chain_count = n_roots as u32;
    Ok(n_roots)
}

fn find(work: &mut [MentionWork], mut x: usize) -> usize {
    while work[x].parent as usize != x {
        let parent = work[x].parent as usize;
        work[x].parent = work[parent].parent;
        x = work[x].parent as usize;
    }
    x
}

fn union(work: &mut [MentionWork], a: usize, b: usize) {
    let ra = find(work, a);
    let rb = find(work, b);
    if ra != rb {
        work[ra].parent = rb as u32;
    }
}

fn eq_ignore_ascii(text: &str, candidates: &[&str]) -> bool {
    candidates.iter().any(|c| text.eq_ignore_ascii_case(c))
}

fn pronoun_gender(text: &str) -> Option<Gender> {
    if eq_ignore_ascii(text, &["he", "him", "his", "himself"]) {
        Some(Gender::Male)
    } else if eq_ignore_ascii(text, &["she", "her", "hers", "herself"]) {
        Some(Gender::Female)
    } else if eq_ignore_ascii(text, &["it", "its", "itself"]) {
        Some(Gender::Neutral)
    } else {
        None
    }
}

/// Heuristic gender hint from a proper-noun surface. Experimental; unknown
/// names do not resolve. Do not emit identity or gender facts from this list.
fn gender_of(text: &str) -> Option<Gender> {
    if eq_ignore_ascii(
        text,
        &["john", "he", "bob", "james", "michael", "david", "richard"],
    ) {
        Some(Gender::Male)
    } else if eq_ignore_ascii(
        text,
        &[
            "mary",
            "she",
            "jane",
            "susan",
            "elizabeth",
            "alice",
            "sarah",
        ],
    ) {
        Some(Gender::Female)
    } else {
        None
    }
}
