use super::*;

impl<'a> Parser<'a> {
    pub(crate) fn parse_module(&mut self) -> Result<ModuleDecl, Diagnostic> {
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
        let end = self
            .expect(TokenKind::Semicolon, "expected ';' after module")?
            .end;
        Ok(ModuleDecl {
            span: Span::new(start, end),
            name,
        })
    }

    pub(crate) fn parse_import(&mut self) -> Result<ImportDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let path = if self.cur.kind == TokenKind::Iri {
            let s = self.text().to_string();
            self.bump()?;
            strip_iri(&s)
        } else if self.cur.kind == TokenKind::String {
            self.expect_string()?
        } else {
            return Err(self.err("expected IRI or string path after import"));
        };
        let alias = if self.kw("as") {
            self.bump()?;
            Some(self.expect_ident()?)
        } else {
            None
        };
        let end = self
            .expect(TokenKind::Semicolon, "expected ';' after import")?
            .end;
        Ok(ImportDecl {
            span: Span::new(start, end),
            path,
            alias,
        })
    }

    pub(crate) fn parse_prefix(&mut self) -> Result<PrefixDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?;
        let prefix = self.expect_ident()?;
        self.expect(TokenKind::Colon, "expected ':' after prefix name")?;
        if self.cur.kind != TokenKind::Iri {
            return Err(self.err("expected IRI after prefix"));
        }
        let iri = strip_iri(self.text());
        self.bump()?;
        let end = self
            .expect(TokenKind::Semicolon, "expected ';' after prefix")?
            .end;
        Ok(PrefixDecl {
            span: Span::new(start, end),
            prefix,
            iri,
        })
    }

    pub(crate) fn parse_locale(&mut self) -> Result<LocaleDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // locale
        let code = self.expect_ident()?;
        let loc = match code.as_str() {
            "en" => crate::locale::Locale::EN,
            "zh" => crate::locale::Locale::ZH,
            "es" => crate::locale::Locale::ES,
            "ja" => crate::locale::Locale::JA,
            "ar" => crate::locale::Locale::AR,
            "hi" => crate::locale::Locale::HI,
            "fr" => crate::locale::Locale::FR,
            "de" => crate::locale::Locale::DE,
            other => {
                return Err(self.err(&format!(
                    "unknown locale '{other}'; valid: en, zh, es, ja, ar, hi, fr, de"
                )))
            }
        };
        self.lex.enable_locale(loc);
        let end = self
            .expect(TokenKind::Semicolon, "expected ';' after locale")?
            .end;
        Ok(LocaleDecl {
            span: Span::new(start, end),
            code,
        })
    }

    pub(crate) fn parse_using(&mut self) -> Result<Vec<CapSpec>, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // using
        let mut caps = Vec::new();
        loop {
            let family = self.expect_ident()?;
            if self.cur.kind == TokenKind::Dot {
                self.bump()?;
                self.expect(TokenKind::LBrace, "expected '{' after Family.")?;
                loop {
                    let method = self.expect_ident()?;
                    let id = format!("{family}.{method}");
                    let end = self.cur.span.end;
                    caps.push(CapSpec {
                        span: Span::new(start, end),
                        id,
                        args: Vec::new(),
                    });
                    if self.cur.kind == TokenKind::Comma {
                        self.bump()?;
                        continue;
                    }
                    break;
                }
                self.expect(TokenKind::RBrace, "expected '}' after using Family.{ ... }")?;
            } else {
                caps.push(CapSpec {
                    span: Span::new(start, self.cur.span.end),
                    id: family,
                    args: Vec::new(),
                });
            }
            if self.cur.kind == TokenKind::Comma {
                self.bump()?;
                continue;
            }
            break;
        }
        self.expect(TokenKind::Semicolon, "expected ';' after using")?;
        Ok(caps)
    }

    pub(crate) fn parse_present(&mut self) -> Result<crate::ast::PresentDecl, Diagnostic> {
        let start = self.cur.span.start;
        self.bump()?; // present
        let name = self.expect_ident()?;
        self.expect(TokenKind::LBrace, "expected '{' after present name")?;
        let mut properties = Vec::new();
        while self.cur.kind != TokenKind::RBrace && self.cur.kind != TokenKind::Eof {
            let key = self.expect_ident()?;
            self.expect(TokenKind::Colon, "expected ':' in present property")?;
            let value = self.parse_expr()?;
            properties.push(NamedArg {
                span: Span::new(start, self.cur.span.end),
                name: key,
                value,
            });
            if self.cur.kind == TokenKind::Comma || self.cur.kind == TokenKind::Semicolon {
                self.bump()?;
            }
        }
        let end = self
            .expect(TokenKind::RBrace, "expected '}' after present")?
            .end;
        if self.cur.kind == TokenKind::Semicolon {
            self.bump()?;
        }
        Ok(crate::ast::PresentDecl {
            span: Span::new(start, end),
            name,
            properties,
        })
    }

    pub(crate) fn parse_requires(&mut self) -> Result<Vec<CapSpec>, Diagnostic> {
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

    pub(crate) fn parse_cap_spec(&mut self) -> Result<CapSpec, Diagnostic> {
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
}
