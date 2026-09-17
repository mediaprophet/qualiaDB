//! Aho-Corasick gazetteer over interned lexicon surfaces.

use std::collections::HashMap;
use std::collections::VecDeque;

use super::span::DocSpan;
use super::terms::{Lexeme, DEFAULT_LEXICON};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub span: DocSpan,
    pub iri: &'static str,
    pub surface: &'static str,
}

struct Node {
    next: HashMap<u8, u16>,
    fail: u16,
    out: Vec<u16>,
}

/// Compiled automaton. Cold construction; scan is a byte walk.
pub struct Gazetteer {
    nodes: Vec<Node>,
    patterns: Vec<&'static Lexeme>,
}

impl Default for Gazetteer {
    fn default() -> Self {
        Self::from_lexicon(DEFAULT_LEXICON)
    }
}

impl Gazetteer {
    pub fn from_lexicon(lex: &'static [Lexeme]) -> Self {
        let mut nodes = vec![Node {
            next: HashMap::new(),
            fail: 0,
            out: Vec::new(),
        }];
        let mut patterns = Vec::new();
        for lexeme in lex {
            let id = patterns.len() as u16;
            patterns.push(lexeme);
            let mut state = 0u16;
            for &b in ascii_fold(lexeme.surface).as_bytes() {
                if let Some(&n) = nodes[state as usize].next.get(&b) {
                    state = n;
                } else {
                    let next_id = nodes.len() as u16;
                    nodes.push(Node {
                        next: HashMap::new(),
                        fail: 0,
                        out: Vec::new(),
                    });
                    nodes[state as usize].next.insert(b, next_id);
                    state = next_id;
                }
            }
            nodes[state as usize].out.push(id);
        }
        build_fail_links(&mut nodes);
        Self { nodes, patterns }
    }

    /// Number of patterns in the compiled automaton.
    pub fn pattern_count(&self) -> usize {
        self.patterns.len()
    }

    pub fn find(&self, source: &str) -> Vec<Hit> {
        let folded = ascii_fold(source);
        let bytes = folded.as_bytes();
        let mut state = 0u16;
        let mut raw = Vec::new();
        for (i, &b) in bytes.iter().enumerate() {
            while state != 0 && !self.nodes[state as usize].next.contains_key(&b) {
                state = self.nodes[state as usize].fail;
            }
            if let Some(&n) = self.nodes[state as usize].next.get(&b) {
                state = n;
            }
            let mut walk = state;
            loop {
                for &pid in &self.nodes[walk as usize].out {
                    let lex = self.patterns[pid as usize];
                    let len = lex.surface.len();
                    if i + 1 < len {
                        continue;
                    }
                    let start = i + 1 - len;
                    if !byte_boundary_ok(source, start, i + 1) {
                        continue;
                    }
                    raw.push(Hit {
                        span: DocSpan::new(start as u32, (i + 1) as u32),
                        iri: lex.iri,
                        surface: lex.surface,
                    });
                }
                if walk == 0 {
                    break;
                }
                walk = self.nodes[walk as usize].fail;
            }
        }
        prefer_longest_nonoverlapping(raw)
    }
}

fn build_fail_links(nodes: &mut [Node]) {
    let mut q = VecDeque::new();
    let roots: Vec<(u8, u16)> = nodes[0].next.iter().map(|(&b, &n)| (b, n)).collect();
    for (_, n) in roots {
        nodes[n as usize].fail = 0;
        q.push_back(n);
    }
    while let Some(v) = q.pop_front() {
        let edges: Vec<(u8, u16)> = nodes[v as usize]
            .next
            .iter()
            .map(|(&b, &n)| (b, n))
            .collect();
        for (b, u) in edges {
            let mut f = nodes[v as usize].fail;
            while f != 0 && !nodes[f as usize].next.contains_key(&b) {
                f = nodes[f as usize].fail;
            }
            let fail = nodes[f as usize].next.get(&b).copied().unwrap_or(0);
            nodes[u as usize].fail = fail;
            let inherited = nodes[fail as usize].out.clone();
            nodes[u as usize].out.extend(inherited);
            q.push_back(u);
        }
    }
}

fn ascii_fold(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_alphabetic() {
                c.to_ascii_lowercase()
            } else {
                c
            }
        })
        .collect()
}

fn byte_boundary_ok(source: &str, start: usize, end: usize) -> bool {
    if start > end || end > source.len() {
        return false;
    }
    if !source.is_char_boundary(start) || !source.is_char_boundary(end) {
        return false;
    }
    let prev_ok = start == 0 || !is_word_byte(source.as_bytes()[start - 1]);
    let next_ok = end == source.len() || !is_word_byte(source.as_bytes()[end]);
    prev_ok && next_ok
}

fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn prefer_longest_nonoverlapping(mut hits: Vec<Hit>) -> Vec<Hit> {
    hits.sort_by(|a, b| {
        a.span
            .start_utf8
            .cmp(&b.span.start_utf8)
            .then_with(|| b.span.end_utf8.cmp(&a.span.end_utf8))
    });
    let mut out = Vec::new();
    let mut cursor = 0u32;
    for hit in hits {
        if hit.span.start_utf8 >= cursor {
            cursor = hit.span.end_utf8;
            out.push(hit);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_north_spring_not_bare_north() {
        let g = Gazetteer::default();
        let src = "North Spring is the reference catchment.";
        let hits = g.find(src);
        assert!(hits.iter().any(|h| h.surface == "North Spring"));
        assert!(hits.iter().any(|h| h.surface == "reference catchment"));
        assert!(!hits
            .iter()
            .any(|h| h.surface == "catchment" && h.span.start_utf8 > 20));
    }

    #[test]
    fn principal_identity_only() {
        let g = Gazetteer::default();
        let hits = g.find("Recorded by Timothy Charles Holborn.");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].iri, "did:qualia:timothy_charles_holborn");
    }
}
