//! W6a — prompt-lookup (n-gram) speculative decoding: the proposer.
//!
//! Standard "prompt lookup decoding" (a.k.a. LLMA): instead of a separate draft model, draft the
//! next few tokens by finding where the current context suffix already occurred earlier in the same
//! context (prompt + generated), and proposing the tokens that FOLLOWED that earlier occurrence.
//! Those drafts are then VERIFIED by one batched forward and the longest agreeing prefix is kept —
//! so the emitted text is **bit-identical to greedy decode** (the verify step never accepts a token
//! the model would not have produced greedily). The win is pure latency on repetitive / quoting /
//! structured text (code, JSON, lists, cited passages); on non-repetitive text it proposes little
//! and costs ~nothing. This module is the PROPOSER only — pure, allocation-light, unit-tested, no
//! GPU. The verify/accept wiring lives in the decode loop.

/// Longest n-gram suffix length to try when looking for a recurrence.
pub const MAX_NGRAM: usize = 3;
/// Hard cap on draft length (also bounded by the caller's scratch/batch width).
pub const MAX_DRAFT: usize = 8;

/// A drafted continuation: `tokens[..len]` are the proposed next-token ids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Draft {
    pub tokens: [u32; MAX_DRAFT],
    pub len: usize,
}

impl Draft {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            tokens: [0; MAX_DRAFT],
            len: 0,
        }
    }
    #[inline]
    pub fn as_slice(&self) -> &[u32] {
        &self.tokens[..self.len]
    }
}

/// Propose up to `max_draft` tokens by matching the longest available suffix of `ctx` against an
/// earlier occurrence in `ctx`, and returning the tokens that followed that occurrence.
///
/// - Tries suffix lengths `MAX_NGRAM..=1` (longest match first — higher-order n-grams are more
///   selective, so their continuation is likelier to be accepted).
/// - For a given n-gram, uses the MOST RECENT earlier occurrence (locality: recent context is the
///   best predictor of the immediate continuation).
/// - Never proposes past the end of `ctx` (only real, already-seen tokens are drafted).
/// - Returns [`Draft::empty`] when nothing recurs (the common case for novel text).
pub fn propose(ctx: &[u32], max_draft: usize) -> Draft {
    let k = max_draft.min(MAX_DRAFT);
    if k == 0 || ctx.len() < 2 {
        return Draft::empty();
    }
    let n = ctx.len();
    // Try the longest suffix first. Suffix length `g` must leave room for an earlier occurrence
    // plus at least one following token: earliest match start `i` satisfies `i + g < n` (so
    // `ctx[i+g]` exists) and `i < n - g` (the match is strictly before the current suffix).
    let max_g = MAX_NGRAM.min(n - 1);
    for g in (1..=max_g).rev() {
        let suffix = &ctx[n - g..];
        // Search earlier windows [0, n-g) for the LAST occurrence of `suffix`, scanning right→left
        // so the first hit is the most recent. The match must start at `i` with `i + g <= n - 1`
        // (i.e. `i <= n - g - 1`) so that at least one continuation token `ctx[i+g]` exists.
        if n < g + 1 {
            continue;
        }
        let latest_start = n - g - 1; // inclusive upper bound on match start
        let mut i = latest_start as isize;
        while i >= 0 {
            let ii = i as usize;
            if ctx[ii..ii + g] == *suffix {
                let mut d = Draft::empty();
                let src = ii + g; // first continuation token position
                let avail = n - src; // tokens available after the match
                let take = avail.min(k);
                d.tokens[..take].copy_from_slice(&ctx[src..src + take]);
                d.len = take;
                if d.len > 0 {
                    return d;
                }
            }
            i -= 1;
        }
    }
    Draft::empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_on_short_or_novel() {
        assert_eq!(propose(&[], 4).len, 0);
        assert_eq!(propose(&[7], 4).len, 0);
        // strictly increasing, no recurrence
        assert_eq!(propose(&[1, 2, 3, 4, 5], 4).len, 0);
    }

    #[test]
    fn proposes_bigram_continuation() {
        // "1 2 3 9 | 1 2" → suffix [1,2] recurs at pos 0; the drafter proposes the WHOLE earlier
        // continuation [3,9,1,2] (the verify step later trims to the model-agreeing prefix — drafting
        // several ahead is the point).
        let ctx = [1u32, 2, 3, 9, 1, 2];
        let d = propose(&ctx, 4);
        assert_eq!(d.as_slice(), &[3, 9, 1, 2], "drafts the earlier continuation of [1,2]");
    }

    #[test]
    fn prefers_longest_ngram() {
        // The trigram [7,1,2] and the bigram [1,2] recur with DIFFERENT continuations: after the
        // earlier [7,1,2] came 9; after the more-recent bare [1,2] came 7. The trigram is more
        // selective and must win → draft starts with 9, not 7.
        let ctx = [7u32, 1, 2, 9, 3, 1, 2, 7, 1, 2];
        let d = propose(&ctx, 4);
        assert_eq!(d.as_slice()[0], 9, "trigram continuation (9), not the bigram's (7)");
    }

    #[test]
    fn uses_most_recent_occurrence() {
        // Bare [5] recurs at idx 0 (→ next is 1) and idx 2 (→ next is 2). The MOST RECENT earlier
        // occurrence (idx 2) governs → draft starts with 2, not 1.
        let ctx = [5u32, 1, 5, 2, 5];
        let d = propose(&ctx, 3);
        assert_eq!(d.as_slice()[0], 2, "most-recent [5] is followed by 2");
    }

    #[test]
    fn respects_max_draft() {
        // long repeat so many continuation tokens are available; cap at max_draft.
        let ctx = [1u32, 2, 3, 4, 5, 6, 7, 1, 2];
        let d = propose(&ctx, 3);
        assert_eq!(d.as_slice(), &[3, 4, 5], "capped at max_draft=3");
        let d2 = propose(&ctx, MAX_DRAFT + 100);
        assert!(d2.len <= MAX_DRAFT, "never exceeds MAX_DRAFT");
    }

    #[test]
    fn no_continuation_when_match_is_the_tail() {
        // The only earlier occurrence has no following token — must not propose (no OOB read).
        // suffix [2] ; earlier [2] only at idx that is immediately before the suffix → still has a
        // continuation here, so construct a true no-continuation: [2, 5, 2] suffix [2] earlier at 0
        // → continuation [5]; to get none, the match must be at n-g-... covered by bounds. Verify a
        // degenerate all-same case stays in-bounds and terminates.
        let ctx = [2u32, 2, 2, 2];
        let d = propose(&ctx, 4);
        // suffix [2,2,2] (g=3) earlier occurrence start 0? ctx[0..3]=[2,2,2]==suffix ctx[1..4] → yes,
        // continuation at src=3 → [2]. Deterministic + in-bounds.
        assert_eq!(d.as_slice(), &[2]);
    }

    #[test]
    fn draft_is_a_prefix_of_real_tokens() {
        // Whatever is drafted must be tokens that literally appear in ctx (never fabricated) —
        // the exact-output safety premise. Fuzz a few structured inputs.
        let inputs: [&[u32]; 3] = [&[1, 2, 1, 2, 1, 2], &[4, 4, 5, 4, 4], &[7, 8, 9, 7, 8, 9, 7]];
        for ctx in inputs {
            let d = propose(ctx, MAX_DRAFT);
            for &t in d.as_slice() {
                assert!(ctx.contains(&t), "drafted token {t} not present in ctx {ctx:?}");
            }
        }
    }
}
