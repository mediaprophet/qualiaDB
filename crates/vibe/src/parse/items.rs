use super::*;

impl<'a> Parser<'a> {
    pub(crate) fn parse_item(&mut self) -> Result<Item, Diagnostic> {
        if self.kw("bind") {
            return Ok(Item::Bind(self.parse_bind_decl()?));
        }
        if self.looks_like_cell() {
            return Ok(Item::Cell(self.parse_cell_decl()?));
        }
        if self.kw("present") {
            return Ok(Item::Present(self.parse_present()?));
        }
        if self.looks_like_function() {
            return Ok(Item::Function(self.parse_function()?));
        }
        if self.kw("on") {
            return Ok(Item::Hook(self.parse_hook()?));
        }
        if self.kw("const") {
            return Ok(Item::Const(self.parse_const()?));
        }
        if self.kw("enum") {
            return Ok(Item::Enum(self.parse_enum()?));
        }
        if self.kw("field") {
            return Ok(Item::Field(self.parse_field()?));
        }
        if self.kw("material") {
            return Ok(Item::Material(self.parse_material()?));
        }
        if self.kw("law") {
            return Ok(Item::Law(self.parse_law()?));
        }
        Ok(Item::Statement(self.parse_stmt()?))
    }

    pub(crate) fn looks_like_cell(&self) -> bool {
        if self.kw("cell") {
            return true;
        }
        if self.kw("pure") || self.kw("hot") || self.kw("effect") {
            let rest = self.lex.source()[self.cur.span.end as usize..].trim_start();
            return rest.starts_with("cell");
        }
        false
    }

    pub(crate) fn parse_cell_decl(&mut self) -> Result<CellDecl, Diagnostic> {
        let start = self.cur.span.start;
        let effect = self.take_effect_class()?;
        if !self.kw("cell") {
            return Err(self.err("expected 'cell'"));
        }
        self.bump()?;
        let name = self.expect_ident()?;
        let mut params = Vec::new();
        if self.cur.kind == TokenKind::LParen {
            self.bump()?;
            params = self.parse_params()?;
            self.expect(TokenKind::RParen, "expected ')'")?;
        }
        let mut when = None;
        if self.kw("when") {
            self.bump()?;
            when = Some(self.parse_expr()?);
        }
        if self.cur.kind == TokenKind::ColonEq || self.cur.kind == TokenKind::Eq {
            self.bump()?;
        }
        let expr = if self.cur.kind == TokenKind::LBrace {
            self.bump()?;
            let e = self.parse_expr()?;
            self.expect(TokenKind::RBrace, "expected '}'")?;
            e
        } else {
            self.parse_expr()?
        };
        if self.cur.kind == TokenKind::Semicolon {
            self.bump()?;
        }
        let end = self.cur.span.start;
        Ok(CellDecl {
            span: Span::new(start, end),
            effect,
            name,
            params,
            expr,
            when,
        })
    }

    pub(crate) fn parse_bind_decl(&mut self) -> Result<BindDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // bind
        let left = self.parse_expr()?;
        self.expect(TokenKind::Bidirectional, "expected '<->' in bind")?;
        let right = self.parse_expr()?;
        let mut clamp = None;
        if self.kw("using") {
            self.bump()?;
            let _name = self.expect_ident()?;
            self.expect(TokenKind::LBracket, "expected '[' after Clamp")?;
            let lo = self.parse_expr()?;
            self.expect(TokenKind::Comma, "expected ',' in Clamp[lo, hi]")?;
            let hi = self.parse_expr()?;
            self.expect(TokenKind::RBracket, "expected ']' after Clamp range")?;
            clamp = Some((lo, hi));
        }
        let mut resolve = BindResolve::Latest;
        if self.canonical_text() == "resolve" {
            self.bump()?;
            let how = self.expect_ident()?;
            resolve = match how.as_str() {
                "latest" => BindResolve::Latest,
                "left" => BindResolve::Left,
                "right" => BindResolve::Right,
                other => {
                    return Err(Diagnostic::new(
                        DiagCode::E001,
                        self.cur.span,
                        format!("unknown bind resolve `{other}`; use latest, left, or right"),
                    ));
                }
            };
        }
        if self.cur.kind == TokenKind::Semicolon {
            self.bump()?;
        }
        let end = self.cur.span.start;
        Ok(BindDecl {
            span: Span::new(start, end),
            left,
            right,
            clamp,
            resolve,
        })
    }

    pub(crate) fn looks_like_function(&self) -> bool {
        if self.kw("fn") || self.kw("async") || self.kw("pure") || self.kw("hot") || self.kw("cold")
        {
            return true;
        }
        self.kw("effect") && self.peek_fn_after_effect()
    }

    pub(crate) fn peek_fn_after_effect(&self) -> bool {
        // `effect fn` vs `effect expr;` â€” we only have one token of lookahead.
        // Heuristic: next token after bump would be needed. Store nothing;
        // parse_function starts with effect and requires fn/async next, else we
        // treat as statement. So look at remaining source after current token.
        let rest = &self.lex.source()[self.cur.span.end as usize..];
        let trimmed = rest.trim_start();
        trimmed.starts_with("fn") || trimmed.starts_with("async")
    }

    pub(crate) fn parse_function(&mut self) -> Result<FunctionDecl, Diagnostic> {
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

    pub(crate) fn parse_hook(&mut self) -> Result<HookDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // on
        let mut path = vec![self.expect_ident()?];
        while self.cur.kind == TokenKind::Colon || self.cur.kind == TokenKind::Dot {
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

    pub(crate) fn parse_const(&mut self) -> Result<ConstDecl, Diagnostic> {
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

    /// Parse a user-defined enum declaration (T9).
    ///
    /// ```vibe
    /// enum Shape {
    ///   Circle(f64),
    ///   Square(f64),
    ///   Rect(f64, f64),
    ///   Point,
    /// }
    /// ```
    pub(crate) fn parse_enum(&mut self) -> Result<EnumDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // 'enum'
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace, "expected '{' after enum name")?;
        let mut variants = Vec::new();
        while self.cur.kind != TokenKind::RBrace {
            let vstart = self.cur.span.start;
            let vname = self.expect_ident()?;
            let mut payload = Vec::new();
            if self.cur.kind == TokenKind::LParen {
                self.bump()?;
                loop {
                    payload.push(self.parse_type()?);
                    if self.cur.kind == TokenKind::Comma {
                        self.bump()?;
                        continue;
                    }
                    break;
                }
                self.expect(
                    TokenKind::RParen,
                    "expected ')' after variant payload types",
                )?;
            }
            variants.push(EnumVariant {
                span: Span::new(vstart, self.cur.span.start),
                name: vname,
                payload,
            });
            if self.cur.kind == TokenKind::Comma {
                self.bump()?;
                continue;
            }
            break;
        }
        let end = self
            .expect(TokenKind::RBrace, "expected '}' after enum variants")?
            .end;
        Ok(EnumDecl {
            span: Span::new(start, end),
            name,
            variants,
        })
    }

    /// Parse a field declaration (T28).
    ///
    /// ```vibe
    /// field pressure_ambient: Pressure
    ///   unit: <qudt:KiloPascal>
    ///   support: region
    ///   representation: grid;
    /// ```
    pub(crate) fn parse_field(&mut self) -> Result<FieldDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // 'field'
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon, "expected ':' after field name")?;
        let ty = self.parse_type()?;
        let mut unit = None;
        let mut support = FieldSupport::Region;
        let mut representation = FieldRepresentation::Grid;
        // Parse optional named properties until ';'
        loop {
            if self.cur.kind == TokenKind::Semicolon {
                self.bump()?;
                break;
            }
            let key = self.expect_ident()?;
            self.expect(TokenKind::Colon, "expected ':' after field property name")?;
            match key.as_str() {
                "unit" => {
                    // unit: <iri> or unit: "string"
                    if self.cur.kind == TokenKind::Iri {
                        let s = self.text().to_string();
                        self.bump()?;
                        unit = Some(strip_iri(&s));
                    } else if self.cur.kind == TokenKind::String {
                        unit = Some(self.expect_string()?);
                    } else {
                        // Could be a prefixed name like qudt:KiloPascal
                        let s = self.expect_ident()?;
                        unit = Some(s);
                    }
                }
                "support" => {
                    let s = self.expect_ident()?;
                    support = match s.as_str() {
                        "region" => FieldSupport::Region,
                        "point" => FieldSupport::Point,
                        "continuant" => FieldSupport::Continuant,
                        "stream" => FieldSupport::Stream,
                        _ => return Err(self.err(&format!("unknown field support '{s}'"))),
                    };
                }
                "representation" => {
                    let s = self.expect_ident()?;
                    representation = match s.as_str() {
                        "grid" => FieldRepresentation::Grid,
                        "mesh" => FieldRepresentation::Mesh,
                        "particles" => FieldRepresentation::Particles,
                        "analytic" => FieldRepresentation::Analytic,
                        "sampled" => FieldRepresentation::Sampled,
                        _ => return Err(self.err(&format!("unknown field representation '{s}'"))),
                    };
                }
                _ => {
                    // Skip unknown property â€” consume one token value
                    self.bump()?;
                }
            }
        }
        Ok(FieldDecl {
            span: Span::new(start, self.cur.span.start),
            name,
            ty,
            unit,
            support,
            representation,
        })
    }

    /// Parse a material declaration (T29).
    ///
    /// ```vibe
    /// material sucrose_cube: Material
    ///   yield: 50.0 <qudt:KiloPascal>
    ///   density: 1580.0;
    /// ```
    pub(crate) fn parse_material(&mut self) -> Result<MaterialDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // 'material'
        let name = self.expect_ident()?;
        self.expect(TokenKind::Colon, "expected ':' after material name")?;
        // The type name (e.g. "Material") â€” we consume it but don't store it
        // since all materials are Material type for now.
        let _ty_name = self.expect_ident()?;
        // Parse properties as named args until ';'
        let mut properties = Vec::new();
        while self.cur.kind != TokenKind::Semicolon {
            let prop = self.parse_named_arg()?;
            properties.push(prop);
            if self.cur.kind == TokenKind::Comma {
                self.bump()?;
                continue;
            }
            break;
        }
        self.expect(
            TokenKind::Semicolon,
            "expected ';' after material declaration",
        )?;
        Ok(MaterialDecl {
            span: Span::new(start, self.cur.span.start),
            name,
            properties,
        })
    }

    /// Parse a law declaration (T30).
    ///
    /// ```vibe
    /// law crush
    ///   when sample(pressure_ambient, pose(self)) > self.material.yield
    ///   => transform.yield(self);
    /// ```
    pub(crate) fn parse_law(&mut self) -> Result<LawDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // 'law'
        let name = self.expect_ident()?;
        if !self.kw("when") {
            return Err(self.err("expected 'when' after law name"));
        }
        self.bump()?; // 'when'
                      // Parse the condition expression up to '=>'
        let condition = self.parse_expr()?;
        self.expect(TokenKind::FatArrow, "expected '=>' after law condition")?;
        let consequence = self.parse_expr()?;
        self.eat_optional_semi()?;
        Ok(LawDecl {
            span: Span::new(start, self.cur.span.start),
            name,
            condition,
            consequence,
        })
    }

    pub(crate) fn parse_budget(&mut self) -> Result<Vec<NamedArg>, Diagnostic> {
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

    pub(crate) fn parse_params(&mut self) -> Result<Vec<Param>, Diagnostic> {
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
}
