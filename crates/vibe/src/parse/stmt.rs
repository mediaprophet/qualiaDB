use super::*;

impl<'a> Parser<'a> {
    pub(crate) fn parse_stmt(&mut self) -> Result<Stmt, Diagnostic> {
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
        if self.cur.kind == TokenKind::LBrace {
            let b = self.parse_block()?;
            self.eat_optional_semi()?;
            return Ok(Stmt::Block(b));
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
        let end = self
            .expect(TokenKind::Semicolon, "expected ';' after expression")?
            .end;
        Ok(Stmt::Expr {
            span: Span::new(expr.span.start, end),
            expr,
        })
    }

    pub(crate) fn parse_let(&mut self) -> Result<Stmt, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let mutable = if self.kw("mut") {
            self.bump()?;
            true
        } else {
            false
        };
        let is_destructure = self.cur.kind == TokenKind::LBrace
            || self.cur.kind == TokenKind::LBracket
            || (self.cur.kind == TokenKind::Ident && {
                let name = self.text();
                let rest = &self.lex.source()[self.cur.span.end as usize..];
                let trimmed = rest.trim_start();
                (trimmed.starts_with('(')
                    && matches!(
                        name,
                        "vec2" | "vec3" | "vec4" | "mat3" | "mat4" | "Ok" | "Err" | "Some"
                    ))
                    || trimmed.starts_with('.')
            });

        if is_destructure {
            let pattern = self.parse_pattern()?;
            let ty = if self.cur.kind == TokenKind::Colon {
                self.bump()?;
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(TokenKind::Eq, "expected '=' in destructuring let")?;
            let value = self.parse_expr()?;
            let end = self.expect(TokenKind::Semicolon, "expected ';'")?.end;
            return Ok(Stmt::LetPat {
                span: Span::new(start, end),
                mutable,
                pattern,
                ty,
                value,
            });
        }
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

    pub(crate) fn parse_if(&mut self) -> Result<Stmt, Diagnostic> {
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

    pub(crate) fn parse_for(&mut self) -> Result<Stmt, Diagnostic> {
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

    pub(crate) fn parse_while(&mut self) -> Result<Stmt, Diagnostic> {
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

    pub(crate) fn parse_match(&mut self) -> Result<Stmt, Diagnostic> {
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

    pub(crate) fn parse_match_arm(&mut self) -> Result<MatchArm, Diagnostic> {
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

    pub(crate) fn parse_pattern(&mut self) -> Result<Pattern, Diagnostic> {
        if self.cur.kind == TokenKind::LBrace {
            self.bump()?;
            let mut fields = Vec::new();
            if self.cur.kind != TokenKind::RBrace {
                loop {
                    let field_name = self.expect_ident()?;
                    let pat = if self.cur.kind == TokenKind::Colon {
                        self.bump()?;
                        self.parse_pattern()?
                    } else {
                        Pattern::Ident(field_name.clone())
                    };
                    fields.push((field_name, pat));
                    if self.cur.kind == TokenKind::Comma {
                        self.bump()?;
                        if self.cur.kind == TokenKind::RBrace {
                            break;
                        }
                        continue;
                    }
                    break;
                }
            }
            self.expect(TokenKind::RBrace, "expected '}'")?;
            return Ok(Pattern::Record(fields));
        }
        if self.cur.kind == TokenKind::LBracket {
            self.bump()?;
            let mut elements = Vec::new();
            if self.cur.kind != TokenKind::RBracket {
                loop {
                    elements.push(self.parse_pattern()?);
                    if self.cur.kind == TokenKind::Comma {
                        self.bump()?;
                        if self.cur.kind == TokenKind::RBracket {
                            break;
                        }
                        continue;
                    }
                    break;
                }
            }
            self.expect(TokenKind::RBracket, "expected ']'")?;
            return Ok(Pattern::List(elements));
        }
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
        ) && matches!(self.canonical_text(), "true" | "false" | "null")
            || matches!(
                self.cur.kind,
                TokenKind::String | TokenKind::Int | TokenKind::Float
            )
        {
            if let Ok(lit) = self.take_literal() {
                return Ok(Pattern::Literal(lit));
            }
        }
        // T9: User-defined enum variant pattern: `EnumName.Variant(args)`
        // or `EnumName.Variant` (unit variant), or vector constructor `vec3(x, y, z)`
        if self.cur.kind == TokenKind::Ident {
            let name = self.text().to_string();
            let rest = &self.lex.source()[self.cur.span.end as usize..];
            let trimmed = rest.trim_start();
            if trimmed.starts_with('.') {
                self.bump()?; // enum name
                self.expect(TokenKind::Dot, "expected '.'")?;
                let variant_name = self.expect_ident()?;
                let mut args = Vec::new();
                if self.cur.kind == TokenKind::LParen {
                    self.bump()?;
                    if self.cur.kind != TokenKind::RParen {
                        loop {
                            args.push(self.parse_pattern()?);
                            if self.cur.kind == TokenKind::Comma {
                                self.bump()?;
                                continue;
                            }
                            break;
                        }
                    }
                    self.expect(TokenKind::RParen, "expected ')' after variant pattern args")?;
                }
                return Ok(Pattern::Variant {
                    enum_name: name,
                    variant_name,
                    args,
                });
            }
            if trimmed.starts_with('(')
                && matches!(name.as_str(), "vec2" | "vec3" | "vec4" | "mat3" | "mat4")
            {
                self.bump()?;
                self.expect(TokenKind::LParen, "expected '('")?;
                let mut args = Vec::new();
                if self.cur.kind != TokenKind::RParen {
                    loop {
                        args.push(self.parse_pattern()?);
                        if self.cur.kind == TokenKind::Comma {
                            self.bump()?;
                            continue;
                        }
                        break;
                    }
                }
                self.expect(TokenKind::RParen, "expected ')'")?;
                return Ok(Pattern::Constructor { name, args });
            }
        }
        let name = self.expect_ident()?;
        Ok(Pattern::Ident(name))
    }

    pub(crate) fn parse_return(&mut self) -> Result<Stmt, Diagnostic> {
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

    pub(crate) fn parse_yield(&mut self) -> Result<Stmt, Diagnostic> {
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

    pub(crate) fn parse_transaction(&mut self) -> Result<Stmt, Diagnostic> {
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

    pub(crate) fn parse_block(&mut self) -> Result<Block, Diagnostic> {
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
}
