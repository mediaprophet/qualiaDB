//! Recursive-descent parser for vibe-0.1.

use crate::ast::*;
use crate::error::{DiagCode, Diagnostic};
use crate::lex::{Lexer, Token, TokenKind};
use crate::span::Span;

pub struct Parser<'a> {
    lex: Lexer<'a>,
    cur: Token,
}

impl<'a> Parser<'a> {
    pub fn new(src: &'a str) -> Result<Self, Diagnostic> {
        let mut lex = Lexer::new(src);
        let cur = lex.next_token()?;
        Ok(Self { lex, cur })
    }

    pub fn source(&self) -> &'a str {
        self.lex.source()
    }

    pub fn parse_program(&mut self) -> Result<Program, Diagnostic> {
        let start = self.cur.span.start;
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
        if self.kw("requires") {
            requires = self.parse_requires()?;
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

    fn parse_module(&mut self) -> Result<ModuleDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // module
        let name = if self.cur.kind == TokenKind::Iri {
            let s = self.text().to_string();
            self.bump()?;
            Name::Iri(strip_iri(&s))
        } else {
            let s = self.expect_ident()?;
            Name::Ident(s)
        };
        let end = self.expect(TokenKind::Semicolon, "expected ';' after module")?.end;
        Ok(ModuleDecl {
            span: Span::new(start, end),
            name,
        })
    }

    fn parse_import(&mut self) -> Result<ImportDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let path = self.expect_string()?;
        let alias = if self.kw("as") {
            self.bump()?;
            Some(self.expect_ident()?)
        } else {
            None
        };
        let end = self.expect(TokenKind::Semicolon, "expected ';' after import")?.end;
        Ok(ImportDecl {
            span: Span::new(start, end),
            path,
            alias,
        })
    }

    fn parse_prefix(&mut self) -> Result<PrefixDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let prefix = self.expect_ident()?;
        self.expect(TokenKind::Colon, "expected ':' after prefix name")?;
        if self.cur.kind != TokenKind::Iri {
            return Err(self.err("expected IRI after prefix"));
        }
        let iri = strip_iri(self.text());
        self.bump()?;
        let end = self.expect(TokenKind::Semicolon, "expected ';' after prefix")?.end;
        Ok(PrefixDecl {
            span: Span::new(start, end),
            prefix,
            iri,
        })
    }

    fn parse_requires(&mut self) -> Result<Vec<CapSpec>, Diagnostic> {
        self.bump()?;
        self.expect(TokenKind::LBracket, "expected '[' after requires")?;
        let mut caps = Vec::new();
        if self.cur.kind != TokenKind::RBracket {
            loop {
                caps.push(self.parse_cap_spec()?);
                if self.cur.kind == TokenKind::Comma {
                    self.bump()?;
                    continue;
                }
                break;
            }
        }
        self.expect(TokenKind::RBracket, "expected ']' after requires")?;
        self.expect(TokenKind::Semicolon, "expected ';' after requires")?;
        Ok(caps)
    }

    fn parse_cap_spec(&mut self) -> Result<CapSpec, Diagnostic> {
        let start = self.cur.span.start;
        if !self.kw("capability") {
            return Err(self.err("expected capability(...)"));
        }
        self.bump()?;
        self.expect(TokenKind::LParen, "expected '('")?;
        let id = self.expect_string()?;
        let mut args = Vec::new();
        while self.cur.kind == TokenKind::Comma {
            self.bump()?;
            args.push(self.parse_named_arg()?);
        }
        let end = self.expect(TokenKind::RParen, "expected ')'")?.end;
        Ok(CapSpec {
            span: Span::new(start, end),
            id,
            args,
        })
    }

    fn parse_item(&mut self) -> Result<Item, Diagnostic> {
        if self.looks_like_function() {
            return Ok(Item::Function(self.parse_function()?));
        }
        if self.kw("on") {
            return Ok(Item::Hook(self.parse_hook()?));
        }
        if self.kw("const") {
            return Ok(Item::Const(self.parse_const()?));
        }
        Ok(Item::Statement(self.parse_stmt()?))
    }

    fn looks_like_function(&self) -> bool {
        if self.kw("fn") || self.kw("async") || self.kw("pure") || self.kw("hot") || self.kw("cold")
        {
            return true;
        }
        self.kw("effect") && self.peek_fn_after_effect()
    }

    fn peek_fn_after_effect(&self) -> bool {
        // `effect fn` vs `effect expr;` — we only have one token of lookahead.
        // Heuristic: next token after bump would be needed. Store nothing;
        // parse_function starts with effect and requires fn/async next, else we
        // treat as statement. So look at remaining source after current token.
        let rest = &self.lex.source()[self.cur.span.end as usize..];
        let trimmed = rest.trim_start();
        trimmed.starts_with("fn") || trimmed.starts_with("async")
    }

    fn parse_function(&mut self) -> Result<FunctionDecl, Diagnostic> {
        let start = self.cur.span.start;
        let effect = self.take_effect_class()?;
        let is_async = if self.kw("async") {
            self.bump()?;
            true
        } else {
            false
        };
        if !self.kw("fn") {
            return Err(self.err("expected fn"));
        }
        self.bump()?;
        let name = self.expect_ident()?;
        self.expect(TokenKind::LParen, "expected '('")?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen, "expected ')'")?;
        let budget = self.parse_budget()?;
        let ret = if self.cur.kind == TokenKind::ThinArrow {
            self.bump()?;
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(FunctionDecl {
            span: Span::new(start, body.span.end),
            effect,
            is_async,
            name,
            params,
            budget,
            ret,
            body,
        })
    }

    fn parse_hook(&mut self) -> Result<HookDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // on
        let mut path = vec![self.expect_ident()?];
        while self.cur.kind == TokenKind::Colon {
            self.bump()?;
            path.push(self.expect_ident()?);
        }
        self.expect(TokenKind::LParen, "expected '(' after event path")?;
        let params = self.parse_params()?;
        self.expect(TokenKind::RParen, "expected ')'")?;
        let budget = self.parse_budget()?;
        let ret = if self.cur.kind == TokenKind::ThinArrow {
            self.bump()?;
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = self.parse_block()?;
        Ok(HookDecl {
            span: Span::new(start, body.span.end),
            path,
            params,
            budget,
            ret,
            body,
        })
    }

    fn parse_const(&mut self) -> Result<ConstDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let name = self.expect_ident()?;
        let ty = if self.cur.kind == TokenKind::Colon {
            self.bump()?;
            Some(self.parse_type()?)
        } else {
            None
        };
        self.expect(TokenKind::Eq, "expected '=' in const")?;
        let value = self.parse_expr()?;
        let end = self.expect(TokenKind::Semicolon, "expected ';'")?.end;
        Ok(ConstDecl {
            span: Span::new(start, end),
            name,
            ty,
            value,
        })
    }

    fn parse_budget(&mut self) -> Result<Vec<NamedArg>, Diagnostic> {
        if !self.kw("budget") && !(self.cur.kind == TokenKind::Ident && self.text() == "budget") {
            return Ok(Vec::new());
        }
        self.bump()?;
        self.expect(TokenKind::LParen, "expected '(' after budget")?;
        let mut args = Vec::new();
        if self.cur.kind != TokenKind::RParen {
            loop {
                args.push(self.parse_named_arg()?);
                if self.cur.kind == TokenKind::Comma {
                    self.bump()?;
                    continue;
                }
                break;
            }
        }
        self.expect(TokenKind::RParen, "expected ')' after budget")?;
        Ok(args)
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, Diagnostic> {
        let mut params = Vec::new();
        if self.cur.kind == TokenKind::RParen {
            return Ok(params);
        }
        loop {
            let start = self.cur.span.start;
            let name = self.expect_ident()?;
            self.expect(TokenKind::Colon, "expected ':' after parameter name")?;
            let ty = self.parse_type()?;
            params.push(Param {
                span: Span::new(start, ty.span.end),
                name,
                ty,
            });
            if self.cur.kind == TokenKind::Comma {
                self.bump()?;
                continue;
            }
            break;
        }
        Ok(params)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
        if self.kw("let") {
            return self.parse_let();
        }
        if self.kw("if") {
            let stmt = self.parse_if()?;
            self.eat_optional_semi()?;
            return Ok(stmt);
        }
        if self.kw("for") {
            let stmt = self.parse_for()?;
            self.eat_optional_semi()?;
            return Ok(stmt);
        }
        if self.kw("while") {
            let stmt = self.parse_while()?;
            self.eat_optional_semi()?;
            return Ok(stmt);
        }
        if self.kw("match") {
            let stmt = self.parse_match()?;
            self.eat_optional_semi()?;
            return Ok(stmt);
        }
        if self.kw("return") {
            return self.parse_return();
        }
        if self.kw("yield") {
            return self.parse_yield();
        }
        if self.kw("transaction") {
            let stmt = self.parse_transaction()?;
            self.eat_optional_semi()?;
            return Ok(stmt);
        }
        if self.kw("effect") && !self.peek_fn_after_effect() {
            let start = self.cur.span.start;
            self.bump()?;
            let expr = self.parse_expr()?;
            let end = self.expect(TokenKind::Semicolon, "expected ';'")?.end;
            return Ok(Stmt::Effect {
                span: Span::new(start, end),
                expr,
            });
        }
        let expr = self.parse_expr()?;
        if self.cur.kind == TokenKind::Eq {
            self.bump()?;
            let value = self.parse_expr()?;
            let end = self.expect(TokenKind::Semicolon, "expected ';'")?.end;
            return Ok(Stmt::Assign {
                span: Span::new(expr.span.start, end),
                target: expr,
                value,
            });
        }
        let end = self.expect(TokenKind::Semicolon, "expected ';' after expression")?.end;
        Ok(Stmt::Expr {
            span: Span::new(expr.span.start, end),
            expr,
        })
    }

    fn parse_let(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let mutable = if self.kw("mut") {
            self.bump()?;
            true
        } else {
            false
        };
        let name = self.expect_ident()?;
        let ty = if self.cur.kind == TokenKind::Colon {
            self.bump()?;
            Some(self.parse_type()?)
        } else {
            None
        };
        let value = if self.cur.kind == TokenKind::Eq {
            self.bump()?;
            Some(self.parse_expr()?)
        } else {
            None
        };
        let end = self.expect(TokenKind::Semicolon, "expected ';'")?.end;
        Ok(Stmt::Let {
            span: Span::new(start, end),
            mutable,
            name,
            ty,
            value,
        })
    }

    fn parse_if(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let cond = self.parse_expr()?;
        let then_block = self.parse_block()?;
        let else_block = if self.kw("else") {
            self.bump()?;
            if self.kw("if") {
                Some(Box::new(self.parse_if()?))
            } else {
                let b = self.parse_block()?;
                Some(Box::new(Stmt::Block(b)))
            }
        } else {
            None
        };
        let end = else_block
            .as_ref()
            .map(|e| e.span().end)
            .unwrap_or(then_block.span.end);
        Ok(Stmt::If {
            span: Span::new(start, end),
            cond,
            then_block,
            else_block,
        })
    }

    fn parse_for(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let name = self.expect_ident()?;
        if !self.kw("in") {
            return Err(self.err("expected 'in'"));
        }
        self.bump()?;
        let iter = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::For {
            span: Span::new(start, body.span.end),
            name,
            iter,
            body,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        Ok(Stmt::While {
            span: Span::new(start, body.span.end),
            cond,
            body,
        })
    }

    fn parse_match(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let scrutinee = self.parse_expr()?;
        self.expect(TokenKind::LBrace, "expected '{'")?;
        let mut arms = Vec::new();
        while self.cur.kind != TokenKind::RBrace && self.cur.kind != TokenKind::Eof {
            arms.push(self.parse_match_arm()?);
        }
        let end = self.expect(TokenKind::RBrace, "expected '}'")?.end;
        Ok(Stmt::Match {
            span: Span::new(start, end),
            scrutinee,
            arms,
        })
    }

    fn parse_match_arm(&mut self) -> Result<MatchArm, Diagnostic> {
        let start = self.cur.span.start;
        let pattern = self.parse_pattern()?;
        self.expect(TokenKind::FatArrow, "expected '=>'")?;
        if self.cur.kind == TokenKind::LBrace {
            let body = self.parse_block()?;
            Ok(MatchArm {
                span: Span::new(start, body.span.end),
                pattern,
                body: ArmBody::Block(body),
            })
        } else {
            let expr = self.parse_expr()?;
            if self.cur.kind == TokenKind::Comma {
                self.bump()?;
            }
            Ok(MatchArm {
                span: Span::new(start, expr.span.end),
                pattern,
                body: ArmBody::Expr(expr),
            })
        }
    }

    fn parse_pattern(&mut self) -> Result<Pattern, Diagnostic> {
        if self.cur.kind == TokenKind::Ident && self.text() == "_" {
            self.bump()?;
            return Ok(Pattern::Wildcard);
        }
        if self.kw("None") || (self.cur.kind == TokenKind::Ident && self.text() == "None") {
            self.bump()?;
            return Ok(Pattern::None);
        }
        if matches!(self.text(), "Ok" | "Err" | "Some") && self.cur.kind == TokenKind::Ident {
            let tag = self.text().to_string();
            self.bump()?;
            self.expect(TokenKind::LParen, "expected '('")?;
            let inner = self.parse_pattern()?;
            self.expect(TokenKind::RParen, "expected ')'")?;
            return Ok(match tag.as_str() {
                "Ok" => Pattern::Ok(Box::new(inner)),
                "Err" => Pattern::Err(Box::new(inner)),
                _ => Pattern::Some(Box::new(inner)),
            });
        }
        if matches!(
            self.cur.kind,
            TokenKind::String | TokenKind::Int | TokenKind::Float | TokenKind::Keyword
        ) && matches!(self.text(), "true" | "false" | "null")
            || matches!(self.cur.kind, TokenKind::String | TokenKind::Int | TokenKind::Float)
        {
            if let Ok(lit) = self.take_literal() {
                return Ok(Pattern::Literal(lit));
            }
        }
        let name = self.expect_ident()?;
        Ok(Pattern::Ident(name))
    }

    fn parse_return(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let value = if self.cur.kind == TokenKind::Semicolon {
            None
        } else {
            Some(self.parse_expr()?)
        };
        let end = self.expect(TokenKind::Semicolon, "expected ';'")?.end;
        Ok(Stmt::Return {
            span: Span::new(start, end),
            value,
        })
    }

    fn parse_yield(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let value = if self.cur.kind == TokenKind::Semicolon {
            None
        } else {
            Some(self.parse_expr()?)
        };
        let end = self.expect(TokenKind::Semicolon, "expected ';'")?.end;
        Ok(Stmt::Yield {
            span: Span::new(start, end),
            value,
        })
    }

    fn parse_transaction(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let mut args = Vec::new();
        if self.cur.kind == TokenKind::LParen {
            self.bump()?;
            if self.cur.kind != TokenKind::RParen {
                loop {
                    args.push(self.parse_named_arg()?);
                    if self.cur.kind == TokenKind::Comma {
                        self.bump()?;
                        continue;
                    }
                    break;
                }
            }
            self.expect(TokenKind::RParen, "expected ')'")?;
        }
        let body = self.parse_block()?;
        Ok(Stmt::Transaction {
            span: Span::new(start, body.span.end),
            args,
            body,
        })
    }

    fn parse_block(&mut self) -> Result<Block, Diagnostic> {
        let start = self.expect(TokenKind::LBrace, "expected '{'")?.start;
        let mut stmts = Vec::new();
        while self.cur.kind != TokenKind::RBrace && self.cur.kind != TokenKind::Eof {
            stmts.push(self.parse_stmt()?);
        }
        let end = self.expect(TokenKind::RBrace, "expected '}'")?.end;
        Ok(Block {
            span: Span::new(start, end),
            stmts,
        })
    }

    fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_and()?;
        while self.cur.kind == TokenKind::PipePipe {
            self.bump()?;
            let right = self.parse_and()?;
            left = bin(left, BinOp::Or, right);
        }
        Ok(left)
    }

    /// Continue parsing binary operators from the mul level upward, given an
    /// already-parsed left operand (e.g. an identifier with postfix operators).
    /// This lets `parse_args` handle expressions like `v2 / g` as call arguments
    /// where the leading identifier was already consumed for named-arg detection.
    fn continue_binary(&mut self, mut left: Expr) -> Result<Expr, Diagnostic> {
        // mul level
        loop {
            let op = match self.cur.kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Rem,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_unary()?;
            left = bin(left, op, right);
        }
        // add level
        loop {
            let op = match self.cur.kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_mul()?;
            left = bin(left, op, right);
        }
        // rel level
        loop {
            let op = match self.cur.kind {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_add()?;
            left = bin(left, op, right);
        }
        // eq level
        loop {
            let op = match self.cur.kind {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::Ne => BinOp::Ne,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_rel()?;
            left = bin(left, op, right);
        }
        // and level
        while self.cur.kind == TokenKind::AmpAmp {
            self.bump()?;
            let right = self.parse_eq()?;
            left = bin(left, BinOp::And, right);
        }
        // or level
        while self.cur.kind == TokenKind::PipePipe {
            self.bump()?;
            let right = self.parse_and()?;
            left = bin(left, BinOp::Or, right);
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_eq()?;
        while self.cur.kind == TokenKind::AmpAmp {
            self.bump()?;
            let right = self.parse_eq()?;
            left = bin(left, BinOp::And, right);
        }
        Ok(left)
    }

    fn parse_eq(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_rel()?;
        loop {
            let op = match self.cur.kind {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::Ne => BinOp::Ne,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_rel()?;
            left = bin(left, op, right);
        }
        Ok(left)
    }

    fn parse_rel(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_add()?;
        loop {
            let op = match self.cur.kind {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_add()?;
            left = bin(left, op, right);
        }
        Ok(left)
    }

    fn parse_add(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_mul()?;
        loop {
            let op = match self.cur.kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_mul()?;
            left = bin(left, op, right);
        }
        Ok(left)
    }

    fn parse_mul(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_unary()?;
        loop {
            let op = match self.cur.kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Rem,
                _ => break,
            };
            self.bump()?;
            let right = self.parse_unary()?;
            left = bin(left, op, right);
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
        if self.kw("await") {
            let start = self.cur.span.start;
            self.bump()?;
            let expr = self.parse_unary()?;
            return Ok(Expr {
                span: Span::new(start, expr.span.end),
                kind: ExprKind::Await(Box::new(expr)),
            });
        }
        let op = match self.cur.kind {
            TokenKind::Bang => Some(UnOp::Not),
            TokenKind::Minus => Some(UnOp::Neg),
            TokenKind::Plus => Some(UnOp::Plus),
            _ => None,
        };
        if let Some(op) = op {
            let start = self.cur.span.start;
            self.bump()?;
            let expr = self.parse_unary()?;
            return Ok(Expr {
                span: Span::new(start, expr.span.end),
                kind: ExprKind::Unary {
                    op,
                    expr: Box::new(expr),
                },
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.cur.kind {
                TokenKind::Dot => {
                    self.bump()?;
                    let name = self.expect_ident()?;
                    let end = self.prev_end(expr.span.end);
                    expr = Expr {
                        span: Span::new(expr.span.start, end),
                        kind: ExprKind::Member {
                            recv: Box::new(expr),
                            name,
                        },
                    };
                }
                TokenKind::LParen => {
                    self.bump()?;
                    let args = self.parse_args()?;
                    let end = self.expect(TokenKind::RParen, "expected ')'")?.end;
                    expr = Expr {
                        span: Span::new(expr.span.start, end),
                        kind: ExprKind::Call {
                            callee: Box::new(expr),
                            args,
                        },
                    };
                }
                TokenKind::LBracket => {
                    self.bump()?;
                    let index = self.parse_expr()?;
                    let end = self.expect(TokenKind::RBracket, "expected ']'")?.end;
                    expr = Expr {
                        span: Span::new(expr.span.start, end),
                        kind: ExprKind::Index {
                            recv: Box::new(expr),
                            index: Box::new(index),
                        },
                    };
                }
                TokenKind::Question => {
                    let end = self.cur.span.end;
                    self.bump()?;
                    expr = Expr {
                        span: Span::new(expr.span.start, end),
                        kind: ExprKind::Try(Box::new(expr)),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Arg>, Diagnostic> {
        let mut args = Vec::new();
        if self.cur.kind == TokenKind::RParen {
            return Ok(args);
        }
        loop {
            if self.cur.kind == TokenKind::Ident {
                let save_span = self.cur.span;
                let name = self.text().to_string();
                self.bump()?;
                if self.cur.kind == TokenKind::Colon {
                    let start = save_span.start;
                    self.bump()?;
                    if is_named_arg_key(&name) {
                        let value = self.parse_expr()?;
                        args.push(Arg::Named(NamedArg {
                            span: Span::new(start, value.span.end),
                            name,
                            value,
                        }));
                    } else {
                        let local = self.expect_ident()?;
                        args.push(Arg::Pos(Expr {
                            span: Span::new(start, self.prev_end(save_span.end)),
                            kind: ExprKind::Prefixed(name, local),
                        }));
                    }
                } else {
                    let mut expr = Expr {
                        span: save_span,
                        kind: ExprKind::Ident(name),
                    };
                    expr = self.continue_postfix(expr)?;
                    expr = self.continue_binary(expr)?;
                    args.push(Arg::Pos(expr));
                }
            } else {
                args.push(Arg::Pos(self.parse_expr()?));
            }
            if self.cur.kind == TokenKind::Comma {
                self.bump()?;
                continue;
            }
            break;
        }
        Ok(args)
    }

    fn continue_postfix(&mut self, mut expr: Expr) -> Result<Expr, Diagnostic> {
        loop {
            match self.cur.kind {
                TokenKind::Dot => {
                    self.bump()?;
                    let name = self.expect_ident()?;
                    expr = Expr {
                        span: Span::new(expr.span.start, self.prev_end(expr.span.end)),
                        kind: ExprKind::Member {
                            recv: Box::new(expr),
                            name,
                        },
                    };
                }
                TokenKind::LParen => {
                    self.bump()?;
                    let args = self.parse_args()?;
                    let end = self.expect(TokenKind::RParen, "expected ')'")?.end;
                    expr = Expr {
                        span: Span::new(expr.span.start, end),
                        kind: ExprKind::Call {
                            callee: Box::new(expr),
                            args,
                        },
                    };
                }
                TokenKind::LBracket => {
                    self.bump()?;
                    let index = self.parse_expr()?;
                    let end = self.expect(TokenKind::RBracket, "expected ']'")?.end;
                    expr = Expr {
                        span: Span::new(expr.span.start, end),
                        kind: ExprKind::Index {
                            recv: Box::new(expr),
                            index: Box::new(index),
                        },
                    };
                }
                TokenKind::Question => {
                    let end = self.cur.span.end;
                    self.bump()?;
                    expr = Expr {
                        span: Span::new(expr.span.start, end),
                        kind: ExprKind::Try(Box::new(expr)),
                    };
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        if self.cur.kind == TokenKind::ForbiddenQuin {
            return Err(Diagnostic::new(
                DiagCode::E001,
                self.cur.span,
                "raw Quin literal <<[ s p o g prov ]>> is forbidden; use quin.statement(...)",
            ));
        }
        if self.cur.kind == TokenKind::TripleStart {
            return self.parse_triple();
        }
        if self.cur.kind == TokenKind::ReifyStart {
            return self.parse_reified();
        }
        if self.cur.kind == TokenKind::LParen {
            self.bump()?;
            let e = self.parse_expr()?;
            self.expect(TokenKind::RParen, "expected ')'")?;
            return Ok(e);
        }
        if self.cur.kind == TokenKind::LBracket {
            return self.parse_list();
        }
        if self.cur.kind == TokenKind::LBrace {
            return self.parse_record();
        }
        if self.cur.kind == TokenKind::Iri {
            let span = self.cur.span;
            let iri = strip_iri(self.text());
            self.bump()?;
            return Ok(Expr {
                span,
                kind: ExprKind::Iri(iri),
            });
        }
        if self.cur.kind == TokenKind::QueryVar {
            let span = self.cur.span;
            let name = self.text()[1..].to_string();
            self.bump()?;
            return Ok(Expr {
                span,
                kind: ExprKind::QueryVar(name),
            });
        }
        if self.cur.kind == TokenKind::Ident && self.text().starts_with("_:") {
            let span = self.cur.span;
            let name = self.text()[2..].to_string();
            self.bump()?;
            return Ok(Expr {
                span,
                kind: ExprKind::Blank(name),
            });
        }
        if self.cur.kind == TokenKind::Ident
            || (self.cur.kind == TokenKind::Keyword && self.text() == "capability")
        {
            let start = self.cur.span.start;
            let name = self.text().to_string();
            let name_end = self.cur.span.end;
            self.bump()?;
            if self.cur.kind == TokenKind::Colon && matches!(self.peek_ident_after_colon(), true) {
                self.bump()?;
                let local = self.expect_ident()?;
                return Ok(Expr {
                    span: Span::new(start, self.prev_end(name_end)),
                    kind: ExprKind::Prefixed(name, local),
                });
            }
            return Ok(Expr {
                span: Span::new(start, name_end),
                kind: ExprKind::Ident(name),
            });
        }
        if let Ok(lit) = self.take_literal() {
            let span = self.prev_span_of_lit();
            return Ok(Expr {
                span,
                kind: ExprKind::Literal(lit),
            });
        }
        Err(self.err("expected expression"))
    }

    fn peek_ident_after_colon(&self) -> bool {
        let rest = self.lex.source()[self.cur.span.end as usize..].trim_start();
        rest.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    }

    fn parse_triple(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let subject = self.parse_term()?;
        let predicate = self.parse_term()?;
        let object = self.parse_term()?;
        let end = self
            .expect(TokenKind::TripleEnd, "expected )>> after triple term")?
            .end;
        Ok(Expr {
            span: Span::new(start, end),
            kind: ExprKind::Triple {
                subject: Box::new(subject),
                predicate: Box::new(predicate),
                object: Box::new(object),
            },
        })
    }

    fn parse_reified(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let subject = self.parse_term()?;
        let predicate = self.parse_term()?;
        let object = self.parse_term()?;
        self.expect(TokenKind::Tilde, "expected '~' before reifier (RDF 1.2)")?;
        let reifier = self.parse_term()?;
        let end = self.expect(TokenKind::ReifyEnd, "expected >>")?.end;
        Ok(Expr {
            span: Span::new(start, end),
            kind: ExprKind::Reified {
                subject: Box::new(subject),
                predicate: Box::new(predicate),
                object: Box::new(object),
                reifier: Box::new(reifier),
            },
        })
    }

    fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_primary()
    }

    fn parse_list(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let mut els = Vec::new();
        if self.cur.kind != TokenKind::RBracket {
            loop {
                els.push(self.parse_expr()?);
                if self.cur.kind == TokenKind::Comma {
                    self.bump()?;
                    continue;
                }
                break;
            }
        }
        let end = self.expect(TokenKind::RBracket, "expected ']'")?.end;
        Ok(Expr {
            span: Span::new(start, end),
            kind: ExprKind::List(els),
        })
    }

    fn parse_record(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let mut fields = Vec::new();
        if self.cur.kind != TokenKind::RBrace {
            loop {
                fields.push(self.parse_named_arg()?);
                if self.cur.kind == TokenKind::Comma {
                    self.bump()?;
                    continue;
                }
                break;
            }
        }
        let end = self.expect(TokenKind::RBrace, "expected '}'")?.end;
        Ok(Expr {
            span: Span::new(start, end),
            kind: ExprKind::Record(fields),
        })
    }

    fn parse_named_arg(&mut self) -> Result<NamedArg, Diagnostic> {
        let start = self.cur.span.start;
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon, "expected ':' in named argument")?;
        let value = self.parse_expr()?;
        Ok(NamedArg {
            span: Span::new(start, value.span.end),
            name,
            value,
        })
    }

    fn parse_type(&mut self) -> Result<TypeExpr, Diagnostic> {
        let start = self.cur.span.start;
        let name = if self.cur.kind == TokenKind::Ident || self.cur.kind == TokenKind::Keyword {
            let n = self.text().to_string();
            self.bump()?;
            n
        } else {
            return Err(self.err("expected type name"));
        };
        let mut args = Vec::new();
        let mut end = self.prev_end(start);
        if self.cur.kind == TokenKind::Lt {
            self.bump()?;
            loop {
                args.push(self.parse_type()?);
                if self.cur.kind == TokenKind::Comma {
                    self.bump()?;
                    continue;
                }
                break;
            }
            end = self.expect(TokenKind::Gt, "expected '>' after type args")?.end;
        }
        Ok(TypeExpr {
            span: Span::new(start, end),
            name,
            args,
        })
    }

    fn take_effect_class(&mut self) -> Result<Option<EffectClass>, Diagnostic> {
        let class = match self.text() {
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
            let s = unquote(self.text())?;
            self.bump()?;
            return Ok(Literal::String(s));
        }
        if self.cur.kind == TokenKind::Int {
            let n = parse_int(self.text())?;
            self.bump()?;
            return Ok(Literal::Int(n));
        }
        if self.cur.kind == TokenKind::Float {
            let bits = parse_float_bits(self.text())?;
            self.bump()?;
            return Ok(Literal::Float(bits));
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

    fn expect_string(&mut self) -> Result<String, Diagnostic> {
        if self.cur.kind != TokenKind::String {
            return Err(self.err("expected string"));
        }
        let s = unquote(self.text())?;
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
        self.cur.kind == TokenKind::Keyword && self.text() == name
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

fn is_named_arg_key(s: &str) -> bool {
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
    )
}

fn is_stmt_kw(s: &str) -> bool {
    matches!(
        s,
        "let" | "if" | "else" | "for" | "while" | "match" | "return" | "yield" | "transaction" | "fn"
            | "on" | "const" | "module" | "import" | "prefix" | "requires"
    )
}

fn bin(left: Expr, op: BinOp, right: Expr) -> Expr {
    Expr {
        span: left.span.merge(right.span),
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

fn item_span(item: &Item) -> Span {
    match item {
        Item::Function(f) => f.span,
        Item::Hook(h) => h.span,
        Item::Const(c) => c.span,
        Item::Statement(s) => s.span(),
    }
}

fn strip_iri(s: &str) -> String {
    s.trim_start_matches('<').trim_end_matches('>').to_string()
}

fn unquote(s: &str) -> Result<String, Diagnostic> {
    let inner = s
        .strip_prefix('"')
        .and_then(|x| x.strip_suffix('"'))
        .ok_or_else(|| Diagnostic::new(DiagCode::E001, Span::point(0), "bad string"))?;
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
                        return Err(Diagnostic::new(
                            DiagCode::E001,
                            Span::point(0),
                            "expected \\u{XXXX}",
                        ));
                    }
                    let mut hex = String::new();
                    loop {
                        match chars.next() {
                            Some('}') => break,
                            Some(h) => hex.push(h),
                            None => {
                                return Err(Diagnostic::new(
                                    DiagCode::E001,
                                    Span::point(0),
                                    "unterminated \\u{}",
                                ))
                            }
                        }
                    }
                    let cp = u32::from_str_radix(&hex, 16).map_err(|_| {
                        Diagnostic::new(DiagCode::E001, Span::point(0), "bad unicode escape")
                    })?;
                    out.push(char::from_u32(cp).ok_or_else(|| {
                        Diagnostic::new(DiagCode::E001, Span::point(0), "invalid codepoint")
                    })?);
                }
                _ => {
                    return Err(Diagnostic::new(
                        DiagCode::E001,
                        Span::point(0),
                        "bad string escape",
                    ))
                }
            }
        } else {
            out.push(c);
        }
    }
    Ok(out)
}

fn parse_int(s: &str) -> Result<i64, Diagnostic> {
    let raw = s
        .trim_end_matches("i32")
        .trim_end_matches("u32")
        .trim_end_matches("i64")
        .trim_end_matches("u64")
        .replace('_', "");
    if let Some(hex) = raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16)
            .or_else(|_| u64::from_str_radix(hex, 16).map(|u| u as i64))
            .map_err(|_| Diagnostic::new(DiagCode::E001, Span::point(0), "bad hex integer"))
    } else {
        raw.parse::<i64>()
            .map_err(|_| Diagnostic::new(DiagCode::E001, Span::point(0), "bad integer"))
    }
}

fn parse_float_bits(s: &str) -> Result<u64, Diagnostic> {
    let raw = s
        .trim_end_matches("f32")
        .trim_end_matches("f64")
        .replace('_', "");
    raw.parse::<f64>()
        .map(f64::to_bits)
        .map_err(|_| Diagnostic::new(DiagCode::E001, Span::point(0), "bad float"))
}

pub fn parse_program(src: &str) -> Result<Program, Diagnostic> {
    Parser::new(src)?.parse_program()
}

pub fn parse_cell(src: &str) -> Result<Expr, Diagnostic> {
    Parser::new(src)?.parse_cell_body()
}
