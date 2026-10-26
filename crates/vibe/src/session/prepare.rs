use super::*;

impl SourceSession {
    pub fn prepare(&self, source: &str) -> Result<CheckedSubmission, Diagnostic> {
        if source.len() > self.limits.submission_bytes
            || self.source_bytes.saturating_add(source.len()) > self.limits.retained_source_bytes
            || self.revision >= self.limits.revisions
        {
            return Err(Diagnostic::new(
                DiagCode::E400,
                Span::point(0),
                "session source/revision limit exceeded",
            ));
        }
        source_limits::check(source)?;
        let trimmed = source.strip_prefix('\u{FEFF}').unwrap_or(source).trim();
        // Mode selection uses parsing only; no check/runtime fallback.
        let parsed_program = if trimmed.starts_with('=') {
            None
        } else {
            crate::parse_program(trimmed).ok()
        };
        let (mut submission, expression) = match parsed_program {
            Some(p) => (p, None),
            None => {
                let input = if trimmed.starts_with('=') {
                    trimmed.to_string()
                } else {
                    format!("= {trimmed}")
                };
                let expr = crate::parse_cell(&input)?;
                let mut p = self.definitions.clone();
                p.items = vec![Item::Cell(CellDecl {
                    span: expr.span,
                    effect: Some(EffectClass::Pure),
                    name: "__session_expression".into(),
                    params: Vec::new(),
                    expr: expr.clone(),
                    when: None,
                })];
                p.imports.clear();
                p.requires.clear();
                p.prefixes.clear();
                p.locales.clear();
                (p, Some(expr))
            }
        };
        if submission.module.is_some() {
            return Err(Diagnostic::new(
                DiagCode::E100,
                submission.span,
                "module installation is separate from source-session execution",
            ));
        }
        let mut definitions = self.definitions.clone();
        definitions.imports.extend(submission.imports.clone());
        definitions.prefixes.extend(submission.prefixes.clone());
        definitions.locales.extend(submission.locales.clone());
        for request in &submission.requires {
            if !(crate::catalog::is_known(&request.id)
                && crate::catalog::granted_covers(
                    &self.grants.iter().map(String::as_str).collect::<Vec<_>>(),
                    &request.id,
                ))
                && !self.grants.iter().any(|g| g == &request.id)
            {
                return Err(Diagnostic::new(
                    DiagCode::E300,
                    request.span,
                    format!("session host has not granted capability `{}`", request.id),
                ));
            }
            if !definitions.requires.iter().any(|c| c.id == request.id) {
                definitions.requires.push(request.clone());
            }
        }
        let mut new_names = HashSet::new();
        for item in &submission.items {
            if matches!(item, Item::Statement(s) if contains_return(s)) {
                return Err(Diagnostic::new(DiagCode::E100, submission.span,
                    "top-level return is unsupported in additive 0.1 source sessions; use a result expression"));
            }
            let mut item_names = Vec::new();
            collect_item_names(item, &mut item_names);
            for name in item_names {
                if definitions
                    .items
                    .iter()
                    .filter_map(declaration_name)
                    .any(|n| n == name)
                    || self.env.vars.contains_key(name)
                    || self.env.fields.contains_key(name)
                    || self.env.materials.contains_key(name)
                    || self.env.laws.contains_key(name)
                    || !new_names.insert(name)
                {
                    return Err(Diagnostic::new(DiagCode::E701, submission.span,
                        format!("session declaration `{name}` already exists; explicit replacement is unsupported")));
                }
            }
            if matches!(item, Item::Statement(s) if !matches!(s, Stmt::Let { .. }|Stmt::LetPat { .. }))
            {
                let mut nested_names = Vec::new();
                collect_item_names(item, &mut nested_names);
                if !nested_names.is_empty() {
                    return Err(Diagnostic::new(DiagCode::E100, submission.span,
                        "nested global declarations are unsupported in additive 0.1 source sessions; declare persistent bindings at top level"));
                }
            }
            if matches!(item, Item::Function(_) | Item::Hook(_) | Item::Enum(_)) {
                definitions.items.push(item.clone());
            }
        }
        if definitions.items.len() > self.limits.declarations {
            return Err(Diagnostic::new(
                DiagCode::E400,
                submission.span,
                "session declaration limit exceeded",
            ));
        }
        let (_, checked, mutables) =
            check_submission(&definitions, &submission, &self.checked, &self.mutables)?;
        values::validate_source(&checked)?;
        for item in &submission.items {
            if let Item::Bind(bind) = item {
                for endpoint in [&bind.left, &bind.right] {
                    if !bind_root(endpoint).is_some_and(|name| mutables.contains(name)) {
                        return Err(Diagnostic::new(
                            DiagCode::E701,
                            endpoint.span,
                            "session bind endpoint must have a declared mutable root",
                        ));
                    }
                }
            }
        }
        if checked.len() > self.limits.bindings {
            return Err(Diagnostic::new(
                DiagCode::E400,
                submission.span,
                "session binding limit exceeded",
            ));
        }
        // Execute with admitted retained requests; this does not grant new ones.
        submission.requires = definitions.requires.clone();
        Ok(CheckedSubmission {
            owner: self.owner,
            base_revision: self.revision,
            source: source.to_string(),
            definitions,
            checked,
            mutables,
            body: expression
                .map(Body::Expression)
                .unwrap_or(Body::Program(submission)),
        })
    }
}

fn declaration_name(item: &Item) -> Option<&str> {
    match item {
        Item::Function(f) => Some(&f.name),
        Item::Enum(e) => Some(&e.name),
        Item::Const(c) => Some(&c.name),
        Item::Cell(c) => Some(&c.name),
        Item::Field(f) => Some(&f.name),
        Item::Material(m) => Some(&m.name),
        Item::Law(l) => Some(&l.name),
        Item::Present(p) => Some(&p.name),
        Item::Statement(Stmt::Let { name, .. }) => Some(name),
        _ => None,
    }
}

fn bind_root(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Ident(n) => Some(n),
        ExprKind::Member { recv, .. } => bind_root(recv),
        _ => None,
    }
}

fn contains_return(stmt: &Stmt) -> bool {
    let block_returns = |block: &Block| block.stmts.iter().any(contains_return);
    match stmt {
        Stmt::Return { .. } => true,
        Stmt::If {
            then_block,
            else_block,
            ..
        } => block_returns(then_block) || else_block.as_deref().is_some_and(contains_return),
        Stmt::For { body, .. }
        | Stmt::While { body, .. }
        | Stmt::Transaction { body, .. }
        | Stmt::Block(body) => block_returns(body),
        Stmt::Match { arms, .. } => arms.iter().any(|a| {
            matches!(&a.body,
            ArmBody::Block(b) if block_returns(b))
        }),
        _ => false,
    }
}

fn collect_item_names<'a>(item: &'a Item, names: &mut Vec<&'a str>) {
    if let Item::Statement(s) = item {
        collect_stmt_names(s, names)
    } else if let Some(name) = declaration_name(item) {
        names.push(name)
    }
}
fn collect_block_names<'a>(block: &'a Block, names: &mut Vec<&'a str>) {
    for s in &block.stmts {
        collect_stmt_names(s, names)
    }
}
fn collect_stmt_names<'a>(stmt: &'a Stmt, names: &mut Vec<&'a str>) {
    match stmt {
        Stmt::Let { name, .. } => names.push(name),
        Stmt::LetPat { pattern, .. } => collect_pattern_names(pattern, names),
        Stmt::If {
            then_block,
            else_block,
            ..
        } => {
            collect_block_names(then_block, names);
            if let Some(s) = else_block {
                collect_stmt_names(s, names)
            }
        }
        Stmt::For { name, body, .. } => {
            names.push(name);
            collect_block_names(body, names)
        }
        Stmt::While { body, .. } | Stmt::Transaction { body, .. } | Stmt::Block(body) => {
            collect_block_names(body, names)
        }
        Stmt::Match { arms, .. } => {
            for arm in arms {
                collect_pattern_names(&arm.pattern, names);
                if let ArmBody::Block(b) = &arm.body {
                    collect_block_names(b, names)
                }
            }
        }
        _ => {}
    }
}
fn collect_pattern_names<'a>(pattern: &'a Pattern, names: &mut Vec<&'a str>) {
    match pattern {
        Pattern::Ident(n) => names.push(n),
        Pattern::Record(fs) => {
            for (_, p) in fs {
                collect_pattern_names(p, names)
            }
        }
        Pattern::List(ps)
        | Pattern::Constructor { args: ps, .. }
        | Pattern::Variant { args: ps, .. } => {
            for p in ps {
                collect_pattern_names(p, names)
            }
        }
        Pattern::Ok(p) | Pattern::Err(p) | Pattern::Some(p) => collect_pattern_names(p, names),
        _ => {}
    }
}
