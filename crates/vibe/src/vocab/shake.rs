//! Tree-shake referenced terms and keep parent IRIs (P17.2).

use crate::ast::{Expr, ExprKind, Item, Program, Stmt};
use crate::span::Span;

use super::{VocabChunk, VocabTerm};

/// Keep referenced terms plus `rdfs:subClassOf` parents. Canonical IRIs unchanged.
pub fn tree_shake(chunk: &VocabChunk, referenced: &[&str]) -> VocabChunk {
    let mut keep: Vec<String> = Vec::new();
    for r in referenced {
        if let Some(iri) = chunk.expand(r).or_else(|| {
            if r.contains("://") {
                Some((*r).to_string())
            } else {
                None
            }
        }) {
            keep.push(iri);
        }
        if let Some(t) = chunk.terms.iter().find(|t| t.label.as_deref() == Some(*r)) {
            keep.push(t.iri.clone());
        }
    }
    let mut out_iris = Vec::new();
    while let Some(iri) = keep.pop() {
        if out_iris.iter().any(|x| x == &iri) {
            continue;
        }
        out_iris.push(iri.clone());
        if let Some(term) = chunk.terms.iter().find(|t| t.iri == iri) {
            for p in &term.parents {
                keep.push(p.clone());
            }
        }
    }
    let terms: Vec<VocabTerm> = chunk
        .terms
        .iter()
        .filter(|t| out_iris.iter().any(|i| i == &t.iri))
        .cloned()
        .collect();
    VocabChunk {
        prefixes: chunk.prefixes.clone(),
        terms,
        content_hash: chunk.content_hash,
    }
}

/// Canonical IRIs used by `prefix:local` names in the program that this chunk owns.
pub fn project_referenced_iris(program: &Program, chunk: &VocabChunk) -> Vec<String> {
    let mut iris = Vec::new();
    for (prefix, local, _) in collect_prefixed(program) {
        if let Some(iri) = chunk.expand(&format!("{prefix}:{local}")) {
            if !iris.contains(&iri) {
                iris.push(iri);
            }
        }
    }
    iris
}

/// Prefixed names whose prefix is in a chunk but whose local name is not.
pub fn unknown_prefixed(program: &Program, chunks: &[VocabChunk]) -> Vec<(String, String, Span)> {
    let mut out = Vec::new();
    for (prefix, local, span) in collect_prefixed(program) {
        let Some(chunk) = chunks.iter().find(|c| c.prefixes.contains_key(&prefix)) else {
            continue;
        };
        if !chunk.has_local(&prefix, &local) {
            out.push((prefix, local, span));
        }
    }
    out
}

fn collect_prefixed(program: &Program) -> Vec<(String, String, Span)> {
    let mut out = Vec::new();
    for item in &program.items {
        match item {
            Item::Function(f) => walk_block(&f.body, &mut out),
            Item::Hook(h) => walk_block(&h.body, &mut out),
            Item::Const(c) => walk_expr(&c.value, &mut out),
            Item::Cell(c) => {
                walk_expr(&c.expr, &mut out);
                if let Some(w) = &c.when {
                    walk_expr(w, &mut out);
                }
            }
            Item::Law(l) => {
                walk_expr(&l.condition, &mut out);
                walk_expr(&l.consequence, &mut out);
            }
            Item::Bind(b) => {
                walk_expr(&b.left, &mut out);
                walk_expr(&b.right, &mut out);
            }
            Item::Present(p) => {
                for prop in &p.properties {
                    walk_expr(&prop.value, &mut out);
                }
            }
            Item::Statement(s) => walk_stmt(s, &mut out),
            Item::Enum(_) | Item::Field(_) | Item::Material(_) => {}
        }
    }
    out
}

fn walk_block(block: &crate::ast::Block, out: &mut Vec<(String, String, Span)>) {
    for s in &block.stmts {
        walk_stmt(s, out);
    }
}

fn walk_stmt(stmt: &Stmt, out: &mut Vec<(String, String, Span)>) {
    match stmt {
        Stmt::Let { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, out);
            }
        }
        Stmt::LetPat { value, .. } => walk_expr(value, out),
        Stmt::Assign { target, value, .. } => {
            walk_expr(target, out);
            walk_expr(value, out);
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            walk_expr(cond, out);
            walk_block(then_block, out);
            if let Some(e) = else_block {
                walk_stmt(e, out);
            }
        }
        Stmt::While { cond, body, .. } | Stmt::For { iter: cond, body, .. } => {
            walk_expr(cond, out);
            walk_block(body, out);
        }
        Stmt::Match { scrutinee, arms, .. } => {
            walk_expr(scrutinee, out);
            for arm in arms {
                match &arm.body {
                    crate::ast::ArmBody::Block(b) => walk_block(b, out),
                    crate::ast::ArmBody::Expr(e) => walk_expr(e, out),
                }
            }
        }
        Stmt::Return { value, .. } | Stmt::Yield { value, .. } => {
            if let Some(v) = value {
                walk_expr(v, out);
            }
        }
        Stmt::Transaction { body, .. } => walk_block(body, out),
        Stmt::Effect { expr, .. } | Stmt::Expr { expr, .. } => walk_expr(expr, out),
        Stmt::Block(b) => walk_block(b, out),
    }
}

fn walk_expr(expr: &Expr, out: &mut Vec<(String, String, Span)>) {
    match &expr.kind {
        ExprKind::Prefixed(p, l) => out.push((p.clone(), l.clone(), expr.span)),
        ExprKind::Call { callee, args } => {
            walk_expr(callee, out);
            for a in args {
                match a {
                    crate::ast::Arg::Pos(e) => walk_expr(e, out),
                    crate::ast::Arg::Named(n) => walk_expr(&n.value, out),
                }
            }
        }
        ExprKind::Member { recv, .. } => walk_expr(recv, out),
        ExprKind::Binary { left, right, .. } => {
            walk_expr(left, out);
            walk_expr(right, out);
        }
        ExprKind::Unary { expr: inner, .. } => walk_expr(inner, out),
        ExprKind::List(xs) => {
            for x in xs {
                walk_expr(x, out);
            }
        }
        ExprKind::Record(fs) => {
            for n in fs {
                walk_expr(&n.value, out);
            }
        }
        ExprKind::Await(inner) | ExprKind::Try(inner) => walk_expr(inner, out),
        ExprKind::Triple {
            subject,
            predicate,
            object,
        }
        | ExprKind::Reified {
            subject,
            predicate,
            object,
            ..
        } => {
            walk_expr(subject, out);
            walk_expr(predicate, out);
            walk_expr(object, out);
        }
        ExprKind::Interpolate(xs) => {
            for x in xs {
                walk_expr(x, out);
            }
        }
        ExprKind::Pipe { left, right } => {
            walk_expr(left, out);
            walk_expr(right, out);
        }
        ExprKind::Index { recv, index } => {
            walk_expr(recv, out);
            walk_expr(index, out);
        }
        ExprKind::Lambda { body, .. } => walk_expr(body, out),
        ExprKind::Tween { from, to, over, .. } => {
            walk_expr(from, out);
            walk_expr(to, out);
            walk_expr(over, out);
        }
        ExprKind::ModalLogic { args, body, .. } => {
            for a in args {
                walk_expr(a, out);
            }
            if let Some(b) = body {
                walk_expr(b, out);
            }
        }
        _ => {}
    }
}
