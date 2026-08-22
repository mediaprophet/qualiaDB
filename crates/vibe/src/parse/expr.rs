use super::*;

impl<'a> Parser<'a> {
    pub(crate) fn parse_expr(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_pipeline()
    }

    pub(crate) fn parse_pipeline(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_tween()?;
        while self.cur.kind == TokenKind::PipeGt {
            self.bump()?;
            let right = self.parse_tween()?;
            let span = Span::new(left.span.start, right.span.end);
            left = Expr {
                span,
                kind: ExprKind::Pipe {
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    pub(crate) fn parse_tween(&mut self) -> Result<Expr, Diagnostic> {
        let from = self.parse_or()?;
        if self.cur.kind != TokenKind::Tilde {
            return Ok(from);
        }
        self.bump()?;
        let to = self.parse_or()?;
        if self.canonical_text() != "over" {
            return Err(Diagnostic::new(
                DiagCode::E001,
                self.cur.span,
                "expected 'over' after '~' tween (from ~ to over duration ease name)",
            ));
        }
        self.bump()?;
        let over = self.parse_or()?;
        let mut ease = None;
        let mut spring = None;
        if self.canonical_text() == "ease" {
            self.bump()?;
            ease = Some(self.expect_ident()?);
        } else if self.canonical_text() == "spring" {
            self.bump()?;
            self.expect(TokenKind::LParen, "expected '(' after spring")?;
            spring = Some(self.parse_named_only_args()?);
            self.expect(TokenKind::RParen, "expected ')' after spring(...)")?;
        }
        let span = Span::new(from.span.start, self.cur.span.start);
        Ok(Expr {
            span,
            kind: ExprKind::Tween {
                from: Box::new(from),
                to: Box::new(to),
                over: Box::new(over),
                ease,
                spring,
            },
        })
    }

    fn parse_named_only_args(&mut self) -> Result<Vec<NamedArg>, Diagnostic> {
        let mut out = Vec::new();
        if self.cur.kind == TokenKind::RParen {
            return Ok(out);
        }
        loop {
            let name = self.expect_ident()?;
            self.expect(TokenKind::Colon, "expected ':' in named arg")?;
            let value = self.parse_expr()?;
            out.push(NamedArg {
                span: value.span,
                name,
                value,
            });
            if self.cur.kind == TokenKind::Comma {
                self.bump()?;
                continue;
            }
            break;
        }
        Ok(out)
    }

    pub(crate) fn parse_or(&mut self) -> Result<Expr, Diagnostic> {
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
    pub(crate) fn continue_binary(&mut self, mut left: Expr) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_and(&mut self) -> Result<Expr, Diagnostic> {
        let mut left = self.parse_eq()?;
        while self.cur.kind == TokenKind::AmpAmp {
            self.bump()?;
            let right = self.parse_eq()?;
            left = bin(left, BinOp::And, right);
        }
        Ok(left)
    }

    pub(crate) fn parse_eq(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_rel(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_add(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_mul(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_unary(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_postfix(&mut self) -> Result<Expr, Diagnostic> {
        let mut expr = self.parse_primary()?;
        loop {
            match self.cur.kind {
                TokenKind::Dot => {
                    self.bump()?;
                    // Allow keywords as member names (e.g. `material.yield`
                    // where `yield` is a keyword).
                    let name = if self.cur.kind == TokenKind::Ident
                        || self.cur.kind == TokenKind::Keyword
                    {
                        let n = self.text().to_string();
                        self.bump()?;
                        n
                    } else {
                        return Err(self.err("expected identifier after '.'"));
                    };
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

    pub(crate) fn parse_args(&mut self) -> Result<Vec<Arg>, Diagnostic> {
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
                        let local = self.expect_prefixed_local()?;
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

    pub(crate) fn continue_postfix(&mut self, mut expr: Expr) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_primary(&mut self) -> Result<Expr, Diagnostic> {
        if self.cur.kind == TokenKind::Pipe {
            return self.parse_lambda();
        }
        if self.cur.kind == TokenKind::InterpolatedString {
            let start = self.cur.span.start;
            let end = self.cur.span.end;
            let raw = self.text().to_string();
            self.bump()?;
            let content = if raw.starts_with("f\"") && raw.ends_with('"') && raw.len() >= 3 {
                &raw[2..raw.len() - 1]
            } else {
                ""
            };
            let mut parts = Vec::new();
            let mut i = 0;
            let bytes = content.as_bytes();
            while i < bytes.len() {
                if bytes[i] == b'{' {
                    let mut depth = 1;
                    let expr_start = i + 1;
                    i += 1;
                    while i < bytes.len() && depth > 0 {
                        if bytes[i] == b'{' {
                            depth += 1;
                        } else if bytes[i] == b'}' {
                            depth -= 1;
                        }
                        i += 1;
                    }
                    let expr_src = &content[expr_start..i - 1];
                    let mut p = Parser::new(expr_src)?;
                    let sub_expr = p.parse_expr()?;
                    parts.push(sub_expr);
                } else {
                    let lit_start = i;
                    while i < bytes.len() && bytes[i] != b'{' {
                        if bytes[i] == b'\\' && i + 1 < bytes.len() {
                            i += 2;
                            continue;
                        }
                        i += 1;
                    }
                    let s = &content[lit_start..i];
                    parts.push(Expr {
                        span: Span::new(start, end),
                        kind: ExprKind::Literal(Literal::String(s.to_string())),
                    });
                }
            }
            return Ok(Expr {
                span: Span::new(start, end),
                kind: ExprKind::Interpolate(parts),
            });
        }
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
        if self.cur.kind == TokenKind::Ident || self.cur.kind == TokenKind::Keyword {
            let txt = self.canonical_text().to_string();
            if txt == "graph" || txt == "sparql" {
                let rest = self.lex.source()[self.cur.span.end as usize..].trim_start();
                if rest.starts_with('?') || rest.starts_with('{') {
                    let start = self.cur.span.start;
                    self.bump()?;
                    let mut is_ask = false;
                    if self.cur.kind == TokenKind::Question {
                        self.bump()?;
                        is_ask = true;
                    }
                    if self.cur.kind == TokenKind::LBrace {
                        self.bump()?;
                        let pattern_start = self.cur.span.start;
                        let mut depth = 1;
                        while self.cur.kind != TokenKind::Eof && depth > 0 {
                            if self.cur.kind == TokenKind::LBrace {
                                depth += 1;
                            } else if self.cur.kind == TokenKind::RBrace {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                            self.bump()?;
                        }
                        let pattern_end = self.cur.span.start;
                        let end = self.expect(TokenKind::RBrace, "expected '}'")?.end;
                        let pattern = self.lex.source()[pattern_start as usize..pattern_end as usize].trim().to_string();
                        let mut vars = Vec::new();
                        for token in pattern.split_whitespace() {
                            if token.starts_with('?') {
                                let v = token[1..].trim_matches(|c: char| !c.is_alphanumeric() && c != '_').to_string();
                                if !vars.contains(&v) {
                                    vars.push(v);
                                }
                            }
                        }
                        return Ok(Expr {
                            span: Span::new(start, end),
                            kind: ExprKind::GraphQuery { is_ask, pattern, variables: vars },
                        });
                    }
                }
            }

            // Modal logic blocks
            let modal_opt = match txt.as_str() {
                "obligate" => Some(ModalKind::DeonticObligate),
                "permit" => Some(ModalKind::DeonticPermit),
                "forbid" => Some(ModalKind::DeonticForbid),
                "knows" => Some(ModalKind::EpistemicKnows),
                "believes" => Some(ModalKind::EpistemicBelieves),
                "paraconsistent" => Some(ModalKind::Paraconsistent),
                "always" => Some(ModalKind::LtlGlobally),
                "eventually" => Some(ModalKind::LtlFinally),
                "until" => Some(ModalKind::LtlUntil),
                _ => None,
            };

            if let Some(modal) = modal_opt {
                let rest = self.lex.source()[self.cur.span.end as usize..].trim_start();
                if rest.starts_with('(') || rest.starts_with('{') {
                    let start = self.cur.span.start;
                    self.bump()?;
                    if self.cur.kind == TokenKind::LParen {
                        self.bump()?;
                        let mut args = Vec::new();
                        if self.cur.kind != TokenKind::RParen {
                            loop {
                                args.push(self.parse_expr()?);
                                if self.cur.kind == TokenKind::Comma {
                                    self.bump()?;
                                    continue;
                                }
                                break;
                            }
                        }
                        let end = self.expect(TokenKind::RParen, "expected ')'")?.end;
                        return Ok(Expr {
                            span: Span::new(start, end),
                            kind: ExprKind::ModalLogic {
                                modality: modal,
                                args,
                                body: None,
                            },
                        });
                    } else if self.cur.kind == TokenKind::LBrace {
                        self.bump()?;
                        let body = self.parse_expr()?;
                        let end = self.expect(TokenKind::RBrace, "expected '}'")?.end;
                        return Ok(Expr {
                            span: Span::new(start, end),
                            kind: ExprKind::ModalLogic {
                                modality: modal,
                                args: Vec::new(),
                                body: Some(Box::new(body)),
                            },
                        });
                    }
                }
            }
        }

        if self.cur.kind == TokenKind::Ident
            || (self.cur.kind == TokenKind::Keyword && self.text() == "capability")
        {
            let start = self.cur.span.start;
            let name = self.text().to_string();
            let name_end = self.cur.span.end;
            self.bump()?;
            if self.cur.kind == TokenKind::Colon && self.peek_prefixed_local_after_colon() {
                self.bump()?;
                let local = self.expect_prefixed_local()?;
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

    pub(crate) fn peek_prefixed_local_after_colon(&self) -> bool {
        let rest = self.lex.source()[self.cur.span.end as usize..].trim_start();
        rest.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
    }

    pub(crate) fn parse_triple(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_reified(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_term(&mut self) -> Result<Expr, Diagnostic> {
        self.parse_primary()
    }

    pub(crate) fn parse_list(&mut self) -> Result<Expr, Diagnostic> {
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

    pub(crate) fn parse_record(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let mut fields = Vec::new();
        if self.cur.kind != TokenKind::RBrace {
            loop {
                if self.cur.kind == TokenKind::RBrace {
                    break;
                }
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

    pub(crate) fn parse_named_arg(&mut self) -> Result<NamedArg, Diagnostic> {
        let start = self.cur.span.start;
        // Allow keywords as named arg names (e.g. `yield: 50.0` in material
        // declarations â€” `yield` is a keyword but also a valid property name).
        let name = if self.cur.kind == TokenKind::Ident || self.cur.kind == TokenKind::Keyword {
            let n = self.text().to_string();
            self.bump()?;
            n
        } else {
            return Err(self.err("expected identifier in named argument"));
        };
        // Record punning: if next token is NOT ':', shorthand { x } -> { x: x }
        let (value, end) = if self.cur.kind == TokenKind::Colon {
            self.bump()?;
            let val = self.parse_expr()?;
            let e = val.span.end;
            (val, e)
        } else {
            let span = Span::new(start, self.prev_end(start));
            (Expr {
                span,
                kind: ExprKind::Ident(name.clone()),
            }, span.end)
        };
        Ok(NamedArg {
            span: Span::new(start, end),
            name,
            value,
        })
    }

    pub(crate) fn parse_type(&mut self) -> Result<TypeExpr, Diagnostic> {
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
            end = self
                .expect(TokenKind::Gt, "expected '>' after type args")?
                .end;
        }
        Ok(TypeExpr {
            span: Span::new(start, end),
            name,
            args,
        })
    }

    pub(crate) fn parse_lambda(&mut self) -> Result<Expr, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // |
        let mut params = Vec::new();
        if self.cur.kind != TokenKind::Pipe {
            loop {
                params.push(self.expect_ident()?);
                if self.cur.kind == TokenKind::Comma {
                    self.bump()?;
                    continue;
                }
                break;
            }
        }
        self.expect(TokenKind::Pipe, "expected '|' after lambda parameters")?;
        let body = self.parse_expr()?;
        Ok(Expr {
            span: Span::new(start, body.span.end),
            kind: ExprKind::Lambda {
                params,
                body: Box::new(body),
            },
        })
    }

    pub(crate) fn canonical_text(&self) -> &str {
        let t = self.text();
        if let Some((_, canonical)) = crate::locale::LocaleRegistry::default().resolve(t) {
            canonical
        } else {
            t
        }
    }
}
