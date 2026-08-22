//! Projectional authoring — structure is the source of truth, text is a view (W1).
//!
//! This module implements the projectional authoring model for VibeScript:
//!
//! - **Project**: Convert an AST `Program` back to canonical source text.
//!   The AST is the source of truth; the text is a *projection* of it.
//! - **Edit**: Apply structural edits to a `Program` without round-tripping
//!   through text. Edits are typed operations, not string patches.
//! - **Round-trip**: `parse → project → parse` yields an equivalent AST.
//! - **Trivia preservation**: When a CST is available (trivia collected),
//!   the projector can interleave comments and whitespace from the original
//!   source.
//!
//! This stops LLMs and humans from forking a second syntax: the structure
//! is authoritative, and text is always derivable from it.
//!
//! Reference: `docs/vibescript-full-impl-PLAN.md` W1.

use std::fmt::Write;

use crate::ast::{
    Arg, BinOp, Block, CellDecl, ConstDecl, EffectClass, EnumDecl, Expr, ExprKind, FieldDecl,
    FieldRepresentation, FieldSupport, FunctionDecl, HookDecl, Item, LawDecl, Literal,
    MaterialDecl, ModalKind, NamedArg, Pattern, Program, Stmt, TypeExpr, UnOp,
};
use crate::trivia::CstNode;

// ── projection options ─────────────────────────────────────────────

/// Options controlling how the projector formats source text.
#[derive(Debug, Clone)]
pub struct ProjectOptions {
    /// Indentation string (default: two spaces).
    pub indent: String,
    /// Blank lines between top-level declarations (default: 1).
    pub blank_lines_between_decls: usize,
    /// Maximum line width before wrapping (default: 80).
    pub max_line_width: usize,
}

impl Default for ProjectOptions {
    fn default() -> Self {
        Self {
            indent: "  ".into(),
            blank_lines_between_decls: 1,
            max_line_width: 80,
        }
    }
}

// ── projector ──────────────────────────────────────────────────────

/// Project a `Program` AST to canonical source text.
///
/// The output is valid VibeScript that, when re-parsed, yields an
/// equivalent AST (modulo spans and trivia).
pub fn project_program(prog: &Program, opts: &ProjectOptions) -> String {
    let mut out = String::new();

    for loc in &prog.locales {
        write!(out, "locale {};\n", loc.code).unwrap();
    }

    // Module declaration
    if let Some(module) = &prog.module {
        project_name(&module.name, "module ", &mut out);
        out.push('\n');
    }

    // Imports
    for imp in &prog.imports {
        write!(out, "import \"{}\"", imp.path).unwrap();
        if let Some(alias) = &imp.alias {
            write!(out, " as {}", alias).unwrap();
        }
        out.push('\n');
    }

    // Prefixes
    for p in &prog.prefixes {
        write!(out, "prefix {}: <{}>\n", p.prefix, p.iri).unwrap();
    }

    // Requires
    if !prog.requires.is_empty() {
        out.push_str("requires [\n");
        for (i, cap) in prog.requires.iter().enumerate() {
            write!(out, "{}capability(\"{}\"", opts.indent, cap.id).unwrap();
            for arg in &cap.args {
                write!(out, ", {}: ", arg.name).unwrap();
                project_expr(&arg.value, opts, &mut out);
            }
            out.push(')');
            if i + 1 < prog.requires.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("];\n");
    }

    // Items
    let blank = "\n".repeat(opts.blank_lines_between_decls);
    let mut first = true;
    for item in &prog.items {
        if !first {
            out.push_str(&blank);
        }
        project_item(item, opts, &mut out);
        first = false;
    }

    out
}

/// Project a single top-level item.
pub fn project_item(item: &Item, opts: &ProjectOptions, out: &mut String) {
    match item {
        Item::Function(fd) => project_function(fd, opts, out),
        Item::Hook(hd) => project_hook(hd, opts, out),
        Item::Const(cd) => project_const(cd, opts, out),
        Item::Enum(ed) => project_enum(ed, opts, out),
        Item::Field(fd) => project_field(fd, opts, out),
        Item::Material(md) => project_material(md, opts, out),
        Item::Law(ld) => project_law(ld, opts, out),
        Item::Cell(cd) => project_cell(cd, opts, out),
        Item::Present(pd) => {
            out.push_str("present ");
            out.push_str(&pd.name);
            out.push_str(" {\n");
            for prop in &pd.properties {
                out.push_str(&opts.indent);
                out.push_str(&prop.name);
                out.push_str(": ");
                project_expr(&prop.value, opts, out);
                out.push('\n');
            }
            out.push_str("}\n");
        }
        Item::Bind(b) => {
            out.push_str("bind ");
            project_expr(&b.left, opts, out);
            out.push_str(" <-> ");
            project_expr(&b.right, opts, out);
            if let Some((lo, hi)) = &b.clamp {
                out.push_str(" using Clamp[");
                project_expr(lo, opts, out);
                out.push_str(", ");
                project_expr(hi, opts, out);
                out.push(']');
            }
            out.push_str(" resolve ");
            out.push_str(match b.resolve {
                crate::ast::BindResolve::Latest => "latest",
                crate::ast::BindResolve::Left => "left",
                crate::ast::BindResolve::Right => "right",
            });
            out.push_str(";\n");
        }
        Item::Statement(s) => {
            project_stmt(s, opts, out, 0);
            out.push('\n');
        }
    }
}

fn project_cell(cd: &CellDecl, opts: &ProjectOptions, out: &mut String) {
    if let Some(effect) = cd.effect {
        write!(out, "{} ", effect_keyword(effect)).unwrap();
    }
    write!(out, "cell {}", cd.name).unwrap();
    if !cd.params.is_empty() {
        out.push('(');
        for (i, p) in cd.params.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            write!(out, "{}: ", p.name).unwrap();
            project_type(&p.ty, out);
        }
        out.push(')');
    }
    if let Some(w) = &cd.when {
        out.push_str(" when ");
        project_expr(w, opts, out);
    }
    out.push_str(" := ");
    project_expr(&cd.expr, opts, out);
    out.push_str(";\n");
}

fn project_function(fd: &FunctionDecl, opts: &ProjectOptions, out: &mut String) {
    if let Some(effect) = fd.effect {
        write!(out, "{} ", effect_keyword(effect)).unwrap();
    }
    if fd.is_async {
        out.push_str("async ");
    }
    write!(out, "fn {}(", fd.name).unwrap();
    for (i, p) in fd.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write!(out, "{}: ", p.name).unwrap();
        project_type(&p.ty, out);
    }
    out.push(')');
    if !fd.budget.is_empty() {
        out.push_str(" budget(");
        for (i, b) in fd.budget.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            write!(out, "{}: ", b.name).unwrap();
            project_expr(&b.value, opts, out);
        }
        out.push(')');
    }
    if let Some(ret) = &fd.ret {
        out.push_str(" -> ");
        project_type(ret, out);
    }
    out.push(' ');
    project_block(&fd.body, opts, out, 0);
    out.push('\n');
}

fn project_hook(hd: &HookDecl, opts: &ProjectOptions, out: &mut String) {
    out.push_str("on ");
    for (i, seg) in hd.path.iter().enumerate() {
        if i > 0 {
            out.push(':');
        }
        out.push_str(seg);
    }
    out.push('(');
    for (i, p) in hd.params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        write!(out, "{}: ", p.name).unwrap();
        project_type(&p.ty, out);
    }
    out.push(')');
    if !hd.budget.is_empty() {
        out.push_str(" budget(");
        for (i, b) in hd.budget.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            write!(out, "{}: ", b.name).unwrap();
            project_expr(&b.value, opts, out);
        }
        out.push(')');
    }
    if let Some(ret) = &hd.ret {
        out.push_str(" -> ");
        project_type(ret, out);
    }
    out.push(' ');
    project_block(&hd.body, opts, out, 0);
    out.push('\n');
}

fn project_const(cd: &ConstDecl, opts: &ProjectOptions, out: &mut String) {
    out.push_str("const ");
    out.push_str(&cd.name);
    if let Some(ty) = &cd.ty {
        out.push_str(": ");
        project_type(ty, out);
    }
    out.push_str(" = ");
    project_expr(&cd.value, opts, out);
    out.push_str(";\n");
}

fn project_enum(ed: &EnumDecl, opts: &ProjectOptions, out: &mut String) {
    write!(out, "enum {} {{\n", ed.name).unwrap();
    for variant in &ed.variants {
        out.push_str(&opts.indent);
        out.push_str(&variant.name);
        if !variant.payload.is_empty() {
            out.push('(');
            for (i, ty) in variant.payload.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                project_type(ty, out);
            }
            out.push(')');
        }
        out.push_str(",\n");
    }
    out.push_str("}\n");
}

fn project_field(fd: &FieldDecl, opts: &ProjectOptions, out: &mut String) {
    write!(out, "field {}: ", fd.name).unwrap();
    project_type(&fd.ty, out);
    let mut props: Vec<(String, String)> = Vec::new();
    if let Some(unit) = &fd.unit {
        props.push(("unit".into(), format!("<{}>", unit)));
    }
    if fd.support != FieldSupport::Region {
        props.push(("support".into(), format!("{:?}", fd.support).to_lowercase()));
    }
    if fd.representation != FieldRepresentation::Grid {
        props.push((
            "representation".into(),
            format!("{:?}", fd.representation).to_lowercase(),
        ));
    }
    if props.is_empty() {
        out.push_str(";\n");
        return;
    }
    out.push('\n');
    for (k, v) in props.iter() {
        write!(out, "{}{}: {}\n", opts.indent, k, v).unwrap();
    }
    out.push_str(";\n");
}

fn project_material(md: &MaterialDecl, opts: &ProjectOptions, out: &mut String) {
    write!(out, "material {}: Material", md.name).unwrap();
    if md.properties.is_empty() {
        out.push_str(";\n");
        return;
    }
    out.push('\n');
    for (i, prop) in md.properties.iter().enumerate() {
        write!(out, "{}{}: ", opts.indent, prop.name).unwrap();
        project_expr(&prop.value, opts, out);
        if i + 1 < md.properties.len() {
            out.push(',');
        }
        out.push('\n');
    }
    out.push_str(";\n");
}

fn project_law(ld: &LawDecl, opts: &ProjectOptions, out: &mut String) {
    write!(out, "law {}\n", ld.name).unwrap();
    write!(out, "{}when ", opts.indent).unwrap();
    project_expr(&ld.condition, opts, out);
    write!(out, "\n{}=> ", opts.indent).unwrap();
    project_expr(&ld.consequence, opts, out);
    out.push_str(";\n");
}

fn project_block(block: &Block, opts: &ProjectOptions, out: &mut String, depth: usize) {
    out.push('{');
    if block.stmts.is_empty() {
        out.push('}');
        return;
    }
    out.push('\n');
    for stmt in &block.stmts {
        project_stmt(stmt, opts, out, depth + 1);
        out.push('\n');
    }
    for _ in 0..depth {
        out.push_str(&opts.indent);
    }
    out.push('}');
}

fn project_stmt(stmt: &Stmt, opts: &ProjectOptions, out: &mut String, depth: usize) {
    let indent = opts.indent.repeat(depth);
    match stmt {
        Stmt::Let {
            mutable,
            name,
            ty,
            value,
            ..
        } => {
            out.push_str(&indent);
            out.push_str("let ");
            if *mutable {
                out.push_str("mut ");
            }
            out.push_str(name);
            if let Some(ty) = ty {
                out.push_str(": ");
                project_type(ty, out);
            }
            if let Some(value) = value {
                out.push_str(" = ");
                project_expr(value, opts, out);
            }
            out.push(';');
        }
        Stmt::LetPat {
            mutable,
            pattern,
            ty,
            value,
            ..
        } => {
            out.push_str(&indent);
            out.push_str("let ");
            if *mutable {
                out.push_str("mut ");
            }
            project_pattern(pattern, out);
            if let Some(t) = ty {
                out.push_str(": ");
                project_type(t, out);
            }
            out.push_str(" = ");
            project_expr(value, opts, out);
            out.push(';');
        }
        Stmt::Assign { target, value, .. } => {
            out.push_str(&indent);
            project_expr(target, opts, out);
            out.push_str(" = ");
            project_expr(value, opts, out);
            out.push(';');
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            out.push_str(&indent);
            out.push_str("if ");
            project_expr(cond, opts, out);
            out.push(' ');
            project_block(then_block, opts, out, depth);
            if let Some(else_stmt) = else_block {
                out.push_str(" else ");
                match else_stmt.as_ref() {
                    Stmt::If { .. } => {
                        // else if — no brace, inline
                        project_stmt(else_stmt, opts, out, depth);
                    }
                    _ => {
                        project_block(
                            match else_stmt.as_ref() {
                                Stmt::Block(b) => b,
                                _ => {
                                    // Wrap single statement in block
                                    return;
                                }
                            },
                            opts,
                            out,
                            depth,
                        );
                    }
                }
            }
        }
        Stmt::For {
            name, iter, body, ..
        } => {
            out.push_str(&indent);
            write!(out, "for {} in ", name).unwrap();
            project_expr(iter, opts, out);
            out.push(' ');
            project_block(body, opts, out, depth);
        }
        Stmt::While { cond, body, .. } => {
            out.push_str(&indent);
            out.push_str("while ");
            project_expr(cond, opts, out);
            out.push(' ');
            project_block(body, opts, out, depth);
        }
        Stmt::Match {
            scrutinee, arms, ..
        } => {
            out.push_str(&indent);
            out.push_str("match ");
            project_expr(scrutinee, opts, out);
            out.push_str(" {\n");
            for arm in arms {
                out.push_str(&opts.indent.repeat(depth + 1));
                project_pattern(&arm.pattern, out);
                out.push_str(" => ");
                match &arm.body {
                    crate::ast::ArmBody::Block(b) => {
                        project_block(b, opts, out, depth + 1);
                    }
                    crate::ast::ArmBody::Expr(e) => {
                        project_expr(e, opts, out);
                    }
                }
                out.push_str(",\n");
            }
            out.push_str(&opts.indent.repeat(depth));
            out.push('}');
        }
        Stmt::Return { value, .. } => {
            out.push_str(&indent);
            out.push_str("return");
            if let Some(v) = value {
                out.push(' ');
                project_expr(v, opts, out);
            }
            out.push(';');
        }
        Stmt::Yield { value, .. } => {
            out.push_str(&indent);
            out.push_str("yield");
            if let Some(v) = value {
                out.push(' ');
                project_expr(v, opts, out);
            }
            out.push(';');
        }
        Stmt::Transaction { args, body, .. } => {
            out.push_str(&indent);
            out.push_str("transaction");
            if !args.is_empty() {
                out.push('(');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    write!(out, "{}: ", a.name).unwrap();
                    project_expr(&a.value, opts, out);
                }
                out.push(')');
            }
            out.push(' ');
            project_block(body, opts, out, depth);
        }
        Stmt::Effect { expr, .. } => {
            out.push_str(&indent);
            out.push_str("effect ");
            project_expr(expr, opts, out);
            out.push(';');
        }
        Stmt::Expr { expr, .. } => {
            out.push_str(&indent);
            project_expr(expr, opts, out);
            out.push(';');
        }
        Stmt::Block(b) => {
            out.push_str(&indent);
            project_block(b, opts, out, depth);
        }
    }
}

fn project_pattern(pat: &Pattern, out: &mut String) {
    match pat {
        Pattern::Wildcard => out.push('_'),
        Pattern::Ident(s) => out.push_str(s),
        Pattern::Literal(l) => project_literal(l, out),
        Pattern::Ok(inner) => {
            out.push_str("Ok(");
            project_pattern(inner, out);
            out.push(')');
        }
        Pattern::Err(inner) => {
            out.push_str("Err(");
            project_pattern(inner, out);
            out.push(')');
        }
        Pattern::Some(inner) => {
            out.push_str("Some(");
            project_pattern(inner, out);
            out.push(')');
        }
        Pattern::None => out.push_str("None"),
        Pattern::Record(fields) => {
            out.push_str("{ ");
            for (i, (k, p)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                if let Pattern::Ident(id) = p {
                    if id == k {
                        out.push_str(k);
                        continue;
                    }
                }
                out.push_str(k);
                out.push_str(": ");
                project_pattern(p, out);
            }
            out.push_str(" }");
        }
        Pattern::List(elements) => {
            out.push('[');
            for (i, p) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                project_pattern(p, out);
            }
            out.push(']');
        }
        Pattern::Constructor { name, args } => {
            out.push_str(name);
            out.push('(');
            for (i, p) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                project_pattern(p, out);
            }
            out.push(')');
        }
        Pattern::Variant {
            enum_name,
            variant_name,
            args,
        } => {
            write!(out, "{}.{}", enum_name, variant_name).unwrap();
            if !args.is_empty() {
                out.push('(');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    project_pattern(a, out);
                }
                out.push(')');
            }
        }
    }
}

fn project_expr(expr: &Expr, opts: &ProjectOptions, out: &mut String) {
    match &expr.kind {
        ExprKind::Literal(l) => project_literal(l, out),
        ExprKind::Ident(s) => out.push_str(s),
        ExprKind::QueryVar(s) => {
            out.push('?');
            out.push_str(s);
        }
        ExprKind::Iri(s) => {
            out.push('<');
            out.push_str(s);
            out.push('>');
        }
        ExprKind::Prefixed(p, l) => {
            write!(out, "{}:{}", p, l).unwrap();
        }
        ExprKind::Blank(s) => {
            out.push_str("_:");
            out.push_str(s);
        }
        ExprKind::Binary { op, left, right } => {
            project_expr(left, opts, out);
            out.push(' ');
            out.push_str(binop_str(*op));
            out.push(' ');
            project_expr(right, opts, out);
        }
        ExprKind::Unary { op, expr } => {
            out.push_str(unop_str(*op));
            project_expr(expr, opts, out);
        }
        ExprKind::Await(inner) => {
            project_expr(inner, opts, out);
            out.push_str(".await");
        }
        ExprKind::Member { recv, name } => {
            project_expr(recv, opts, out);
            out.push('.');
            out.push_str(name);
        }
        ExprKind::Call { callee, args } => {
            project_expr(callee, opts, out);
            out.push('(');
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match arg {
                    Arg::Pos(e) => project_expr(e, opts, out),
                    Arg::Named(na) => {
                        write!(out, "{}: ", na.name).unwrap();
                        project_expr(&na.value, opts, out);
                    }
                }
            }
            out.push(')');
        }
        ExprKind::Index { recv, index } => {
            project_expr(recv, opts, out);
            out.push('[');
            project_expr(index, opts, out);
            out.push(']');
        }
        ExprKind::Try(inner) => {
            project_expr(inner, opts, out);
            out.push('?');
        }
        ExprKind::List(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                project_expr(item, opts, out);
            }
            out.push(']');
        }
        ExprKind::Record(entries) => {
            out.push('{');
            if !entries.is_empty() {
                out.push(' ');
            }
            for (i, entry) in entries.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                write!(out, "{}: ", entry.name).unwrap();
                project_expr(&entry.value, opts, out);
            }
            if !entries.is_empty() {
                out.push(' ');
            }
            out.push('}');
        }
        ExprKind::Triple {
            subject,
            predicate,
            object,
        } => {
            out.push_str("<<(");
            project_expr(subject, opts, out);
            out.push(' ');
            project_expr(predicate, opts, out);
            out.push(' ');
            project_expr(object, opts, out);
            out.push_str(")>>");
        }
        ExprKind::Reified {
            subject,
            predicate,
            object,
            reifier,
        } => {
            out.push_str("<<(");
            project_expr(subject, opts, out);
            out.push(' ');
            project_expr(predicate, opts, out);
            out.push(' ');
            project_expr(object, opts, out);
            out.push_str(") ~ ");
            project_expr(reifier, opts, out);
            out.push_str(">>");
        }
        ExprKind::Pipe { left, right } => {
            project_expr(left, opts, out);
            out.push_str(" |> ");
            project_expr(right, opts, out);
        }
        ExprKind::GraphQuery { is_ask, pattern, .. } => {
            if *is_ask {
                out.push_str("graph? { ");
            } else {
                out.push_str("graph { ");
            }
            out.push_str(pattern);
            out.push_str(" }");
        }
        ExprKind::ModalLogic { modality, args, body } => {
            let name = match modality {
                ModalKind::DeonticObligate => "obligate",
                ModalKind::DeonticPermit => "permit",
                ModalKind::DeonticForbid => "forbid",
                ModalKind::EpistemicKnows => "knows",
                ModalKind::EpistemicBelieves => "believes",
                ModalKind::Paraconsistent => "paraconsistent",
                ModalKind::LtlGlobally => "always",
                ModalKind::LtlFinally => "eventually",
                ModalKind::LtlUntil => "until",
                ModalKind::DlSubsumes => "subsumes",
                ModalKind::N3Defeasible => "defeasible_rule",
            };
            out.push_str(name);
            if !args.is_empty() {
                out.push('(');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    project_expr(a, opts, out);
                }
                out.push(')');
            }
            if let Some(b) = body {
                out.push_str(" { ");
                project_expr(b, opts, out);
                out.push_str(" }");
            }
        }
        ExprKind::Interpolate(parts) => {
            out.push_str("f\"");
            for p in parts {
                if let ExprKind::Literal(Literal::String(s)) = &p.kind {
                    out.push_str(s);
                } else {
                    out.push('{');
                    project_expr(p, opts, out);
                    out.push('}');
                }
            }
            out.push('"');
        }
        ExprKind::Lambda { params, body } => {
            out.push('|');
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(p);
            }
            out.push_str("| ");
            project_expr(body, opts, out);
        }
        ExprKind::Tween {
            from,
            to,
            over,
            ease,
            spring,
        } => {
            project_expr(from, opts, out);
            out.push_str(" ~ ");
            project_expr(to, opts, out);
            out.push_str(" over ");
            project_expr(over, opts, out);
            if let Some(e) = ease {
                out.push_str(" ease ");
                out.push_str(e);
            }
            if let Some(args) = spring {
                out.push_str(" spring(");
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    write!(out, "{}: ", a.name).unwrap();
                    project_expr(&a.value, opts, out);
                }
                out.push(')');
            }
        }
    }
}

fn project_literal(lit: &Literal, out: &mut String) {
    match lit {
        Literal::Null => out.push_str("null"),
        Literal::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Literal::Int(n) => write!(out, "{}", n).unwrap(),
        Literal::UInt(n) => write!(out, "{}u", n).unwrap(),
        Literal::Float(bits) => {
            let f = f64::from_bits(*bits);
            if f == f.trunc() && f.is_finite() {
                write!(out, "{:.1}", f).unwrap();
            } else {
                write!(out, "{}", f).unwrap();
            }
        }
        Literal::Quantity { value, unit } => {
            let f = f64::from_bits(*value);
            write!(out, "{}{}", f, unit).unwrap();
        }
        Literal::Color(c) => {
            write!(out, "#{:02x}{:02x}{:02x}", c.r, c.g, c.b).unwrap();
            if c.a != 255 {
                write!(out, "{:02x}", c.a).unwrap();
            }
        }
        Literal::String(s) => {
            out.push('"');
            out.push_str(s);
            out.push('"');
        }
    }
}

fn project_type(ty: &TypeExpr, out: &mut String) {
    out.push_str(&ty.name);
    if !ty.args.is_empty() {
        out.push('<');
        for (i, a) in ty.args.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            project_type(a, out);
        }
        out.push('>');
    }
}

fn project_name(name: &crate::ast::Name, prefix: &str, out: &mut String) {
    out.push_str(prefix);
    match name {
        crate::ast::Name::Ident(s) => out.push_str(s),
        crate::ast::Name::Iri(s) => {
            out.push('<');
            out.push_str(s);
            out.push('>');
        }
    }
}

fn effect_keyword(e: EffectClass) -> &'static str {
    match e {
        EffectClass::Pure => "pure",
        EffectClass::Hot => "hot",
        EffectClass::Cold => "cold",
        EffectClass::Async => "async",
        EffectClass::External => "effect",
    }
}

fn binop_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Or => "||",
        BinOp::And => "&&",
        BinOp::Eq => "==",
        BinOp::Ne => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Rem => "%",
    }
}

fn unop_str(op: UnOp) -> &'static str {
    match op {
        UnOp::Not => "!",
        UnOp::Neg => "-",
        UnOp::Plus => "+",
    }
}

// ── structural edits ───────────────────────────────────────────────

/// A structural edit operation on a `Program`.
///
/// Edits are typed — they operate on the AST structure, not on text.
/// This is the core of projectional authoring: the structure is
/// authoritative, text is derived.
#[derive(Debug, Clone)]
pub enum Edit {
    /// Add a new top-level item at the end (or at a specific index).
    AddItem { item: Item, index: Option<usize> },
    /// Remove the item at the given index.
    RemoveItem { index: usize },
    /// Replace the item at the given index.
    ReplaceItem { index: usize, item: Item },
    /// Rename a function, const, field, material, or law.
    RenameItem { index: usize, new_name: String },
    /// Modify a field's unit.
    SetFieldUnit { index: usize, unit: Option<String> },
    /// Modify a field's support.
    SetFieldSupport { index: usize, support: FieldSupport },
    /// Modify a field's representation.
    SetFieldRepresentation {
        index: usize,
        representation: FieldRepresentation,
    },
    /// Add a property to a material.
    AddMaterialProperty { index: usize, property: NamedArg },
    /// Remove a property from a material by name.
    RemoveMaterialProperty { index: usize, name: String },
    /// Set a law's condition.
    SetLawCondition { index: usize, condition: Expr },
    /// Set a law's consequence.
    SetLawConsequence { index: usize, consequence: Expr },
    /// Add a prefix declaration.
    AddPrefix { prefix: String, iri: String },
    /// Remove a prefix declaration by prefix name.
    RemovePrefix { prefix: String },
    /// Add a requires capability.
    AddRequires { cap: crate::ast::CapSpec },
    /// Remove a requires capability by id.
    RemoveRequires { id: String },
}

/// Apply a structural edit to a program, returning a new program.
///
/// This does not modify the original — projectional authoring treats
/// the AST as immutable structure; edits produce new structure.
pub fn apply_edit(prog: &Program, edit: &Edit) -> Program {
    let mut p = prog.clone();
    match edit {
        Edit::AddItem { item, index } => {
            if let Some(i) = index {
                p.items.insert(*i, item.clone());
            } else {
                p.items.push(item.clone());
            }
        }
        Edit::RemoveItem { index } => {
            if *index < p.items.len() {
                p.items.remove(*index);
            }
        }
        Edit::ReplaceItem { index, item } => {
            if *index < p.items.len() {
                p.items[*index] = item.clone();
            }
        }
        Edit::RenameItem { index, new_name } => {
            if *index < p.items.len() {
                rename_item(&mut p.items[*index], new_name);
            }
        }
        Edit::SetFieldUnit { index, unit } => {
            if let Some(Item::Field(fd)) = p.items.get_mut(*index) {
                fd.unit = unit.clone();
            }
        }
        Edit::SetFieldSupport { index, support } => {
            if let Some(Item::Field(fd)) = p.items.get_mut(*index) {
                fd.support = *support;
            }
        }
        Edit::SetFieldRepresentation {
            index,
            representation,
        } => {
            if let Some(Item::Field(fd)) = p.items.get_mut(*index) {
                fd.representation = *representation;
            }
        }
        Edit::AddMaterialProperty { index, property } => {
            if let Some(Item::Material(md)) = p.items.get_mut(*index) {
                md.properties.push(property.clone());
            }
        }
        Edit::RemoveMaterialProperty { index, name } => {
            if let Some(Item::Material(md)) = p.items.get_mut(*index) {
                md.properties.retain(|p| p.name != *name);
            }
        }
        Edit::SetLawCondition { index, condition } => {
            if let Some(Item::Law(ld)) = p.items.get_mut(*index) {
                ld.condition = condition.clone();
            }
        }
        Edit::SetLawConsequence { index, consequence } => {
            if let Some(Item::Law(ld)) = p.items.get_mut(*index) {
                ld.consequence = consequence.clone();
            }
        }
        Edit::AddPrefix { prefix, iri } => {
            p.prefixes.push(crate::ast::PrefixDecl {
                span: crate::span::Span::point(0),
                prefix: prefix.clone(),
                iri: iri.clone(),
            });
        }
        Edit::RemovePrefix { prefix } => {
            p.prefixes.retain(|p| p.prefix != *prefix);
        }
        Edit::AddRequires { cap } => {
            p.requires.push(cap.clone());
        }
        Edit::RemoveRequires { id } => {
            p.requires.retain(|c| c.id != *id);
        }
    }
    p
}

/// Apply a sequence of edits, producing the final program.
pub fn apply_edits(prog: &Program, edits: &[Edit]) -> Program {
    let mut p = prog.clone();
    for edit in edits {
        p = apply_edit(&p, edit);
    }
    p
}

fn rename_item(item: &mut Item, new_name: &str) {
    match item {
        Item::Function(fd) => fd.name = new_name.to_string(),
        Item::Const(cd) => cd.name = new_name.to_string(),
        Item::Field(fd) => fd.name = new_name.to_string(),
        Item::Material(md) => md.name = new_name.to_string(),
        Item::Law(ld) => ld.name = new_name.to_string(),
        Item::Enum(ed) => ed.name = new_name.to_string(),
        _ => {}
    }
}

// ── trivia-aware projection ────────────────────────────────────────

/// Project a program with trivia preservation.
///
/// Given a CST (program with trivia), the projector interleaves
/// comments and whitespace from the original source into the
/// projected text. This is a best-effort process — structural edits
/// may invalidate original trivia positions.
pub fn project_with_trivia(
    prog: &Program,
    cst_items: &[CstNode<Item>],
    opts: &ProjectOptions,
) -> String {
    let mut out = String::new();

    // Project the program structure first
    let base = project_program(prog, opts);

    // If we have CST items, try to interleave leading comments
    // from each item. This is a simple approach: for each CST item,
    // extract leading comments and insert them before the projected
    // item text.
    if cst_items.is_empty() {
        return base;
    }

    // Re-project item by item, prepending leading trivia
    let blank = "\n".repeat(opts.blank_lines_between_decls);
    let mut first = true;

    // Project headers (module, imports, prefixes, requires) from base
    // by finding where items start
    // Simple approach: project headers, then items with trivia
    for loc in &prog.locales {
        write!(out, "locale {};\n", loc.code).unwrap();
    }
    if let Some(module) = &prog.module {
        project_name(&module.name, "module ", &mut out);
        out.push('\n');
    }
    for imp in &prog.imports {
        write!(out, "import \"{}\"", imp.path).unwrap();
        if let Some(alias) = &imp.alias {
            write!(out, " as {}", alias).unwrap();
        }
        out.push('\n');
    }
    for p in &prog.prefixes {
        write!(out, "prefix {}: <{}>\n", p.prefix, p.iri).unwrap();
    }
    if !prog.requires.is_empty() {
        out.push_str("requires [\n");
        for (i, cap) in prog.requires.iter().enumerate() {
            write!(out, "{}capability(\"{}\"", opts.indent, cap.id).unwrap();
            for arg in &cap.args {
                write!(out, ", {}: ", arg.name).unwrap();
                project_expr(&arg.value, opts, &mut out);
            }
            out.push(')');
            if i + 1 < prog.requires.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str("];\n");
    }

    for (i, item) in prog.items.iter().enumerate() {
        if !first {
            out.push_str(&blank);
        }
        // Prepend leading comments from CST if available
        if let Some(cst) = cst_items.get(i) {
            for trivia in &cst.leading {
                if trivia.is_comment() {
                    out.push_str(&trivia.text);
                    out.push('\n');
                }
            }
        }
        project_item(item, opts, &mut out);
        first = false;
    }

    out
}

// ── helpers for building AST nodes ────────────────────────────────

/// Helper: build a simple field declaration.
pub fn make_field(
    name: &str,
    ty: &str,
    unit: Option<&str>,
    support: FieldSupport,
    representation: FieldRepresentation,
) -> Item {
    Item::Field(FieldDecl {
        span: crate::span::Span::point(0),
        name: name.to_string(),
        ty: TypeExpr {
            span: crate::span::Span::point(0),
            name: ty.to_string(),
            args: Vec::new(),
        },
        unit: unit.map(|s| s.to_string()),
        support,
        representation,
    })
}

/// Helper: build a simple material declaration.
pub fn make_material(name: &str, properties: Vec<(&str, Expr)>) -> Item {
    Item::Material(MaterialDecl {
        span: crate::span::Span::point(0),
        name: name.to_string(),
        properties: properties
            .into_iter()
            .map(|(n, v)| NamedArg {
                span: crate::span::Span::point(0),
                name: n.to_string(),
                value: v,
            })
            .collect(),
    })
}

/// Helper: build a simple law declaration.
pub fn make_law(name: &str, condition: Expr, consequence: Expr) -> Item {
    Item::Law(LawDecl {
        span: crate::span::Span::point(0),
        name: name.to_string(),
        condition,
        consequence,
    })
}

/// Helper: build an integer literal expression.
pub fn make_int(n: i64) -> Expr {
    Expr {
        span: crate::span::Span::point(0),
        kind: ExprKind::Literal(Literal::Int(n)),
    }
}

/// Helper: build a float literal expression.
pub fn make_float(f: f64) -> Expr {
    Expr {
        span: crate::span::Span::point(0),
        kind: ExprKind::Literal(Literal::Float(f.to_bits())),
    }
}

/// Helper: build a string literal expression.
pub fn make_string(s: &str) -> Expr {
    Expr {
        span: crate::span::Span::point(0),
        kind: ExprKind::Literal(Literal::String(s.to_string())),
    }
}

/// Helper: build an identifier expression.
pub fn make_ident(s: &str) -> Expr {
    Expr {
        span: crate::span::Span::point(0),
        kind: ExprKind::Ident(s.to_string()),
    }
}

/// Helper: build a binary expression.
pub fn make_binary(op: BinOp, left: Expr, right: Expr) -> Expr {
    Expr {
        span: crate::span::Span::point(0),
        kind: ExprKind::Binary {
            op,
            left: Box::new(left),
            right: Box::new(right),
        },
    }
}

/// Helper: build a call expression.
pub fn make_call(callee: &str, args: Vec<Expr>) -> Expr {
    Expr {
        span: crate::span::Span::point(0),
        kind: ExprKind::Call {
            callee: Box::new(make_ident(callee)),
            args: args.into_iter().map(Arg::Pos).collect(),
        },
    }
}

/// Helper: build a member access expression.
pub fn make_member(recv: &str, name: &str) -> Expr {
    Expr {
        span: crate::span::Span::point(0),
        kind: ExprKind::Member {
            recv: Box::new(make_ident(recv)),
            name: name.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{EnumVariant, Param};
    use crate::parse::parse_program;
    use crate::trivia::Trivia;

    // ── projection tests ───────────────────────────────────────────

    #[test]
    fn project_empty_program() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: Vec::new(),
        };
        let out = project_program(&prog, &ProjectOptions::default());
        assert!(out.is_empty());
    }

    #[test]
    fn project_field_basic() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_field(
                "pressure",
                "Pressure",
                Some("qudt:KiloPascal"),
                FieldSupport::Region,
                FieldRepresentation::Grid,
            )],
        };
        let out = project_program(&prog, &ProjectOptions::default());
        assert!(out.contains("field pressure: Pressure"));
        assert!(out.contains("unit: <qudt:KiloPascal>"));
        assert!(out.ends_with(";\n"));
    }

    #[test]
    fn project_field_with_support_and_repr() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_field(
                "temp",
                "Temperature",
                Some("qudt:Kelvin"),
                FieldSupport::Point,
                FieldRepresentation::Sampled,
            )],
        };
        let out = project_program(&prog, &ProjectOptions::default());
        assert!(out.contains("field temp: Temperature"));
        assert!(out.contains("unit: <qudt:Kelvin>"));
        assert!(out.contains("support: point"));
        assert!(out.contains("representation: sampled"));
    }

    #[test]
    fn project_material_basic() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_material(
                "steel",
                vec![
                    ("yield", make_float(250.0)),
                    ("density", make_float(7850.0)),
                ],
            )],
        };
        let out = project_program(&prog, &ProjectOptions::default());
        assert!(out.contains("material steel: Material"));
        assert!(out.contains("yield: 250.0"));
        assert!(out.contains("density: 7850.0"));
    }

    #[test]
    fn project_law_basic() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_law(
                "crush",
                make_binary(
                    BinOp::Gt,
                    make_ident("pressure"),
                    make_member("steel", "yield"),
                ),
                make_int(1),
            )],
        };
        let out = project_program(&prog, &ProjectOptions::default());
        assert!(out.contains("law crush"));
        assert!(out.contains("when pressure > steel.yield"));
        assert!(out.contains("=> 1"));
    }

    #[test]
    fn project_function_basic() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![Item::Function(FunctionDecl {
                span: crate::span::Span::point(0),
                effect: Some(EffectClass::Pure),
                is_async: false,
                name: "add".to_string(),
                params: vec![
                    Param {
                        span: crate::span::Span::point(0),
                        name: "a".to_string(),
                        ty: TypeExpr {
                            span: crate::span::Span::point(0),
                            name: "i64".to_string(),
                            args: Vec::new(),
                        },
                    },
                    Param {
                        span: crate::span::Span::point(0),
                        name: "b".to_string(),
                        ty: TypeExpr {
                            span: crate::span::Span::point(0),
                            name: "i64".to_string(),
                            args: Vec::new(),
                        },
                    },
                ],
                budget: Vec::new(),
                ret: Some(TypeExpr {
                    span: crate::span::Span::point(0),
                    name: "i64".to_string(),
                    args: Vec::new(),
                }),
                body: Block {
                    span: crate::span::Span::point(0),
                    stmts: vec![Stmt::Return {
                        span: crate::span::Span::point(0),
                        value: Some(make_binary(BinOp::Add, make_ident("a"), make_ident("b"))),
                    }],
                },
            })],
        };
        let out = project_program(&prog, &ProjectOptions::default());
        assert!(out.contains("pure fn add(a: i64, b: i64) -> i64"));
        assert!(out.contains("return a + b;"));
    }

    #[test]
    fn project_const_basic() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![Item::Const(ConstDecl {
                span: crate::span::Span::point(0),
                name: "PI".to_string(),
                ty: Some(TypeExpr {
                    span: crate::span::Span::point(0),
                    name: "f64".to_string(),
                    args: Vec::new(),
                }),
                value: make_float(3.14159),
            })],
        };
        let out = project_program(&prog, &ProjectOptions::default());
        assert!(out.contains("const PI: f64 = 3.14159;"));
    }

    #[test]
    fn project_enum_basic() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![Item::Enum(EnumDecl {
                span: crate::span::Span::point(0),
                name: "Shape".to_string(),
                variants: vec![
                    EnumVariant {
                        span: crate::span::Span::point(0),
                        name: "Circle".to_string(),
                        payload: vec![TypeExpr {
                            span: crate::span::Span::point(0),
                            name: "f64".to_string(),
                            args: Vec::new(),
                        }],
                    },
                    EnumVariant {
                        span: crate::span::Span::point(0),
                        name: "Point".to_string(),
                        payload: Vec::new(),
                    },
                ],
            })],
        };
        let out = project_program(&prog, &ProjectOptions::default());
        assert!(out.contains("enum Shape {"));
        assert!(out.contains("Circle(f64),"));
        assert!(out.contains("Point,"));
    }

    #[test]
    fn project_prefix_and_requires() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: vec![crate::ast::PrefixDecl {
                span: crate::span::Span::point(0),
                prefix: "qudt".to_string(),
                iri: "http://qudt.org/vocab/unit".to_string(),
            }],
            locales: Vec::new(),
            requires: vec![crate::ast::CapSpec {
                span: crate::span::Span::point(0),
                id: "graph.read".to_string(),
                args: Vec::new(),
            }],
            items: Vec::new(),
        };
        let out = project_program(&prog, &ProjectOptions::default());
        assert!(out.contains("prefix qudt: <http://qudt.org/vocab/unit>"));
        assert!(out.contains("requires ["));
        assert!(out.contains("capability(\"graph.read\")"));
    }

    // ── round-trip tests ───────────────────────────────────────────

    #[test]
    fn roundtrip_field() {
        let src = "field pressure: Pressure\n  unit: <qudt:KiloPascal>\n  support: point\n  representation: grid;\n";
        let prog = parse_program(src).expect("parse");
        let projected = project_program(&prog, &ProjectOptions::default());
        let reprog = parse_program(&projected).expect("re-parse");
        assert_eq!(prog.items.len(), reprog.items.len());
        // Both should be fields with the same name
        match (&prog.items[0], &reprog.items[0]) {
            (Item::Field(a), Item::Field(b)) => {
                assert_eq!(a.name, b.name);
                assert_eq!(a.unit, b.unit);
                assert_eq!(a.support, b.support);
                assert_eq!(a.representation, b.representation);
            }
            _ => panic!("expected fields"),
        }
    }

    #[test]
    fn roundtrip_material() {
        let src = "material steel: Material\n  yield: 250.0,\n  density: 7850.0;\n";
        let prog = parse_program(src).expect("parse");
        let projected = project_program(&prog, &ProjectOptions::default());
        let reprog = parse_program(&projected).expect("re-parse");
        assert_eq!(prog.items.len(), reprog.items.len());
        match (&prog.items[0], &reprog.items[0]) {
            (Item::Material(a), Item::Material(b)) => {
                assert_eq!(a.name, b.name);
                assert_eq!(a.properties.len(), b.properties.len());
            }
            _ => panic!("expected materials"),
        }
    }

    #[test]
    fn roundtrip_law() {
        let src = "law crush\n  when pressure > steel.yield\n  => 1;\n";
        let prog = parse_program(src).expect("parse");
        let projected = project_program(&prog, &ProjectOptions::default());
        let reprog = parse_program(&projected).expect("re-parse");
        assert_eq!(prog.items.len(), reprog.items.len());
        match (&prog.items[0], &reprog.items[0]) {
            (Item::Law(a), Item::Law(b)) => {
                assert_eq!(a.name, b.name);
            }
            _ => panic!("expected laws"),
        }
    }

    #[test]
    fn roundtrip_function() {
        let src = "pure fn add(a: i64, b: i64) -> i64 {\n  return a + b;\n}\n";
        let prog = parse_program(src).expect("parse");
        let projected = project_program(&prog, &ProjectOptions::default());
        let reprog = parse_program(&projected).expect("re-parse");
        assert_eq!(prog.items.len(), reprog.items.len());
        match (&prog.items[0], &reprog.items[0]) {
            (Item::Function(a), Item::Function(b)) => {
                assert_eq!(a.name, b.name);
                assert_eq!(a.params.len(), b.params.len());
                assert_eq!(a.effect, b.effect);
            }
            _ => panic!("expected functions"),
        }
    }

    #[test]
    fn roundtrip_const() {
        let src = "const PI: f64 = 3.14159;\n";
        let prog = parse_program(src).expect("parse");
        let projected = project_program(&prog, &ProjectOptions::default());
        let reprog = parse_program(&projected).expect("re-parse");
        assert_eq!(prog.items.len(), reprog.items.len());
        match (&prog.items[0], &reprog.items[0]) {
            (Item::Const(a), Item::Const(b)) => {
                assert_eq!(a.name, b.name);
            }
            _ => panic!("expected consts"),
        }
    }

    #[test]
    fn roundtrip_enum() {
        let src = "enum Shape {\n  Circle(f64),\n  Point,\n}\n";
        let prog = parse_program(src).expect("parse");
        let projected = project_program(&prog, &ProjectOptions::default());
        let reprog = parse_program(&projected).expect("re-parse");
        assert_eq!(prog.items.len(), reprog.items.len());
        match (&prog.items[0], &reprog.items[0]) {
            (Item::Enum(a), Item::Enum(b)) => {
                assert_eq!(a.name, b.name);
                assert_eq!(a.variants.len(), b.variants.len());
                assert_eq!(a.variants[0].name, b.variants[0].name);
            }
            _ => panic!("expected enums"),
        }
    }

    // ── structural edit tests ──────────────────────────────────────

    #[test]
    fn edit_add_item() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_field(
                "a",
                "A",
                None,
                FieldSupport::Region,
                FieldRepresentation::Grid,
            )],
        };
        let new_item = make_field(
            "b",
            "B",
            None,
            FieldSupport::Region,
            FieldRepresentation::Grid,
        );
        let edited = apply_edit(
            &prog,
            &Edit::AddItem {
                item: new_item,
                index: None,
            },
        );
        assert_eq!(edited.items.len(), 2);
        match &edited.items[1] {
            Item::Field(fd) => assert_eq!(fd.name, "b"),
            _ => panic!("expected field"),
        }
    }

    #[test]
    fn edit_remove_item() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![
                make_field(
                    "a",
                    "A",
                    None,
                    FieldSupport::Region,
                    FieldRepresentation::Grid,
                ),
                make_field(
                    "b",
                    "B",
                    None,
                    FieldSupport::Region,
                    FieldRepresentation::Grid,
                ),
            ],
        };
        let edited = apply_edit(&prog, &Edit::RemoveItem { index: 0 });
        assert_eq!(edited.items.len(), 1);
        match &edited.items[0] {
            Item::Field(fd) => assert_eq!(fd.name, "b"),
            _ => panic!("expected field"),
        }
    }

    #[test]
    fn edit_rename_item() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_field(
                "old_name",
                "A",
                None,
                FieldSupport::Region,
                FieldRepresentation::Grid,
            )],
        };
        let edited = apply_edit(
            &prog,
            &Edit::RenameItem {
                index: 0,
                new_name: "new_name".into(),
            },
        );
        match &edited.items[0] {
            Item::Field(fd) => assert_eq!(fd.name, "new_name"),
            _ => panic!("expected field"),
        }
    }

    #[test]
    fn edit_set_field_unit() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_field(
                "temp",
                "Temperature",
                None,
                FieldSupport::Region,
                FieldRepresentation::Grid,
            )],
        };
        let edited = apply_edit(
            &prog,
            &Edit::SetFieldUnit {
                index: 0,
                unit: Some("qudt:Kelvin".into()),
            },
        );
        match &edited.items[0] {
            Item::Field(fd) => assert_eq!(fd.unit.as_deref(), Some("qudt:Kelvin")),
            _ => panic!("expected field"),
        }
    }

    #[test]
    fn edit_set_field_support() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_field(
                "temp",
                "Temperature",
                None,
                FieldSupport::Region,
                FieldRepresentation::Grid,
            )],
        };
        let edited = apply_edit(
            &prog,
            &Edit::SetFieldSupport {
                index: 0,
                support: FieldSupport::Stream,
            },
        );
        match &edited.items[0] {
            Item::Field(fd) => assert_eq!(fd.support, FieldSupport::Stream),
            _ => panic!("expected field"),
        }
    }

    #[test]
    fn edit_add_material_property() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_material("steel", vec![("yield", make_float(250.0))])],
        };
        let edited = apply_edit(
            &prog,
            &Edit::AddMaterialProperty {
                index: 0,
                property: NamedArg {
                    span: crate::span::Span::point(0),
                    name: "density".into(),
                    value: make_float(7850.0),
                },
            },
        );
        match &edited.items[0] {
            Item::Material(md) => {
                assert_eq!(md.properties.len(), 2);
                assert_eq!(md.properties[1].name, "density");
            }
            _ => panic!("expected material"),
        }
    }

    #[test]
    fn edit_remove_material_property() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_material(
                "steel",
                vec![
                    ("yield", make_float(250.0)),
                    ("density", make_float(7850.0)),
                ],
            )],
        };
        let edited = apply_edit(
            &prog,
            &Edit::RemoveMaterialProperty {
                index: 0,
                name: "yield".into(),
            },
        );
        match &edited.items[0] {
            Item::Material(md) => {
                assert_eq!(md.properties.len(), 1);
                assert_eq!(md.properties[0].name, "density");
            }
            _ => panic!("expected material"),
        }
    }

    #[test]
    fn edit_add_prefix() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: Vec::new(),
        };
        let edited = apply_edit(
            &prog,
            &Edit::AddPrefix {
                prefix: "qudt".into(),
                iri: "http://qudt.org/".into(),
            },
        );
        assert_eq!(edited.prefixes.len(), 1);
        assert_eq!(edited.prefixes[0].prefix, "qudt");
    }

    #[test]
    fn edit_remove_prefix() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: vec![
                crate::ast::PrefixDecl {
                    span: crate::span::Span::point(0),
                    prefix: "qudt".into(),
                    iri: "http://qudt.org/".into(),
                },
                crate::ast::PrefixDecl {
                    span: crate::span::Span::point(0),
                    prefix: "ex".into(),
                    iri: "http://example.org/".into(),
                },
            ],
            locales: Vec::new(),
            requires: Vec::new(),
            items: Vec::new(),
        };
        let edited = apply_edit(
            &prog,
            &Edit::RemovePrefix {
                prefix: "qudt".into(),
            },
        );
        assert_eq!(edited.prefixes.len(), 1);
        assert_eq!(edited.prefixes[0].prefix, "ex");
    }

    #[test]
    fn edit_apply_sequence() {
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: Vec::new(),
        };
        let edits = vec![
            Edit::AddPrefix {
                prefix: "qudt".into(),
                iri: "http://qudt.org/".into(),
            },
            Edit::AddItem {
                item: make_field(
                    "temp",
                    "Temperature",
                    Some("qudt:Kelvin"),
                    FieldSupport::Point,
                    FieldRepresentation::Sampled,
                ),
                index: None,
            },
            Edit::AddItem {
                item: make_material("steel", vec![("yield", make_float(250.0))]),
                index: None,
            },
        ];
        let edited = apply_edits(&prog, &edits);
        assert_eq!(edited.prefixes.len(), 1);
        assert_eq!(edited.items.len(), 2);
    }

    // ── edit + project integration ─────────────────────────────────

    #[test]
    fn edit_then_project_produces_valid_source() {
        let src = "field pressure: Pressure\n  unit: <qudt:KiloPascal>;\n";
        let prog = parse_program(src).expect("parse");
        let edited = apply_edit(
            &prog,
            &Edit::SetFieldSupport {
                index: 0,
                support: FieldSupport::Stream,
            },
        );
        let projected = project_program(&edited, &ProjectOptions::default());
        // Should be re-parseable
        let reprog = parse_program(&projected).expect("re-parse");
        match &reprog.items[0] {
            Item::Field(fd) => {
                assert_eq!(fd.support, FieldSupport::Stream);
                assert_eq!(fd.unit.as_deref(), Some("qudt:KiloPascal"));
            }
            _ => panic!("expected field"),
        }
    }

    #[test]
    fn edit_add_field_then_project() {
        let src = "material steel: Material\n  yield: 250.0;\n";
        let prog = parse_program(src).expect("parse");
        let new_field = make_field(
            "temp",
            "Temperature",
            Some("qudt:Kelvin"),
            FieldSupport::Point,
            FieldRepresentation::Grid,
        );
        let edited = apply_edit(
            &prog,
            &Edit::AddItem {
                item: new_field,
                index: Some(0),
            },
        );
        let projected = project_program(&edited, &ProjectOptions::default());
        let reprog = parse_program(&projected).expect("re-parse");
        assert_eq!(reprog.items.len(), 2);
        // Field should be first (inserted at index 0)
        match &reprog.items[0] {
            Item::Field(fd) => assert_eq!(fd.name, "temp"),
            _ => panic!("expected field at index 0"),
        }
        match &reprog.items[1] {
            Item::Material(md) => assert_eq!(md.name, "steel"),
            _ => panic!("expected material at index 1"),
        }
    }

    // ── expression projection tests ────────────────────────────────

    #[test]
    fn project_expr_list() {
        let e = Expr {
            span: crate::span::Span::point(0),
            kind: ExprKind::List(vec![make_int(1), make_int(2), make_int(3)]),
        };
        let mut out = String::new();
        project_expr(&e, &ProjectOptions::default(), &mut out);
        assert_eq!(out, "[1, 2, 3]");
    }

    #[test]
    fn project_expr_record() {
        let e = Expr {
            span: crate::span::Span::point(0),
            kind: ExprKind::Record(vec![
                NamedArg {
                    span: crate::span::Span::point(0),
                    name: "x".into(),
                    value: make_int(1),
                },
                NamedArg {
                    span: crate::span::Span::point(0),
                    name: "y".into(),
                    value: make_int(2),
                },
            ]),
        };
        let mut out = String::new();
        project_expr(&e, &ProjectOptions::default(), &mut out);
        assert_eq!(out, "{ x: 1, y: 2 }");
    }

    #[test]
    fn project_expr_binary() {
        let e = make_binary(BinOp::Add, make_int(1), make_int(2));
        let mut out = String::new();
        project_expr(&e, &ProjectOptions::default(), &mut out);
        assert_eq!(out, "1 + 2");
    }

    #[test]
    fn project_expr_call() {
        let e = make_call("math.max", vec![make_int(0), make_int(100)]);
        let mut out = String::new();
        project_expr(&e, &ProjectOptions::default(), &mut out);
        assert_eq!(out, "math.max(0, 100)");
    }

    #[test]
    fn project_expr_member() {
        let e = make_member("steel", "yield");
        let mut out = String::new();
        project_expr(&e, &ProjectOptions::default(), &mut out);
        assert_eq!(out, "steel.yield");
    }

    #[test]
    fn project_expr_string() {
        let e = make_string("hello");
        let mut out = String::new();
        project_expr(&e, &ProjectOptions::default(), &mut out);
        assert_eq!(out, "\"hello\"");
    }

    #[test]
    fn project_expr_iri() {
        let e = Expr {
            span: crate::span::Span::point(0),
            kind: ExprKind::Iri("https://example.org/foo".to_string()),
        };
        let mut out = String::new();
        project_expr(&e, &ProjectOptions::default(), &mut out);
        assert_eq!(out, "<https://example.org/foo>");
    }

    // ── trivia-aware projection ────────────────────────────────────

    #[test]
    fn project_with_trivia_preserves_comments() {
        let cst_items = vec![CstNode::new(
            make_field(
                "temp",
                "Temperature",
                Some("qudt:Kelvin"),
                FieldSupport::Region,
                FieldRepresentation::Grid,
            ),
            vec![Trivia::line_comment(
                "// This is a field",
                crate::span::Span::point(0),
            )],
            vec![],
        )];
        let prog = Program {
            span: crate::span::Span::point(0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            locales: Vec::new(),
            requires: Vec::new(),
            items: vec![make_field(
                "temp",
                "Temperature",
                Some("qudt:Kelvin"),
                FieldSupport::Region,
                FieldRepresentation::Grid,
            )],
        };
        let out = project_with_trivia(&prog, &cst_items, &ProjectOptions::default());
        assert!(out.contains("// This is a field"));
        assert!(out.contains("field temp: Temperature"));
    }
}
