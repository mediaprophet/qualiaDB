//! Value-producing conditional expressions: `if cond { a } else { b }`.
//!
//! Grammar (expression position only; statement-form `if` lives in `stmt.rs`):
//!
//! ```text
//! if_expr   := "if" expr branch "else" ( if_expr | branch )
//! branch    := "{" expr "}"
//! ```
//!
//! Semantics fixed by the parser:
//! - each branch carries exactly one result expression, with no trailing `;`;
//! - the final `else` is mandatory (no implicit `null`);
//! - the conditional is an atom closed by its final `}`. Binary operators that
//!   follow apply to the whole conditional (`if a {1} else {2} + 3` is
//!   `(if a {1} else {2}) + 3`). Postfix member access, calls, indexing and `?`
//!   on an *unparenthesised* conditional are rejected; write `(if ...).field`.
//! - nesting is bounded by [`MAX_CONDITIONAL_DEPTH`].

use super::*;

/// Maximum nesting depth of conditional expressions (including `else if`
/// chain links) accepted by the parser. Bounds parser recursion.
pub const MAX_CONDITIONAL_DEPTH: u16 = 64;

impl<'a> Parser<'a> {
    /// Parse a conditional expression. The current token must be `if`.
    pub(crate) fn parse_if_expr(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.cur.span.start;
        if self.cond_depth >= MAX_CONDITIONAL_DEPTH {
            return Err(Diagnostic::new(
                DiagCode::E001,
                self.cur.span,
                format!("conditional expression nesting exceeds limit of {MAX_CONDITIONAL_DEPTH}"),
            ));
        }
        self.cond_depth += 1;
        let result = self.parse_if_expr_inner(start);
        self.cond_depth -= 1;
        result
    }

    fn parse_if_expr_inner(&mut self, start: u32) -> Result<Expr, Diagnostic> {
        let if_span = self.cur.span;
        self.bump()?; // `if`
        if self.cur.kind == TokenKind::LBrace {
            return Err(self.err("expected condition after 'if'"));
        }
        let cond = self.parse_expr()?;
        let (then_value, _) = self.parse_cond_branch("then")?;
        if !self.kw("else") {
            return Err(Diagnostic::new(
                DiagCode::E001,
                Span::new(if_span.start, self.cur.span.end.max(if_span.end)),
                "conditional expression requires a final 'else' branch",
            ));
        }
        self.bump()?; // `else`
        let (else_value, end) = if self.kw("if") {
            let nested = self.parse_if_expr()?;
            let end = nested.span.end;
            (nested, end)
        } else {
            self.parse_cond_branch("else")?
        };
        Ok(Expr {
            span: Span::new(start, end),
            kind: ExprKind::IfElse {
                cond: Box::new(cond),
                then_value: Box::new(then_value),
                else_value: Box::new(else_value),
            },
        })
    }

    /// Parse `{ expr }`, returning the inner expression and the end offset of
    /// the closing brace.
    fn parse_cond_branch(&mut self, which: &str) -> Result<(Expr, u32), Diagnostic> {
        if self.cur.kind != TokenKind::LBrace {
            return Err(self.err(&format!(
                "expected '{{' to open the {which} branch of a conditional expression"
            )));
        }
        let open = self.cur.span;
        self.bump()?;
        if self.cur.kind == TokenKind::RBrace {
            return Err(Diagnostic::new(
                DiagCode::E001,
                Span::new(open.start, self.cur.span.end),
                format!("empty {which} branch in conditional expression; expected a value"),
            ));
        }
        let value = self.parse_expr()?;
        if self.cur.kind == TokenKind::Semicolon {
            return Err(self.err(
                "conditional expression branch holds a single value; remove the trailing ';'",
            ));
        }
        if self.cur.kind != TokenKind::RBrace {
            return Err(self.err(&format!(
                "expected '}}' to close the {which} branch of a conditional expression"
            )));
        }
        let end = self.cur.span.end;
        self.bump()?;
        Ok((value, end))
    }
}
