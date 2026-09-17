//! vibe-0.1 lexer. IRI vs relational `<` follows vibescript-core.md §2.

use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Eof,
    Ident,
    Keyword,
    String,
    Int,
    Float,
    Iri,
    QueryVar,
    TripleStart,
    TripleEnd,
    ReifyStart,
    ReifyEnd,
    ForbiddenQuin,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Colon,
    Dot,
    Question,
    Eq,
    EqEq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    AmpAmp,
    PipePipe,
    PipeGt,
    /// Bare `|` — lambda delimiter (`|x| x * x`).
    Pipe,
    Quantity,
    Bang,
    FatArrow,
    ThinArrow,
    Tilde,
    /// `<->` two-way bind.
    Bidirectional,
    ColonEq,
    InterpolatedString,
    Color,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    /// True when the last skipped region included whitespace (not comments only).
    last_gap_ws: bool,
    /// Extra keyword locales beyond English. English is always active.
    extra_locales: Vec<crate::locale::Locale>,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        let src = strip_bom(src);
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            last_gap_ws: true,
            extra_locales: Vec::new(),
        }
    }

    /// Activate a second (or further) keyword locale for this source.
    /// English keywords remain recognized.
    pub fn enable_locale(&mut self, loc: crate::locale::Locale) {
        if loc != crate::locale::Locale::EN && !self.extra_locales.contains(&loc) {
            self.extra_locales.push(loc);
        }
    }

    pub fn source(&self) -> &'a str {
        self.src
    }

    pub fn next_token(&mut self) -> Result<Token, Diagnostic> {
        self.skip_ws_and_comments()?;
        let start = self.pos;
        if self.pos >= self.bytes.len() {
            return Ok(Token {
                kind: TokenKind::Eof,
                span: Span::point(start as u32),
            });
        }
        let b = self.bytes[self.pos];
        match b {
            b'"' => self.lex_string(start),
            b'0'..=b'9' => self.lex_number(start),
            b'?' => self.lex_question(start),
            b'<' => self.lex_lt(start),
            b'>' => self.lex_gt(start),
            b'(' => self.bump_simple(TokenKind::LParen, start),
            b')' => self.lex_rparen(start),
            b'[' => self.bump_simple(TokenKind::LBracket, start),
            b']' => self.bump_simple(TokenKind::RBracket, start),
            b'{' => self.bump_simple(TokenKind::LBrace, start),
            b'}' => self.bump_simple(TokenKind::RBrace, start),
            b',' => self.bump_simple(TokenKind::Comma, start),
            b';' => self.bump_simple(TokenKind::Semicolon, start),
            b':' => {
                if self.peek_at(1) == Some(b'=') {
                    self.pos += 2;
                    Ok(self.tok(TokenKind::ColonEq, start))
                } else {
                    self.bump_simple(TokenKind::Colon, start)
                }
            }
            b'.' => self.bump_simple(TokenKind::Dot, start),
            b'~' => self.bump_simple(TokenKind::Tilde, start),
            b'+' => self.bump_simple(TokenKind::Plus, start),
            b'-' => self.lex_minus(start),
            b'*' => self.bump_simple(TokenKind::Star, start),
            b'/' => self.bump_simple(TokenKind::Slash, start),
            b'%' => self.bump_simple(TokenKind::Percent, start),
            b'=' => self.lex_eq(start),
            b'!' => self.lex_bang(start),
            b'&' => self.lex_amp(start),
            b'|' => self.lex_pipe(start),
            b'_' if self.peek_at(1) == Some(b':') => self.lex_blank(start),
            b'f' if self.peek_at(1) == Some(b'"') => self.lex_interpolated_string(start),
            b'#' => self.lex_color(start),
            b'A'..=b'Z' | b'a'..=b'z' | b'_' => self.lex_ident_or_kw(start),
            // T40: Unicode identifiers — non-ASCII bytes that could be
            // XID_Continue start a Unicode identifier. The byte must be
            // a UTF-8 lead byte (0xC0–0xFF).
            b if crate::unicode_ident::could_be_unicode_ident_start(b) => {
                self.lex_unicode_ident_or_kw(start)
            }
            _ => Err(Diagnostic::new(
                DiagCode::E001,
                Span::new(start as u32, (start + 1) as u32),
                format!("unexpected character {:?}", b as char),
            )),
        }
    }

    fn skip_ws_and_comments(&mut self) -> Result<(), Diagnostic> {
        self.last_gap_ws = false;
        loop {
            let before = self.pos;
            while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
                self.pos += 1;
            }
            if self.pos > before {
                self.last_gap_ws = true;
            }
            if self.pos + 1 < self.bytes.len()
                && self.bytes[self.pos] == b'/'
                && self.bytes[self.pos + 1] == b'/'
            {
                self.pos += 2;
                while self.pos < self.bytes.len()
                    && self.bytes[self.pos] != b'\n'
                    && self.bytes[self.pos] != b'\r'
                {
                    self.pos += 1;
                }
                continue;
            }
            if self.pos + 1 < self.bytes.len()
                && self.bytes[self.pos] == b'/'
                && self.bytes[self.pos + 1] == b'*'
            {
                let start = self.pos;
                self.pos += 2;
                let mut closed = false;
                while self.pos + 1 < self.bytes.len() {
                    if self.bytes[self.pos] == b'*' && self.bytes[self.pos + 1] == b'/' {
                        self.pos += 2;
                        closed = true;
                        break;
                    }
                    self.pos += 1;
                }
                if !closed {
                    return Err(Diagnostic::new(
                        DiagCode::E001,
                        Span::new(start as u32, self.pos as u32),
                        "unclosed block comment",
                    ));
                }
                continue;
            }
            break;
        }
        Ok(())
    }

    fn lex_string(&mut self, start: usize) -> Result<Token, Diagnostic> {
        self.pos += 1;
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c == b'"' {
                self.pos += 1;
                return Ok(self.tok(TokenKind::String, start));
            }
            if c == b'\\' {
                self.pos += 1;
                if self.pos >= self.bytes.len() {
                    break;
                }
                self.pos += 1;
                continue;
            }
            if c == b'\n' {
                return Err(Diagnostic::new(
                    DiagCode::E001,
                    Span::new(start as u32, self.pos as u32),
                    "unterminated string",
                ));
            }
            self.pos += 1;
        }
        Err(Diagnostic::new(
            DiagCode::E001,
            Span::new(start as u32, self.pos as u32),
            "unterminated string",
        ))
    }

    fn lex_interpolated_string(&mut self, start: usize) -> Result<Token, Diagnostic> {
        self.pos += 2; // skip f"
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            if c == b'"' {
                self.pos += 1;
                return Ok(self.tok(TokenKind::InterpolatedString, start));
            }
            if c == b'\\' {
                self.pos += 1;
                if self.pos >= self.bytes.len() {
                    break;
                }
                self.pos += 1;
                continue;
            }
            if c == b'\n' {
                return Err(Diagnostic::new(
                    DiagCode::E001,
                    Span::new(start as u32, self.pos as u32),
                    "unterminated interpolated string",
                ));
            }
            self.pos += 1;
        }
        Err(Diagnostic::new(
            DiagCode::E001,
            Span::new(start as u32, self.pos as u32),
            "unterminated interpolated string",
        ))
    }

    fn lex_number(&mut self, start: usize) -> Result<Token, Diagnostic> {
        if self.bytes[self.pos] == b'0' && self.peek_at(1) == Some(b'x') {
            self.pos += 2;
            while matches!(
                self.peek(),
                Some(b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F' | b'_')
            ) {
                self.pos += 1;
            }
            self.skip_int_suffix();
            return Ok(self.tok(TokenKind::Int, start));
        }
        while matches!(self.peek(), Some(b'0'..=b'9' | b'_')) {
            self.pos += 1;
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') && matches!(self.peek_at(1), Some(b'0'..=b'9')) {
            is_float = true;
            self.pos += 1;
            while matches!(self.peek(), Some(b'0'..=b'9' | b'_')) {
                self.pos += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.pos += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            while matches!(self.peek(), Some(b'0'..=b'9' | b'_')) {
                self.pos += 1;
            }
        }

        // Check for unit suffix e.g. 500ms, 60fps, 2.4GHz, 100kPa, 90deg, 15.0[m/s], 80%
        if self.peek() == Some(b'[') {
            self.pos += 1;
            while self.pos < self.bytes.len()
                && self.bytes[self.pos] != b']'
                && self.bytes[self.pos] != b'\n'
            {
                self.pos += 1;
            }
            if self.peek() == Some(b']') {
                self.pos += 1;
                return Ok(self.tok(TokenKind::Quantity, start));
            }
        } else if self.peek() == Some(b'%') {
            self.pos += 1;
            return Ok(self.tok(TokenKind::Quantity, start));
        } else if matches!(self.peek(), Some(b'a'..=b'z' | b'A'..=b'Z')) {
            if !self.rest_starts_with(b"f32")
                && !self.rest_starts_with(b"f64")
                && !self.rest_starts_with(b"i32")
                && !self.rest_starts_with(b"u32")
                && !self.rest_starts_with(b"i64")
                && !self.rest_starts_with(b"u64")
                && !self.rest_starts_with(b"i16")
                && !self.rest_starts_with(b"u16")
                && !self.rest_starts_with(b"i8")
                && !self.rest_starts_with(b"u8")
            {
                while matches!(
                    self.peek(),
                    Some(b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'0'..=b'9')
                ) {
                    self.pos += 1;
                }
                return Ok(self.tok(TokenKind::Quantity, start));
            }
        }

        if is_float {
            if self.rest_starts_with(b"f32") || self.rest_starts_with(b"f64") {
                self.pos += 3;
            }
            Ok(self.tok(TokenKind::Float, start))
        } else {
            self.skip_int_suffix();
            Ok(self.tok(TokenKind::Int, start))
        }
    }

    fn skip_int_suffix(&mut self) {
        for suf in [b"i32".as_slice(), b"u32", b"i64", b"u64"] {
            if self.rest_starts_with(suf) {
                self.pos += suf.len();
                return;
            }
        }
    }

    fn lex_question(&mut self, start: usize) -> Result<Token, Diagnostic> {
        if matches!(self.peek_at(1), Some(b'A'..=b'Z' | b'a'..=b'z' | b'_')) {
            self.pos += 1;
            while matches!(
                self.peek(),
                Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')
            ) {
                self.pos += 1;
            }
            return Ok(self.tok(TokenKind::QueryVar, start));
        }
        self.pos += 1;
        Ok(self.tok(TokenKind::Question, start))
    }

    fn lex_lt(&mut self, start: usize) -> Result<Token, Diagnostic> {
        // <<[  forbidden Quin literal
        if self.peek_at(1) == Some(b'<') && self.peek_at(2) == Some(b'[') {
            self.pos += 3;
            return Ok(self.tok(TokenKind::ForbiddenQuin, start));
        }
        // <<(
        if self.peek_at(1) == Some(b'<') && self.peek_at(2) == Some(b'(') {
            self.pos += 3;
            return Ok(self.tok(TokenKind::TripleStart, start));
        }
        // <<
        if self.peek_at(1) == Some(b'<') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::ReifyStart, start));
        }
        // <->
        if self.peek_at(1) == Some(b'-') && self.peek_at(2) == Some(b'>') {
            self.pos += 3;
            return Ok(self.tok(TokenKind::Bidirectional, start));
        }
        // <=
        if self.peek_at(1) == Some(b'=') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::Le, start));
        }
        // `<` + non-ws is an IRI only if the interior looks like an IRI
        // (`:` `/` `#`). Otherwise it is a type-argument `<` (`Result<T, E>`).
        if let Some(n) = self.peek_at(1) {
            if !n.is_ascii_whitespace() {
                if let Some(gt) = self.bytes[self.pos + 1..].iter().position(|&c| c == b'>') {
                    let inner = &self.bytes[self.pos + 1..self.pos + 1 + gt];
                    let looks_iri =
                        inner.contains(&b':') || inner.contains(&b'/') || inner.contains(&b'#');
                    if looks_iri {
                        self.pos += 2 + gt;
                        return Ok(self.tok(TokenKind::Iri, start));
                    }
                    // Type argument: Result<T, E>
                } else if !self.last_gap_ws {
                    return Err(Diagnostic::new(
                        DiagCode::E001,
                        Span::new(start as u32, (start + 1) as u32),
                        "relational < requires whitespace on both sides (a<b is illegal)",
                    ));
                }
            }
        }
        self.pos += 1;
        Ok(self.tok(TokenKind::Lt, start))
    }

    fn lex_gt(&mut self, start: usize) -> Result<Token, Diagnostic> {
        if self.peek_at(1) == Some(b'>') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::ReifyEnd, start));
        }
        if self.peek_at(1) == Some(b'=') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::Ge, start));
        }
        self.pos += 1;
        Ok(self.tok(TokenKind::Gt, start))
    }

    fn lex_rparen(&mut self, start: usize) -> Result<Token, Diagnostic> {
        if self.peek_at(1) == Some(b'>') && self.peek_at(2) == Some(b'>') {
            self.pos += 3;
            return Ok(self.tok(TokenKind::TripleEnd, start));
        }
        self.pos += 1;
        Ok(self.tok(TokenKind::RParen, start))
    }

    fn lex_minus(&mut self, start: usize) -> Result<Token, Diagnostic> {
        if self.peek_at(1) == Some(b'>') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::ThinArrow, start));
        }
        self.pos += 1;
        Ok(self.tok(TokenKind::Minus, start))
    }

    fn lex_eq(&mut self, start: usize) -> Result<Token, Diagnostic> {
        if self.peek_at(1) == Some(b'>') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::FatArrow, start));
        }
        if self.peek_at(1) == Some(b'=') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::EqEq, start));
        }
        self.pos += 1;
        Ok(self.tok(TokenKind::Eq, start))
    }

    fn lex_bang(&mut self, start: usize) -> Result<Token, Diagnostic> {
        if self.peek_at(1) == Some(b'=') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::Ne, start));
        }
        self.pos += 1;
        Ok(self.tok(TokenKind::Bang, start))
    }

    fn lex_amp(&mut self, start: usize) -> Result<Token, Diagnostic> {
        if self.peek_at(1) == Some(b'&') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::AmpAmp, start));
        }
        Err(Diagnostic::new(
            DiagCode::E001,
            Span::new(start as u32, (start + 1) as u32),
            "expected &&",
        ))
    }

    fn lex_pipe(&mut self, start: usize) -> Result<Token, Diagnostic> {
        if self.peek_at(1) == Some(b'|') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::PipePipe, start));
        }
        if self.peek_at(1) == Some(b'>') {
            self.pos += 2;
            return Ok(self.tok(TokenKind::PipeGt, start));
        }
        self.pos += 1;
        Ok(self.tok(TokenKind::Pipe, start))
    }

    fn lex_blank(&mut self, start: usize) -> Result<Token, Diagnostic> {
        self.pos += 2;
        if !matches!(
            self.peek(),
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')
        ) {
            return Err(Diagnostic::new(
                DiagCode::E001,
                Span::new(start as u32, self.pos as u32),
                "blank node needs a label",
            ));
        }
        while matches!(
            self.peek(),
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'-')
        ) {
            self.pos += 1;
        }
        Ok(self.tok(TokenKind::Ident, start))
    }

    fn lex_ident_or_kw(&mut self, start: usize) -> Result<Token, Diagnostic> {
        self.pos += 1;
        while matches!(
            self.peek(),
            Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')
        ) {
            self.pos += 1;
        }
        // T40: Also consume any trailing Unicode XID_Continue characters
        // so that identifiers like "café" or "変数" are fully consumed.
        self.consume_unicode_ident_tail();
        let text = &self.src[start..self.pos];
        // T40: Validate Unicode identifier policy if the identifier
        // contains any non-ASCII characters.
        if text.bytes().any(|b| b >= 0x80) {
            if let Err(policy_err) = crate::unicode_ident::validate_identifier(text) {
                return Err(Diagnostic::new(
                    DiagCode::E001,
                    Span::new(start as u32, self.pos as u32),
                    format!("invalid Unicode identifier: {policy_err}"),
                ));
            }
        }
        let kind = if self.is_keyword_text(text) {
            TokenKind::Keyword
        } else {
            TokenKind::Ident
        };
        Ok(self.tok(kind, start))
    }

    /// T40: Lex a Unicode identifier starting with a non-ASCII byte.
    /// Validates against the BiDi/NFC/homoglyph policy.
    fn lex_unicode_ident_or_kw(&mut self, start: usize) -> Result<Token, Diagnostic> {
        // Consume the first UTF-8 character
        self.consume_utf8_char();
        // Consume any continuation characters (ASCII or Unicode)
        self.consume_unicode_ident_tail();
        let text = &self.src[start..self.pos];
        // Validate against the Unicode identifier policy
        if let Err(policy_err) = crate::unicode_ident::validate_identifier(text) {
            return Err(Diagnostic::new(
                DiagCode::E001,
                Span::new(start as u32, self.pos as u32),
                format!("invalid Unicode identifier: {policy_err}"),
            ));
        }
        let kind = if self.is_keyword_text(text) {
            TokenKind::Keyword
        } else {
            TokenKind::Ident
        };
        Ok(self.tok(kind, start))
    }

    fn lex_color(&mut self, start: usize) -> Result<Token, Diagnostic> {
        self.pos += 1; // '#'
        let hex_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')) {
            self.pos += 1;
        }
        let n = self.pos - hex_start;
        if n == 3 || n == 4 || n == 6 || n == 8 {
            return Ok(self.tok(TokenKind::Color, start));
        }
        Err(Diagnostic::new(
            DiagCode::E001,
            Span::new(start as u32, self.pos as u32),
            "color literal must be #rgb, #rgba, #rrggbb, or #rrggbbaa",
        ))
    }

    fn is_keyword_text(&self, text: &str) -> bool {
        is_keyword_in(text, &self.extra_locales)
    }

    /// Consume one UTF-8 encoded character from the input.
    fn consume_utf8_char(&mut self) {
        if self.pos >= self.bytes.len() {
            return;
        }
        let b = self.bytes[self.pos];
        let len = if b < 0x80 {
            1
        } else if b < 0xC0 {
            1 // Continuation byte without lead — shouldn't happen, but safe
        } else if b < 0xE0 {
            2
        } else if b < 0xF0 {
            3
        } else {
            4
        };
        self.pos += len.min(self.bytes.len() - self.pos);
    }

    /// Consume the tail of an identifier — ASCII or Unicode XID_Continue
    /// characters. Stops at the first byte that is not part of an
    /// identifier character.
    fn consume_unicode_ident_tail(&mut self) {
        loop {
            // ASCII fast path
            if matches!(
                self.peek(),
                Some(b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_')
            ) {
                self.pos += 1;
                continue;
            }
            // Unicode path: check if the next bytes form a valid UTF-8
            // character that is XID_Continue
            if let Some(b) = self.peek() {
                if crate::unicode_ident::could_be_unicode_ident_start(b) {
                    let char_start = self.pos;
                    self.consume_utf8_char();
                    if let Some(s) = self.src.get(char_start..self.pos) {
                        if let Some(ch) = s.chars().next() {
                            if crate::unicode_ident::is_xid_continue(ch) {
                                continue;
                            }
                        }
                    }
                    // Not XID_Continue — rewind
                    self.pos = char_start;
                }
            }
            break;
        }
    }

    fn bump_simple(&mut self, kind: TokenKind, start: usize) -> Result<Token, Diagnostic> {
        self.pos += 1;
        Ok(self.tok(kind, start))
    }

    fn tok(&self, kind: TokenKind, start: usize) -> Token {
        Token {
            kind,
            span: Span::new(start as u32, self.pos as u32),
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_at(&self, off: usize) -> Option<u8> {
        self.bytes.get(self.pos + off).copied()
    }

    fn rest_starts_with(&self, s: &[u8]) -> bool {
        self.bytes.get(self.pos..).is_some_and(|r| r.starts_with(s))
    }
}

fn strip_bom(src: &str) -> &str {
    src.strip_prefix('\u{feff}').unwrap_or(src)
}

#[allow(dead_code)]
pub fn is_keyword(text: &str) -> bool {
    is_keyword_in(text, &[])
}

fn is_keyword_in(text: &str, extra: &[crate::locale::Locale]) -> bool {
    if crate::locale::ENGLISH_KEYWORDS.iter().any(|k| *k == text) {
        return true;
    }
    if extra.is_empty() {
        return false;
    }
    let reg = crate::locale::LocaleRegistry::default();
    for loc in extra {
        if let Some(table) = reg.table_for(*loc) {
            if table.resolve(text).is_some() {
                return true;
            }
        }
    }
    false
}

#[allow(dead_code)]
pub fn tokenize(src: &str) -> Result<Vec<Token>, Diagnostic> {
    let mut lex = Lexer::new(src);
    let mut out = Vec::new();
    loop {
        let t = lex.next_token()?;
        let eof = t.kind == TokenKind::Eof;
        out.push(t);
        if eof {
            break;
        }
    }
    Ok(out)
}
