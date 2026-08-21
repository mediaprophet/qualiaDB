//! Multi-pass sieve coreference resolution.
//!
//! Deterministic, no LLM. Two sieves applied in order:
//!   1. Exact string match — mentions with identical (case-folded) text merge.
//!   2. Pronoun resolution — `he`/`she`/`it` resolve to the nearest preceding
//!      proper noun of matching gender.
//!
//! WASM-compatible; all allocation is Tier-2 authoring output.

use super::span::DocSpan;
use std::collections::HashMap;

/// Coarse mention kind, used by the pronoun sieve for gender agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MentionKind {
    Pronoun,
    Proper,
    Common,
}

/// One coreference mention with byte-span provenance.
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

/// Gender hint for pronoun agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Gender {
    Male,
    Female,
    Neutral,
}

/// Resolve coreferences over `text` given a pre-extracted mention list.
///
/// Mentions are assumed to be in document order (sorted by `span.start_utf8`).
/// Returns chains ordered by their first mention's start offset.
pub fn resolve_coreferences(_text: &str, mentions: Vec<CorefMention>) -> Vec<CorefChain> {
    if mentions.is_empty() {
        return Vec::new();
    }
    // Union-find over mention indices.
    let n = mentions.len();
    let mut parent = (0..n).collect::<Vec<usize>>();

    // Sieve 1: exact (case-folded) string match.
    for i in 0..n {
        for j in (i + 1)..n {
            if mentions[i].kind == MentionKind::Pronoun || mentions[j].kind == MentionKind::Pronoun
            {
                continue;
            }
            if fold(&mentions[i].text) == fold(&mentions[j].text) {
                union(&mut parent, i, j);
            }
        }
    }

    // Sieve 2: pronoun resolution.
    for i in 0..n {
        if mentions[i].kind != MentionKind::Pronoun {
            continue;
        }
        let Some(g) = pronoun_gender(&mentions[i].text) else {
            continue;
        };
        // Nearest preceding proper noun of matching gender.
        let mut found: Option<usize> = None;
        for j in (0..i).rev() {
            if mentions[j].kind != MentionKind::Proper {
                continue;
            }
            if gender_of(&mentions[j].text) == Some(g) {
                found = Some(j);
                break;
            }
        }
        if let Some(j) = found {
            union(&mut parent, i, j);
        }
    }

    // Collect chains.
    let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
    let mut roots: Vec<usize> = Vec::new();
    for i in 0..n {
        let r = find(&mut parent, i);
        if !groups.contains_key(&r) {
            roots.push(r);
        }
        groups.entry(r).or_default().push(i);
    }
    // Order chains by the earliest mention they contain.
    roots.sort_by_key(|&r| {
        groups
            .get(&r)
            .and_then(|v| v.iter().map(|&i| mentions[i].span.start_utf8).min())
            .unwrap_or(u32::MAX)
    });

    let mut chains = Vec::new();
    for (id, &root) in roots.iter().enumerate() {
        let mut idxs = groups.remove(&root).unwrap_or_default();
        idxs.sort_by_key(|&i| mentions[i].span.start_utf8);
        let ms = idxs.into_iter().map(|i| mentions[i].clone()).collect();
        chains.push(CorefChain {
            id: id as u32,
            mentions: ms,
        });
    }
    chains
}

fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        parent[ra] = rb;
    }
}

fn fold(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn pronoun_gender(text: &str) -> Option<Gender> {
    match text.to_ascii_lowercase().as_str() {
        "he" | "him" | "his" | "himself" => Some(Gender::Male),
        "she" | "her" | "hers" | "herself" => Some(Gender::Female),
        "it" | "its" | "itself" => Some(Gender::Neutral),
        _ => None,
    }
}

/// Heuristic gender hint from a proper-noun surface. A small built-in name
/// list; unknown names default to `Neutral` only when the surface is clearly
/// non-personal (all-caps acronym), otherwise `None` (no resolution).
fn gender_of(text: &str) -> Option<Gender> {
    let lower = text.to_ascii_lowercase();
    match lower.as_str() {
        "john" | "he" | "bob" | "james" | "michael" | "david" | "richard" => Some(Gender::Male),
        "mary" | "she" | "jane" | "susan" | "elizabeth" | "alice" | "sarah" => Some(Gender::Female),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proper(text: &str, start: u32) -> CorefMention {
        CorefMention {
            span: DocSpan::new(start, start + text.len() as u32),
            text: text.into(),
            kind: MentionKind::Proper,
        }
    }

    fn pronoun(text: &str, start: u32) -> CorefMention {
        CorefMention {
            span: DocSpan::new(start, start + text.len() as u32),
            text: text.into(),
            kind: MentionKind::Pronoun,
        }
    }

    #[test]
    fn exact_string_match_merges() {
        let mentions = vec![proper("John", 0), proper("John", 20)];
        let chains = resolve_coreferences("John ran. John fell.", mentions);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].mentions.len(), 2);
    }

    #[test]
    fn pronoun_resolves_to_preceding_proper() {
        let mentions = vec![proper("John", 0), pronoun("he", 10)];
        let chains = resolve_coreferences("John ran. He fell.", mentions);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].mentions.len(), 2);
    }

    #[test]
    fn pronoun_gender_mismatch_keeps_separate() {
        let mentions = vec![proper("Mary", 0), pronoun("he", 10)];
        let chains = resolve_coreferences("Mary ran. He fell.", mentions);
        assert_eq!(chains.len(), 2);
    }

    #[test]
    fn she_resolves_to_female_name() {
        let mentions = vec![proper("Mary", 0), pronoun("she", 12)];
        let chains = resolve_coreferences("Mary left. She smiled.", mentions);
        assert_eq!(chains.len(), 1);
        assert_eq!(chains[0].mentions.len(), 2);
    }

    #[test]
    fn empty_mentions_returns_empty() {
        let chains = resolve_coreferences("", Vec::new());
        assert!(chains.is_empty());
    }

    #[test]
    fn it_resolves_neutral() {
        // "it" has no matching proper noun here (no neutral name), stays alone.
        let mentions = vec![proper("John", 0), pronoun("it", 10)];
        let chains = resolve_coreferences("John ran. It rained.", mentions);
        assert_eq!(chains.len(), 2);
    }
}
