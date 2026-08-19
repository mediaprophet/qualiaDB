//! CST trivia — comments, whitespace, and exact token text (T36).
//!
//! The AST drops comments and whitespace. The CST preserves them so
//! that `poet translate` and `poet format` can round-trip source text
//! without destroying commentary.
//!
//! ## Design
//!
//! - [`Trivia`] represents a comment or whitespace gap.
//! - [`TriviaSink`] collects trivia as the lexer advances.
//! - [`CstNode`] wraps an AST node with its leading and trailing trivia.
//!
//! The trivia sink is optional — the lexer can run in "AST mode"
//! (trivia discarded, current behaviour) or "CST mode" (trivia
//! collected). This avoids changing existing behaviour.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` §8.8 T36.

use crate::span::Span;

/// The kind of trivia.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TriviaKind {
    /// A line comment: `// ...`
    LineComment,
    /// A block comment: `/* ... */`
    BlockComment,
    /// Whitespace (spaces, tabs, newlines).
    Whitespace,
}

/// A single trivia element — a comment or whitespace gap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Trivia {
    /// The kind of trivia.
    pub kind: TriviaKind,
    /// The exact source text of this trivia (including delimiters).
    pub text: String,
    /// The span of this trivia in the source.
    pub span: Span,
}

impl Trivia {
    /// Create a line comment trivia.
    pub fn line_comment(text: &str, span: Span) -> Self {
        Self {
            kind: TriviaKind::LineComment,
            text: text.to_string(),
            span,
        }
    }

    /// Create a block comment trivia.
    pub fn block_comment(text: &str, span: Span) -> Self {
        Self {
            kind: TriviaKind::BlockComment,
            text: text.to_string(),
            span,
        }
    }

    /// Create a whitespace trivia.
    pub fn whitespace(text: &str, span: Span) -> Self {
        Self {
            kind: TriviaKind::Whitespace,
            text: text.to_string(),
            span,
        }
    }

    /// Is this trivia a comment?
    pub fn is_comment(&self) -> bool {
        matches!(self.kind, TriviaKind::LineComment | TriviaKind::BlockComment)
    }

    /// Is this trivia whitespace?
    pub fn is_whitespace(&self) -> bool {
        matches!(self.kind, TriviaKind::Whitespace)
    }
}

/// A sink that collects trivia as the lexer advances.
/// Use [`TriviaSink::new`] to create a collecting sink, or
/// [`TriviaSink::disabled`] to create a discarding sink.
#[derive(Debug, Clone)]
pub struct TriviaSink {
    entries: Vec<Trivia>,
    enabled: bool,
}

impl Default for TriviaSink {
    fn default() -> Self {
        Self::disabled()
    }
}

impl TriviaSink {
    /// Create a trivia sink that collects trivia.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            enabled: true,
        }
    }

    /// Create a trivia sink that discards trivia (AST mode).
    pub fn disabled() -> Self {
        Self {
            entries: Vec::new(),
            enabled: false,
        }
    }

    /// Is this sink collecting trivia?
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record a trivia element. If the sink is disabled, this is a no-op.
    pub fn record(&mut self, trivia: Trivia) {
        if self.enabled {
            self.entries.push(trivia);
        }
    }

    /// Record a line comment.
    pub fn record_line_comment(&mut self, text: &str, span: Span) {
        if self.enabled {
            self.record(Trivia::line_comment(text, span));
        }
    }

    /// Record a block comment.
    pub fn record_block_comment(&mut self, text: &str, span: Span) {
        if self.enabled {
            self.record(Trivia::block_comment(text, span));
        }
    }

    /// Record whitespace.
    pub fn record_whitespace(&mut self, text: &str, span: Span) {
        if self.enabled {
            self.record(Trivia::whitespace(text, span));
        }
    }

    /// Get all collected trivia.
    pub fn entries(&self) -> &[Trivia] {
        &self.entries
    }

    /// Take all collected trivia, leaving the sink empty.
    pub fn take(&mut self) -> Vec<Trivia> {
        std::mem::take(&mut self.entries)
    }

    /// Number of trivia entries collected.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the sink empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get only the comment trivia (line + block).
    pub fn comments(&self) -> Vec<&Trivia> {
        self.entries.iter().filter(|t| t.is_comment()).collect()
    }

    /// Get only the whitespace trivia.
    pub fn whitespace(&self) -> Vec<&Trivia> {
        self.entries.iter().filter(|t| t.is_whitespace()).collect()
    }
}

/// A CST node — an AST node with its leading and trailing trivia.
///
/// This is a generic wrapper that preserves the trivia around a node
/// so that the source can be reconstructed.
#[derive(Debug, Clone)]
pub struct CstNode<T> {
    /// The AST node itself.
    pub node: T,
    /// Trivia before this node (comments + whitespace).
    pub leading: Vec<Trivia>,
    /// Trivia after this node (comments + whitespace).
    pub trailing: Vec<Trivia>,
}

impl<T> CstNode<T> {
    pub fn new(node: T, leading: Vec<Trivia>, trailing: Vec<Trivia>) -> Self {
        Self { node, leading, trailing }
    }

    /// Get the leading comments.
    pub fn leading_comments(&self) -> Vec<&Trivia> {
        self.leading.iter().filter(|t| t.is_comment()).collect()
    }

    /// Get the trailing comments.
    pub fn trailing_comments(&self) -> Vec<&Trivia> {
        self.trailing.iter().filter(|t| t.is_comment()).collect()
    }
}

/// Extract trivia from source text by scanning for comments.
///
/// This is a standalone function that doesn't require the lexer to be
/// in CST mode. It scans the source text and returns all comments
/// found, with their spans. This is useful for tools that need to
/// extract comments from already-parsed source.
pub fn extract_comments(source: &str) -> Vec<Trivia> {
    let bytes = source.as_bytes();
    let mut comments = Vec::new();
    let mut pos = 0;

    while pos < bytes.len() {
        // Line comment
        if pos + 1 < bytes.len() && bytes[pos] == b'/' && bytes[pos + 1] == b'/' {
            let start = pos;
            pos += 2;
            while pos < bytes.len() && bytes[pos] != b'\n' && bytes[pos] != b'\r' {
                pos += 1;
            }
            let text = &source[start..pos];
            comments.push(Trivia::line_comment(
                text,
                Span::new(start as u32, pos as u32),
            ));
            continue;
        }

        // Block comment
        if pos + 1 < bytes.len() && bytes[pos] == b'/' && bytes[pos + 1] == b'*' {
            let start = pos;
            pos += 2;
            let mut closed = false;
            while pos + 1 < bytes.len() {
                if bytes[pos] == b'*' && bytes[pos + 1] == b'/' {
                    pos += 2;
                    closed = true;
                    break;
                }
                pos += 1;
            }
            if !closed {
                // Unterminated block comment — include to end.
                pos = bytes.len();
            }
            let text = &source[start..pos];
            comments.push(Trivia::block_comment(
                text,
                Span::new(start as u32, pos as u32),
            ));
            continue;
        }

        // Skip string literals (don't look for comments inside strings)
        if bytes[pos] == b'"' {
            pos += 1;
            while pos < bytes.len() && bytes[pos] != b'"' {
                if bytes[pos] == b'\\' && pos + 1 < bytes.len() {
                    pos += 2;
                } else {
                    pos += 1;
                }
            }
            if pos < bytes.len() {
                pos += 1; // skip closing quote
            }
            continue;
        }

        pos += 1;
    }

    comments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trivia_line_comment() {
        let t = Trivia::line_comment("// hello", Span::new(0, 8));
        assert!(t.is_comment());
        assert!(!t.is_whitespace());
        assert_eq!(t.kind, TriviaKind::LineComment);
        assert_eq!(t.text, "// hello");
    }

    #[test]
    fn trivia_block_comment() {
        let t = Trivia::block_comment("/* hi */", Span::new(0, 8));
        assert!(t.is_comment());
        assert_eq!(t.kind, TriviaKind::BlockComment);
    }

    #[test]
    fn trivia_whitespace() {
        let t = Trivia::whitespace("  \n  ", Span::new(0, 5));
        assert!(t.is_whitespace());
        assert!(!t.is_comment());
        assert_eq!(t.kind, TriviaKind::Whitespace);
    }

    #[test]
    fn trivia_sink_disabled_discards() {
        let mut sink = TriviaSink::disabled();
        sink.record_line_comment("// test", Span::point(0));
        assert!(sink.is_empty());
        assert!(!sink.is_enabled());
    }

    #[test]
    fn trivia_sink_enabled_collects() {
        let mut sink = TriviaSink::new();
        sink.record_line_comment("// hello", Span::new(0, 8));
        sink.record_block_comment("/* world */", Span::new(10, 21));
        sink.record_whitespace("  ", Span::new(22, 24));
        assert_eq!(sink.len(), 3);
        assert!(sink.is_enabled());
    }

    #[test]
    fn trivia_sink_comments_filter() {
        let mut sink = TriviaSink::new();
        sink.record_line_comment("// c1", Span::point(0));
        sink.record_whitespace("  ", Span::point(5));
        sink.record_block_comment("/* c2 */", Span::point(7));
        let comments = sink.comments();
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text, "// c1");
        assert_eq!(comments[1].text, "/* c2 */");
    }

    #[test]
    fn trivia_sink_whitespace_filter() {
        let mut sink = TriviaSink::new();
        sink.record_line_comment("// c1", Span::point(0));
        sink.record_whitespace("  ", Span::point(5));
        sink.record_whitespace("\n", Span::point(7));
        let ws = sink.whitespace();
        assert_eq!(ws.len(), 2);
    }

    #[test]
    fn trivia_sink_take() {
        let mut sink = TriviaSink::new();
        sink.record_line_comment("// c1", Span::point(0));
        let taken = sink.take();
        assert_eq!(taken.len(), 1);
        assert!(sink.is_empty());
    }

    #[test]
    fn cst_node_construction() {
        let node = 42i32;
        let leading = vec![Trivia::line_comment("// pre", Span::point(0))];
        let trailing = vec![Trivia::line_comment("// post", Span::point(10))];
        let cst = CstNode::new(node, leading, trailing);
        assert_eq!(cst.node, 42);
        assert_eq!(cst.leading.len(), 1);
        assert_eq!(cst.trailing.len(), 1);
        assert_eq!(cst.leading_comments().len(), 1);
        assert_eq!(cst.trailing_comments().len(), 1);
    }

    #[test]
    fn extract_comments_line_comments() {
        let src = "// first\nlet x = 1; // second\n";
        let comments = extract_comments(src);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text, "// first");
        assert_eq!(comments[1].text, "// second");
    }

    #[test]
    fn extract_comments_block_comments() {
        let src = "/* a */ let x = 1; /* b */";
        let comments = extract_comments(src);
        assert_eq!(comments.len(), 2);
        assert_eq!(comments[0].text, "/* a */");
        assert_eq!(comments[1].text, "/* b */");
    }

    #[test]
    fn extract_comments_mixed() {
        let src = "// line\n/* block */\nlet x = 1; // trailing\n";
        let comments = extract_comments(src);
        assert_eq!(comments.len(), 3);
        assert!(comments[0].is_comment());
        assert!(comments[1].is_comment());
        assert!(comments[2].is_comment());
    }

    #[test]
    fn extract_comments_ignores_comments_in_strings() {
        let src = "let s = \"// not a comment\"; // real comment";
        let comments = extract_comments(src);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "// real comment");
    }

    #[test]
    fn extract_comments_empty_source() {
        let comments = extract_comments("");
        assert!(comments.is_empty());
    }

    #[test]
    fn extract_comments_no_comments() {
        let comments = extract_comments("let x = 1; let y = 2;");
        assert!(comments.is_empty());
    }

    #[test]
    fn extract_comments_unterminated_block() {
        let comments = extract_comments("/* never closed");
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].text, "/* never closed");
    }

    #[test]
    fn extract_comments_spans_correct() {
        let src = "// hi\nlet x = 1;";
        let comments = extract_comments(src);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].span.start, 0);
        assert_eq!(comments[0].span.end, 5); // "// hi"
    }
}
