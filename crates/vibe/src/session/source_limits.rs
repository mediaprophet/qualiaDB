//! Canonical-lexer ingress guard for the additive source-session profile.
//!
//! Run before recursive parsing/checking. These conservative syntax limits are
//! independent of geometry/resource budgets. Wide flat data is allowed; deeply
//! nested or long expression paths require smaller source submissions. This is
//! not a replacement parser or a claim of process/allocator isolation.
use crate::lex::{Lexer, TokenKind};
use crate::locale::{Locale, LocaleRegistry};
use crate::{DiagCode, Diagnostic, Span};

const MAX_GROUPS: usize = 16;
const MAX_PATH_UNITS: usize = 64;

#[derive(Clone, Copy)]
struct Frame {
    opener: TokenKind,
    units: usize,
    maximum: usize,
    function_body: bool,
}
impl Frame {
    const fn new(opener: TokenKind, function_body: bool) -> Self {
        Self {
            opener,
            units: 0,
            maximum: 0,
            function_body,
        }
    }
    fn charge(&mut self, units: usize, span: Span) -> Result<(), Diagnostic> {
        self.units = self.units.saturating_add(units);
        self.maximum = self.maximum.max(self.units);
        if self.units > MAX_PATH_UNITS {
            return Err(limit(
                span,
                "source-session expression path exceeds qualified syntax limit",
            ));
        }
        Ok(())
    }
}

fn limit(span: Span, message: &str) -> Diagnostic {
    Diagnostic::new(DiagCode::E400, span, message)
}

fn locale(code: &str) -> Option<Locale> {
    match code {
        "en" => Some(Locale::EN),
        "zh" => Some(Locale::ZH),
        "es" => Some(Locale::ES),
        "ja" => Some(Locale::JA),
        "ar" => Some(Locale::AR),
        "hi" => Some(Locale::HI),
        "fr" => Some(Locale::FR),
        "de" => Some(Locale::DE),
        _ => None,
    }
}

pub(super) fn check(source: &str) -> Result<(), Diagnostic> {
    let mut lexer = Lexer::new(source);
    let keywords = LocaleRegistry::with_all_locales();
    let mut frames = [Frame::new(TokenKind::Eof, false); MAX_GROUPS + 1];
    let mut depth = 0;
    let mut previous = TokenKind::Eof;
    let mut function_header = false;
    let mut awaiting_locale = false;
    let mut pending_locale = None;
    loop {
        let token = lexer.next_token()?;
        let word = &lexer.source()[token.span.start as usize..token.span.end as usize];
        let canonical = if token.kind == TokenKind::Keyword {
            keywords.resolve(word).map(|(_, keyword)| keyword)
        } else {
            None
        };
        if token.kind == TokenKind::Eof {
            return Ok(());
        }
        if token.kind == TokenKind::InterpolatedString {
            // Embedded parser calls need their own shared nesting contract;
            // ordinary strings remain available in this bounded profile.
            return Err(limit(
                token.span,
                "interpolated source is not admitted by this bounded source-session profile",
            ));
        }
        if awaiting_locale {
            pending_locale = locale(word);
            awaiting_locale = false;
        } else if depth == 0 && canonical == Some("locale") && previous != TokenKind::Dot {
            awaiting_locale = true;
        }
        if depth == 0 && canonical == Some("fn") && previous != TokenKind::Dot {
            function_header = true;
        }
        match token.kind {
            TokenKind::LParen
            | TokenKind::LBracket
            | TokenKind::LBrace
            | TokenKind::TripleStart
            | TokenKind::ReifyStart => {
                if depth == MAX_GROUPS {
                    return Err(limit(
                        token.span,
                        "source-session delimiter nesting exceeds qualified syntax limit",
                    ));
                }
                let function_body =
                    depth == 0 && token.kind == TokenKind::LBrace && function_header;
                if function_body {
                    function_header = false;
                }
                depth += 1;
                frames[depth] = Frame::new(token.kind, function_body);
            }
            TokenKind::RParen
            | TokenKind::RBracket
            | TokenKind::RBrace
            | TokenKind::TripleEnd
            | TokenKind::ReifyEnd => {
                let expected = match token.kind {
                    TokenKind::RParen => TokenKind::LParen,
                    TokenKind::RBracket => TokenKind::LBracket,
                    TokenKind::RBrace => TokenKind::LBrace,
                    TokenKind::TripleEnd => TokenKind::TripleStart,
                    _ => TokenKind::ReifyStart,
                };
                if depth == 0 || frames[depth].opener != expected {
                    return Err(Diagnostic::new(
                        DiagCode::E001,
                        token.span,
                        "unbalanced source-session delimiter",
                    ));
                }
                let complete = frames[depth];
                depth -= 1;
                if complete.function_body {
                    frames[depth].units = 0;
                } else {
                    frames[depth].charge(complete.maximum.saturating_add(1), token.span)?;
                }
            }
            TokenKind::Semicolon | TokenKind::Comma => {
                frames[depth].units = 0;
                if depth == 0 && token.kind == TokenKind::Semicolon {
                    if let Some(loc) = pending_locale.take() {
                        lexer.enable_locale(loc);
                    }
                    function_header = false;
                }
            }
            _ => frames[depth].charge(1, token.span)?,
        }
        previous = token.kind;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_deep_delimiters_before_parser_entry() {
        for (open, close) in [("[", "]"), ("(", ")"), ("{x:", "}")] {
            let source = format!("= {}0{}", open.repeat(1000), close.repeat(1000));
            assert_eq!(check(&source).unwrap_err().code, DiagCode::E400);
        }
    }
    #[test]
    fn rejects_unary_left_deep_postfix_and_generic_paths() {
        for source in [
            format!("= {}true", "!".repeat(1000)),
            format!("= 1{}", "+1".repeat(1000)),
            format!("= x{}", ".field".repeat(1000)),
            format!("= x{}", "[0]".repeat(1000)),
            format!("let x: {}i64{};", "List<".repeat(1000), ">".repeat(1000)),
        ] {
            assert_eq!(check(&source).unwrap_err().code, DiagCode::E400);
        }
    }
    #[test]
    fn separators_in_children_do_not_reset_parent_expression_path() {
        let source = format!("= 0{}", "+ math.max(1,2)".repeat(1000));
        assert_eq!(check(&source).unwrap_err().code, DiagCode::E400);
    }
    #[test]
    fn accepts_wide_flat_data_and_separate_statements() {
        check(&format!("= [{}0]", "1,".repeat(1000))).unwrap();
        check(&format!("{}0;", "let x=1;".repeat(1000))).unwrap();
        check(&format!(
            "{}0;",
            "pure fn f(x:i64)->i64{return x+1;}".repeat(100)
        ))
        .unwrap();
    }
    #[test]
    fn literals_comments_and_lexer_errors_keep_canonical_meaning() {
        check("= \"[[[[ !!! fn {\"; /* [[[ */").unwrap();
        assert_eq!(check("= 1; @").unwrap_err().code, DiagCode::E001);
        assert_eq!(check("= f\"hello {1}\"").unwrap_err().code, DiagCode::E400);
    }
    #[test]
    fn keyword_locale_applies_only_when_declared() {
        check("locale zh; 纯 函数 f(x:i64)->i64{返回 x+1;}").unwrap();
        let source = format!("= 0{}", "+ 函数 + {x:1}".repeat(1000));
        assert_eq!(check(&source).unwrap_err().code, DiagCode::E400);
    }
}
