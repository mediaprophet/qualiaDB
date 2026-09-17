//! Golden corpus loader for agent evaluation.
//!
//! A golden corpus is a collection of test cases — each with an input prompt,
//! expected output (or expected substrate/relations), and optional scoring
//! criteria. The loader reads a simple text-based format (one case per
//! stanza, separated by `---`) and produces an in-memory corpus structure.
//!
//! Format:
//! ```text
//! # case: test_name
//! input: <input text>
//! expected: <expected output or relations>
//! tags: tag1,tag2
//! ---
//! # case: another_test
//! input: ...
//! expected: ...
//! ```
//!
//! Lines starting with `#` are comments (or case headers when prefixed with
//! `case:`). The `---` separator delimits cases.

use std::collections::BTreeSet;

/// A single golden test case.
#[derive(Debug, Clone, PartialEq)]
pub struct GoldenCase {
    /// Case name (from `# case:` header).
    pub name: String,
    /// Input text to feed to the agent.
    pub input: String,
    /// Expected output or extracted relations.
    pub expected: String,
    /// Optional tags for filtering.
    pub tags: BTreeSet<String>,
}

/// A loaded golden corpus.
#[derive(Debug, Clone, PartialEq)]
pub struct GoldenCorpus {
    /// Corpus name (typically derived from the file/IRI).
    pub name: String,
    /// All test cases in the corpus.
    pub cases: Vec<GoldenCase>,
}

impl GoldenCorpus {
    /// Number of cases in the corpus.
    pub fn len(&self) -> usize {
        self.cases.len()
    }

    /// Whether the corpus is empty.
    pub fn is_empty(&self) -> bool {
        self.cases.is_empty()
    }

    /// Filter cases by tag.
    pub fn cases_with_tag(&self, tag: &str) -> Vec<&GoldenCase> {
        self.cases.iter().filter(|c| c.tags.contains(tag)).collect()
    }

    /// All unique tags across all cases.
    pub fn all_tags(&self) -> BTreeSet<String> {
        let mut tags = BTreeSet::new();
        for case in &self.cases {
            tags.extend(case.tags.iter().cloned());
        }
        tags
    }
}

/// Parse a golden corpus from a text string.
///
/// The format is stanza-based: cases are separated by `---` on its own line.
/// Within a stanza, `# case:` sets the name, `input:` sets the input,
/// `expected:` sets the expected output, and `tags:` sets comma-separated tags.
/// Multi-line values are supported by indenting continuation lines.
pub fn parse_corpus(name: &str, text: &str) -> GoldenCorpus {
    let mut cases = Vec::new();
    let mut current_name = String::new();
    let mut current_input = String::new();
    let mut current_expected = String::new();
    let mut current_tags = BTreeSet::new();
    let mut current_field: Option<&str> = None;

    let flush = |cases: &mut Vec<GoldenCase>,
                 name: &mut String,
                 input: &mut String,
                 expected: &mut String,
                 tags: &mut BTreeSet<String>| {
        if !name.is_empty() || !input.is_empty() || !expected.is_empty() {
            cases.push(GoldenCase {
                name: if name.is_empty() {
                    format!("case_{}", cases.len())
                } else {
                    name.clone()
                },
                input: input.trim().to_string(),
                expected: expected.trim().to_string(),
                tags: tags.clone(),
            });
            name.clear();
            input.clear();
            expected.clear();
            tags.clear();
        }
    };

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed == "---" {
            flush(
                &mut cases,
                &mut current_name,
                &mut current_input,
                &mut current_expected,
                &mut current_tags,
            );
            current_field = None;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("# case:") {
            current_name = rest.trim().to_string();
            current_field = None;
            continue;
        }

        // Skip pure comments (but not field continuations)
        if trimmed.starts_with('#') && current_field.is_none() {
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("input:") {
            current_input = rest.trim().to_string();
            current_field = Some("input");
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("expected:") {
            current_expected = rest.trim().to_string();
            current_field = Some("expected");
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("tags:") {
            for tag in rest.split(',') {
                let t = tag.trim();
                if !t.is_empty() {
                    current_tags.insert(t.to_string());
                }
            }
            current_field = None;
            continue;
        }

        // Continuation line for the current field
        if let Some(field) = current_field {
            match field {
                "input" => {
                    current_input.push('\n');
                    current_input.push_str(trimmed);
                }
                "expected" => {
                    current_expected.push('\n');
                    current_expected.push_str(trimmed);
                }
                _ => {}
            }
        }
    }

    // Flush the last case
    flush(
        &mut cases,
        &mut current_name,
        &mut current_input,
        &mut current_expected,
        &mut current_tags,
    );

    GoldenCorpus {
        name: name.to_string(),
        cases,
    }
}

/// Load a golden corpus from a file path.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_corpus_from_file(path: &str) -> Result<GoldenCorpus, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read corpus file {path}: {e}"))?;
    let name = std::path::Path::new(path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("corpus")
        .to_string();
    Ok(parse_corpus(&name, &text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_case() {
        let text = "# case: test_one\ninput: hello world\nexpected: greeting\n";
        let corpus = parse_corpus("test", text);
        assert_eq!(corpus.len(), 1);
        assert_eq!(corpus.cases[0].name, "test_one");
        assert_eq!(corpus.cases[0].input, "hello world");
        assert_eq!(corpus.cases[0].expected, "greeting");
    }

    #[test]
    fn parse_multiple_cases() {
        let text = "\
# case: first
input: a
expected: x
---
# case: second
input: b
expected: y
";
        let corpus = parse_corpus("test", text);
        assert_eq!(corpus.len(), 2);
        assert_eq!(corpus.cases[0].name, "first");
        assert_eq!(corpus.cases[1].name, "second");
    }

    #[test]
    fn parse_with_tags() {
        let text = "\
# case: tagged
input: data
expected: result
tags: nlp,coref,important
";
        let corpus = parse_corpus("test", text);
        assert_eq!(corpus.len(), 1);
        let tags = &corpus.cases[0].tags;
        assert!(tags.contains("nlp"));
        assert!(tags.contains("coref"));
        assert!(tags.contains("important"));
    }

    #[test]
    fn parse_multiline_input() {
        let text = "\
# case: multi
input: line one
  line two
  line three
expected: result
";
        let corpus = parse_corpus("test", text);
        assert_eq!(corpus.len(), 1);
        assert!(corpus.cases[0].input.contains("line one"));
        assert!(corpus.cases[0].input.contains("line two"));
        assert!(corpus.cases[0].input.contains("line three"));
    }

    #[test]
    fn parse_empty_corpus() {
        let corpus = parse_corpus("empty", "");
        assert!(corpus.is_empty());
    }

    #[test]
    fn parse_auto_named_case() {
        let text = "input: data\nexpected: result\n";
        let corpus = parse_corpus("test", text);
        assert_eq!(corpus.len(), 1);
        assert!(corpus.cases[0].name.starts_with("case_"));
    }

    #[test]
    fn cases_with_tag_filter() {
        let text = "\
# case: a
input: x
expected: y
tags: fast
---
# case: b
input: x
expected: y
tags: slow
";
        let corpus = parse_corpus("test", text);
        let fast = corpus.cases_with_tag("fast");
        assert_eq!(fast.len(), 1);
        assert_eq!(fast[0].name, "a");
    }

    #[test]
    fn all_tags_collects_unique() {
        let text = "\
# case: a
input: x
expected: y
tags: nlp,fast
---
# case: b
input: x
expected: y
tags: nlp,slow
";
        let corpus = parse_corpus("test", text);
        let tags = corpus.all_tags();
        assert_eq!(tags.len(), 3);
        assert!(tags.contains("nlp"));
        assert!(tags.contains("fast"));
        assert!(tags.contains("slow"));
    }

    #[test]
    fn comments_ignored() {
        let text = "\
# This is a comment
# case: real
input: data
# Another comment
expected: result
";
        let corpus = parse_corpus("test", text);
        assert_eq!(corpus.len(), 1);
        assert_eq!(corpus.cases[0].name, "real");
    }
}
