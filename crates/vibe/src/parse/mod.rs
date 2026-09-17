//! Recursive-descent parser for vibe-0.1.

use crate::ast::*;
use crate::error::{DiagCode, Diagnostic};
use crate::lex::{Lexer, Token, TokenKind};
use crate::span::Span;

mod expr;
mod items;
mod preamble;
mod stmt;

pub struct Parser<'a> {
    pub(crate) lex: Lexer<'a>,
    pub(crate) cur: Token,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Result<Self, Diagnostic> {
        let mut lex = Lexer::new(src);
        let cur = lex.next_token()?;
        Ok(Self { lex, cur })
    }

    #[allow(dead_code)]
    pub fn source(&self) -> &'a str {
        self.lex.source()
    }

    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let start = self.cur.span.start;
        let mut locales = Vec::new();
        while self.kw("locale") {
            locales.push(self.parse_locale()?);
        }
        let module = if self.kw("module") {
            Some(self.parse_module()?)
        } else {
            None
        };
        let mut imports = Vec::new();
        while self.kw("import") {
            imports.push(self.parse_import()?);
        }
        let mut prefixes = Vec::new();
        while self.kw("prefix") {
            prefixes.push(self.parse_prefix()?);
        }
        let mut requires = Vec::new();
        loop {
            if self.kw("using") {
                requires.extend(self.parse_using()?);
                continue;
            }
            if self.kw("requires") {
                requires.extend(self.parse_requires()?);
                continue;
            }
            break;
        }
        let mut items = Vec::new();
        while self.cur.kind != TokenKind::Eof {
            items.push(self.parse_item()?);
        }
        let end = items
            .last()
            .map(|i| item_span(i).end)
            .unwrap_or(self.cur.span.end);
        Ok(Program {
            span: Span::new(start, end),
            module,
            imports,
            prefixes,
            locales,
            requires,
            items,
        })
    }

    pub fn parse_cell_body(&mut self) -> Result<Expr, Diagnostic> {
        self.expect(TokenKind::Eq, "cell body must start with '='")?;
        let expr = self.parse_expr()?;
        if self.cur.kind != TokenKind::Eof && self.cur.kind != TokenKind::Semicolon {
            return Err(self.err("unexpected tokens after cell expression"));
        }
        Ok(expr)
    }
    fn take_effect_class(&mut self) -> Result<Option<EffectClass>, Diagnostic> {
        let class = match self.canonical_text() {
            "pure" => Some(EffectClass::Pure),
            "hot" => Some(EffectClass::Hot),
            "cold" => Some(EffectClass::Cold),
            "async" => return Ok(None),
            "effect" => Some(EffectClass::External),
            _ => None,
        };
        if class.is_some() && self.cur.kind == TokenKind::Keyword {
            self.bump()?;
        }
        Ok(class)
    }

    fn take_literal(&mut self) -> Result<Literal, Diagnostic> {
        if self.kw("true") {
            self.bump()?;
            return Ok(Literal::Bool(true));
        }
        if self.kw("false") {
            self.bump()?;
            return Ok(Literal::Bool(false));
        }
        if self.kw("null") {
            self.bump()?;
            return Ok(Literal::Null);
        }
        if self.cur.kind == TokenKind::String {
            let span = self.cur.span;
            let s = unquote(self.text(), span)?;
            self.bump()?;
            return Ok(Literal::String(s));
        }
        if self.cur.kind == TokenKind::Int {
            let span = self.cur.span;
            let lit = parse_int(self.text(), span)?;
            self.bump()?;
            return Ok(lit);
        }
        if self.cur.kind == TokenKind::Float {
            let span = self.cur.span;
            let bits = parse_float_bits(self.text(), span)?;
            self.bump()?;
            return Ok(Literal::Float(bits));
        }
        if self.cur.kind == TokenKind::Quantity {
            let span = self.cur.span;
            let raw = self.text();
            let (bits, unit) = parse_quantity_literal(raw, span)?;
            self.bump()?;
            return Ok(Literal::Quantity { value: bits, unit });
        }
        if self.cur.kind == TokenKind::Color {
            let span = self.cur.span;
            let raw = self.text();
            let color = parse_color_literal(raw, span)?;
            self.bump()?;
            return Ok(Literal::Color(color));
        }
        Err(self.err("expected literal"))
    }

    fn prev_span_of_lit(&self) -> Span {
        // last bumped token is not stored; use cur as fallback
        Span::point(self.cur.span.start)
    }

    fn prev_end(&self, fallback: u32) -> u32 {
        if self.cur.span.start > 0 {
            self.cur.span.start
        } else {
            fallback
        }
    }

    fn expect(&mut self, kind: TokenKind, msg: &str) -> Result<Span, Diagnostic> {
        if self.cur.kind == kind {
            let span = self.cur.span;
            self.bump()?;
            Ok(span)
        } else {
            Err(self.err(msg))
        }
    }

    fn expect_ident(&mut self) -> Result<String, Diagnostic> {
        if self.cur.kind == TokenKind::Ident
            || (self.cur.kind == TokenKind::Keyword && !is_stmt_kw(self.text()))
        {
            let s = self.text().to_string();
            self.bump()?;
            return Ok(s);
        }
        Err(self.err("expected identifier"))
    }

    /// Prefixed local names include SNOMED-style numeric concept ids (`snomed:386661006`).
    fn expect_prefixed_local(&mut self) -> Result<String, Diagnostic> {
        if self.cur.kind == TokenKind::Int
            || self.cur.kind == TokenKind::Ident
            || (self.cur.kind == TokenKind::Keyword && !is_stmt_kw(self.text()))
        {
            let s = self.text().to_string();
            self.bump()?;
            return Ok(s);
        }
        Err(self.err("expected prefixed local name"))
    }

    fn expect_string(&mut self) -> Result<String, Diagnostic> {
        if self.cur.kind != TokenKind::String {
            return Err(self.err("expected string"));
        }
        let s = unquote(self.text(), self.cur.span)?;
        self.bump()?;
        Ok(s)
    }

    fn eat_optional_semi(&mut self) -> Result<(), Diagnostic> {
        if self.cur.kind == TokenKind::Semicolon {
            self.bump()?;
        }
        Ok(())
    }

    fn kw(&self, name: &str) -> bool {
        self.cur.kind == TokenKind::Keyword && self.canonical_text() == name
    }

    fn text(&self) -> &str {
        self.cur.span.slice(self.lex.source())
    }

    fn bump(&mut self) -> Result<(), Diagnostic> {
        self.cur = self.lex.next_token()?;
        Ok(())
    }

    fn err(&self, msg: &str) -> Diagnostic {
        Diagnostic::new(DiagCode::E001, self.cur.span, msg.to_string())
    }
}

pub(super) fn is_named_arg_key(s: &str) -> bool {
    matches!(
        s,
        "take"
            | "subject"
            | "predicate"
            | "object"
            | "context"
            | "topic"
            | "schema"
            | "scope"
            | "steps"
            | "workspace"
            | "output"
            | "tolerance"
    )
}

pub(super) fn is_stmt_kw(s: &str) -> bool {
    let canon = if let Some((_, c)) = crate::locale::LocaleRegistry::default().resolve(s) {
        c
    } else {
        s
    };
    matches!(
        canon,
        "let"
            | "if"
            | "else"
            | "for"
            | "while"
            | "match"
            | "return"
            | "yield"
            | "transaction"
            | "fn"
            | "on"
            | "const"
            | "module"
            | "import"
            | "prefix"
            | "requires"
    )
}

pub(super) fn bin(left: Expr, op: BinOp, right: Expr) -> Expr {
    Expr {
        span: left.span.merge(right.span),
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

pub(super) fn item_span(item: &Item) -> Span {
    match item {
        Item::Function(f) => f.span,
        Item::Hook(h) => h.span,
        Item::Const(c) => c.span,
        Item::Enum(e) => e.span,
        Item::Field(f) => f.span,
        Item::Material(m) => m.span,
        Item::Law(l) => l.span,
        Item::Cell(c) => c.span,
        Item::Present(p) => p.span,
        Item::Bind(b) => b.span,
        Item::Statement(s) => s.span(),
    }
}

pub(super) fn parse_color_literal(
    raw: &str,
    span: Span,
) -> Result<crate::ast::ColorLit, Diagnostic> {
    let hex = raw.strip_prefix('#').unwrap_or(raw);
    let digit = |i: usize| {
        u8::from_str_radix(&hex[i..i + 1], 16)
            .map_err(|_| Diagnostic::new(DiagCode::E001, span, "invalid hex color"))
    };
    let pair = |i: usize| {
        u8::from_str_radix(&hex[i..i + 2], 16)
            .map_err(|_| Diagnostic::new(DiagCode::E001, span, "invalid hex color"))
    };
    match hex.len() {
        3 => Ok(crate::ast::ColorLit {
            r: digit(0)? * 17,
            g: digit(1)? * 17,
            b: digit(2)? * 17,
            a: 255,
        }),
        4 => Ok(crate::ast::ColorLit {
            r: digit(0)? * 17,
            g: digit(1)? * 17,
            b: digit(2)? * 17,
            a: digit(3)? * 17,
        }),
        6 => Ok(crate::ast::ColorLit {
            r: pair(0)?,
            g: pair(2)?,
            b: pair(4)?,
            a: 255,
        }),
        8 => Ok(crate::ast::ColorLit {
            r: pair(0)?,
            g: pair(2)?,
            b: pair(4)?,
            a: pair(6)?,
        }),
        _ => Err(Diagnostic::new(
            DiagCode::E001,
            span,
            "color literal must be #rgb, #rgba, #rrggbb, or #rrggbbaa",
        )),
    }
}

pub(super) fn strip_iri(s: &str) -> String {
    s.trim_start_matches('<').trim_end_matches('>').to_string()
}

pub(super) fn unquote(s: &str, span: Span) -> Result<String, Diagnostic> {
    let inner = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .ok_or_else(|| Diagnostic::new(DiagCode::E001, span, "bad string"))?;
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('u') => {
                    // \u{XXXX}
                    if chars.next() != Some('{') {
                        return Err(Diagnostic::new(DiagCode::E001, span, "expected \\u{XXXX}"));
                    }
                    let mut hex = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some(h) => hex.push(h),
                            None => {
                                return Err(Diagnostic::new(
                                    DiagCode::E001,
                                    span,
                                    "unterminated \\u{}",
                                ))
                            }
                        }
                    }
                    let cp = u32::from_str_radix(&hex, 16)
                        .map_err(|_| Diagnostic::new(DiagCode::E001, span, "bad unicode escape"))?;
                    out.push(char::from_u32(cp).ok_or_else(|| {
                        Diagnostic::new(DiagCode::E001, span, "invalid codepoint")
                    })?);
                }
                _ => return Err(Diagnostic::new(DiagCode::E001, span, "bad string escape")),
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

pub(super) fn parse_int(s: &str, span: Span) -> Result<Literal, Diagnostic> {
    let raw = s
        .trim_end_matches("i32")
        .trim_end_matches("u32")
        .trim_end_matches("i64")
        .trim_end_matches("u64")
        .replace('_', "");
    let is_unsigned = s.ends_with("u64") || s.ends_with("u32");
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        if is_unsigned {
            u64::from_str_radix(hex, 16)
                .map(Literal::UInt)
                .map_err(|_| Diagnostic::new(DiagCode::E001, span, "bad hex integer"))
        } else if let Ok(i) = i64::from_str_radix(hex, 16) {
            Ok(Literal::Int(i))
        } else if let Ok(u) = u64::from_str_radix(hex, 16) {
            Ok(Literal::UInt(u))
        } else {
            Err(Diagnostic::new(DiagCode::E001, span, "bad hex integer"))
        }
    } else if is_unsigned {
        raw.parse::<u64>()
            .map(Literal::UInt)
            .map_err(|_| Diagnostic::new(DiagCode::E001, span, "bad integer"))
    } else if let Ok(i) = raw.parse::<i64>() {
        Ok(Literal::Int(i))
    } else if let Ok(u) = raw.parse::<u64>() {
        Ok(Literal::UInt(u))
    } else {
        Err(Diagnostic::new(DiagCode::E001, span, "bad integer"))
    }
}

pub(super) fn parse_float_bits(s: &str, span: Span) -> Result<u64, Diagnostic> {
    let raw = s
        .trim_end_matches("f32")
        .trim_end_matches("f64")
        .replace('_', "");
    raw.parse::<f64>()
        .map(f64::to_bits)
        .map_err(|_| Diagnostic::new(DiagCode::E001, span, "bad float"))
}

pub(super) fn parse_quantity_literal(raw: &str, span: Span) -> Result<(u64, String), Diagnostic> {
    let mut num_end = 0;
    let chars: Vec<char> = raw.chars().collect();
    while num_end < chars.len() {
        let c = chars[num_end];
        if c.is_ascii_digit()
            || c == '.'
            || c == '_'
            || c == 'e'
            || c == 'E'
            || (c == '-' && num_end > 0 && (chars[num_end - 1] == 'e' || chars[num_end - 1] == 'E'))
            || (c == '+' && num_end > 0 && (chars[num_end - 1] == 'e' || chars[num_end - 1] == 'E'))
        {
            num_end += 1;
        } else {
            break;
        }
    }
    let num_str: String = chars[..num_end].iter().filter(|&&c| c != '_').collect();
    let mut unit_str: String = chars[num_end..].iter().collect();
    if unit_str.starts_with('[') && unit_str.ends_with(']') {
        unit_str = unit_str[1..unit_str.len() - 1].to_string();
    }
    let val: f64 = num_str.parse().map_err(|_| {
        Diagnostic::new(
            DiagCode::E001,
            span,
            format!("invalid quantity number `{num_str}`"),
        )
    })?;
    Ok((val.to_bits(), unit_str))
}

pub fn parse_program(src: &str) -> Result<Program, Diagnostic> {
    Parser::new(src)?.parse_program()
}

pub fn parse_cell(src: &str) -> Result<Expr, Diagnostic> {
    Parser::new(src)?.parse_cell_body()
}
