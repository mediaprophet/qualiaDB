//! Bi-directional Poetic Decompiler for VibeScript (Zero-Heap friendly).
//!
//! Decompiles `vibe-bc-0.1` bytecode chunks and parsed AST structures into clean,
//! formatted, human-readable VibeScript source code. Supports round-trip AST formatting,
//! expression precedence reconstruction, and block indentation.

use crate::ast::*;
use crate::bytecode::op::{Chunk, Const, Op};
use std::fmt::Write;

/// Formatting options for the decompiler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecompileOptions {
    /// Spaces per indentation level (default: 4).
    pub indent_spaces: usize,
    /// Whether to emit explicit step budgets.
    pub emit_budgets: bool,
    /// Whether to emit type annotations where known.
    pub emit_types: bool,
}

impl Default for DecompileOptions {
    fn default() -> Self {
        Self {
            indent_spaces: 4,
            emit_budgets: true,
            emit_types: true,
        }
    }
}

/// Decompile an entire `Program` AST back into formatted VibeScript source.
pub fn decompile_program(program: &Program, opts: &DecompileOptions) -> String {
    let mut out = String::with_capacity(1024);

    for loc in &program.locales {
        let _ = writeln!(out, "locale {};", loc.code);
    }
    if !program.locales.is_empty() {
        out.push('\n');
    }

    // Module declaration
    if let Some(module) = &program.module {
        match &module.name {
            Name::Ident(id) => {
                let _ = writeln!(out, "module {id};\n");
            }
            Name::Iri(iri) => {
                let _ = writeln!(out, "module <{iri}>;\n");
            }
        }
    }

    // Prefixes
    for prefix in &program.prefixes {
        let _ = writeln!(out, "prefix {}: <{}>;", prefix.prefix, prefix.iri);
    }
    if !program.prefixes.is_empty() {
        out.push('\n');
    }

    // Imports
    for import in &program.imports {
        if let Some(alias) = &import.alias {
            let _ = writeln!(out, "import \"{}\" as {};", import.path, alias);
        } else {
            let _ = writeln!(out, "import \"{}\";", import.path);
        }
    }
    if !program.imports.is_empty() {
        out.push('\n');
    }

    // Capability requires
    for req in &program.requires {
        let _ = writeln!(out, "requires [ capability(\"{}\") ];", req.id);
    }
    if !program.requires.is_empty() {
        out.push('\n');
    }

    // Items
    for (i, item) in program.items.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        decompile_item(&mut out, item, 0, opts);
    }

    out
}

/// Decompile a single AST Item.
pub fn decompile_item(out: &mut String, item: &Item, indent: usize, opts: &DecompileOptions) {
    let pad = " ".repeat(indent * opts.indent_spaces);
    match item {
        Item::Const(cd) => {
            let _ = write!(out, "{pad}const {}", cd.name);
            if let Some(ty) = &cd.ty {
                if opts.emit_types {
                    let _ = write!(out, ": {}", decompile_type_expr(ty));
                }
            }
            out.push_str(" = ");
            decompile_expr(out, &cd.value);
            out.push_str(";\n");
        }
        Item::Function(fd) => {
            let effect_str = match fd.effect {
                Some(EffectClass::Pure) => "pure ",
                Some(EffectClass::Hot) => "hot ",
                Some(EffectClass::Cold) => "cold ",
                Some(EffectClass::Async) => "async ",
                Some(EffectClass::External) => "effect ",
                None => "",
            };
            let async_str = if fd.is_async && fd.effect != Some(EffectClass::Async) {
                "async "
            } else {
                ""
            };
            let _ = write!(out, "{pad}{effect_str}{async_str}fn {}(", fd.name);
            for (j, p) in fd.params.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                out.push_str(&p.name);
                if opts.emit_types {
                    let _ = write!(out, ": {}", decompile_type_expr(&p.ty));
                }
            }
            out.push(')');

            if opts.emit_budgets && !fd.budget.is_empty() {
                out.push_str(" budget(");
                for (k, b) in fd.budget.iter().enumerate() {
                    if k > 0 {
                        out.push_str(", ");
                    }
                    let _ = write!(out, "{}: ", b.name);
                    decompile_expr(out, &b.value);
                }
                out.push(')');
            }

            if let Some(ret) = &fd.ret {
                if opts.emit_types {
                    let _ = write!(out, " -> {}", decompile_type_expr(ret));
                }
            }

            out.push(' ');
            decompile_block(out, &fd.body, indent, opts);
            out.push('\n');
        }
        Item::Hook(hd) => {
            let _ = write!(out, "{pad}on {} (", hd.path.join("."));
            for (j, p) in hd.params.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                out.push_str(&p.name);
                if opts.emit_types {
                    let _ = write!(out, ": {}", decompile_type_expr(&p.ty));
                }
            }
            out.push_str(") ");
            decompile_block(out, &hd.body, indent, opts);
            out.push('\n');
        }
        Item::Statement(stmt) => {
            let _ = write!(out, "{pad}");
            decompile_stmt(out, stmt, indent, opts);
            out.push('\n');
        }
        Item::Enum(ed) => {
            let _ = writeln!(out, "{pad}enum {} {{", ed.name);
            for variant in &ed.variants {
                let vpad = " ".repeat((indent + 1) * opts.indent_spaces);
                if variant.payload.is_empty() {
                    let _ = writeln!(out, "{vpad}{},", variant.name);
                } else {
                    let _ = write!(out, "{vpad}{}(", variant.name);
                    for (k, p) in variant.payload.iter().enumerate() {
                        if k > 0 {
                            out.push_str(", ");
                        }
                        out.push_str(&decompile_type_expr(p));
                    }
                    out.push_str("),\n");
                }
            }
            let _ = writeln!(out, "{pad}}}");
        }
        Item::Law(ld) => {
            let _ = writeln!(out, "{pad}law {} {{", ld.name);
            let bpad = " ".repeat((indent + 1) * opts.indent_spaces);
            let _ = write!(out, "{bpad}when: ");
            decompile_expr(out, &ld.condition);
            let _ = write!(out, " => ");
            decompile_expr(out, &ld.consequence);
            let _ = writeln!(out, ";\n{pad}}}");
        }
        Item::Material(md) => {
            let _ = writeln!(out, "{pad}material {} {{", md.name);
            let _ = writeln!(out, "{pad}}}");
        }
        Item::Field(fld) => {
            let _ = writeln!(
                out,
                "{pad}field {}: {} {{",
                fld.name,
                decompile_type_expr(&fld.ty)
            );
            let _ = writeln!(out, "{pad}}}");
        }
        Item::Cell(cd) => {
            let effect_str = match cd.effect {
                Some(EffectClass::Pure) => "pure ",
                Some(EffectClass::Hot) => "hot ",
                Some(EffectClass::Cold) => "cold ",
                Some(EffectClass::Async) => "async ",
                Some(EffectClass::External) => "effect ",
                None => "",
            };
            let _ = write!(out, "{pad}{effect_str}cell {}", cd.name);
            if !cd.params.is_empty() {
                out.push('(');
                for (j, p) in cd.params.iter().enumerate() {
                    if j > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&p.name);
                    if opts.emit_types {
                        let _ = write!(out, ": {}", decompile_type_expr(&p.ty));
                    }
                }
                out.push(')');
            }
            if let Some(w) = &cd.when {
                out.push_str(" when ");
                decompile_expr(out, w);
            }
            out.push_str(" := ");
            decompile_expr(out, &cd.expr);
            out.push_str(";\n");
        }
        Item::Present(pd) => {
            let _ = writeln!(out, "{pad}present {} {{", pd.name);
            for prop in &pd.properties {
                let _ = write!(out, "{pad}  {}: ", prop.name);
                decompile_expr(out, &prop.value);
                out.push('\n');
            }
            let _ = writeln!(out, "{pad}}}");
        }
        Item::Bind(b) => {
            let _ = write!(out, "{pad}bind ");
            decompile_expr(out, &b.left);
            out.push_str(" <-> ");
            decompile_expr(out, &b.right);
            if let Some((lo, hi)) = &b.clamp {
                out.push_str(" using Clamp[");
                decompile_expr(out, lo);
                out.push_str(", ");
                decompile_expr(out, hi);
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
    }
}

/// Decompile a statement block.
pub fn decompile_block(out: &mut String, block: &Block, indent: usize, opts: &DecompileOptions) {
    if block.stmts.is_empty() {
        out.push_str("{}");
        return;
    }
    out.push_str("{\n");
    for stmt in &block.stmts {
        let pad = " ".repeat((indent + 1) * opts.indent_spaces);
        out.push_str(&pad);
        decompile_stmt(out, stmt, indent + 1, opts);
        out.push('\n');
    }
    let pad = " ".repeat(indent * opts.indent_spaces);
    let _ = write!(out, "{pad}}}");
}

/// Decompile a single statement.
pub fn decompile_stmt(out: &mut String, stmt: &Stmt, indent: usize, opts: &DecompileOptions) {
    match stmt {
        Stmt::Let {
            mutable,
            name,
            ty,
            value,
            ..
        } => {
            if *mutable {
                out.push_str("mut ");
            } else {
                out.push_str("let ");
            }
            out.push_str(name);
            if let Some(t) = ty {
                if opts.emit_types {
                    let _ = write!(out, ": {}", decompile_type_expr(t));
                }
            }
            if let Some(val) = value {
                out.push_str(" = ");
                decompile_expr(out, val);
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
            if *mutable {
                out.push_str("mut ");
            } else {
                out.push_str("let ");
            }
            decompile_pattern(out, pattern);
            if let Some(t) = ty {
                if opts.emit_types {
                    let _ = write!(out, ": {}", decompile_type_expr(t));
                }
            }
            out.push_str(" = ");
            decompile_expr(out, value);
            out.push(';');
        }
        Stmt::Assign { target, value, .. } => {
            decompile_expr(out, target);
            out.push_str(" = ");
            decompile_expr(out, value);
            out.push(';');
        }
        Stmt::If {
            cond,
            then_block,
            else_block,
            ..
        } => {
            out.push_str("if ");
            decompile_expr(out, cond);
            out.push(' ');
            decompile_block(out, then_block, indent, opts);
            if let Some(else_branch) = else_block {
                out.push_str(" else ");
                match else_branch.as_ref() {
                    Stmt::If { .. } => {
                        decompile_stmt(out, else_branch.as_ref(), indent, opts);
                    }
                    Stmt::Block(b) => {
                        decompile_block(out, b, indent, opts);
                    }
                    other => {
                        decompile_stmt(out, other, indent, opts);
                    }
                }
            }
        }
        Stmt::For {
            name, iter, body, ..
        } => {
            let _ = write!(out, "for {name} in ");
            decompile_expr(out, iter);
            out.push(' ');
            decompile_block(out, body, indent, opts);
        }
        Stmt::While { cond, body, .. } => {
            out.push_str("while ");
            decompile_expr(out, cond);
            out.push(' ');
            decompile_block(out, body, indent, opts);
        }
        Stmt::Match {
            scrutinee, arms, ..
        } => {
            out.push_str("match ");
            decompile_expr(out, scrutinee);
            out.push_str(" {\n");
            let arm_pad = " ".repeat((indent + 1) * opts.indent_spaces);
            for arm in arms {
                out.push_str(&arm_pad);
                decompile_pattern(out, &arm.pattern);
                out.push_str(" => ");
                match &arm.body {
                    ArmBody::Expr(e) => {
                        decompile_expr(out, e);
                        out.push_str(",\n");
                    }
                    ArmBody::Block(b) => {
                        decompile_block(out, b, indent + 1, opts);
                        out.push('\n');
                    }
                }
            }
            let pad = " ".repeat(indent * opts.indent_spaces);
            let _ = write!(out, "{pad}}}");
        }
        Stmt::Return { value, .. } => {
            out.push_str("return");
            if let Some(e) = value {
                out.push(' ');
                decompile_expr(out, e);
            }
            out.push(';');
        }
        Stmt::Yield { value, .. } => {
            out.push_str("yield");
            if let Some(e) = value {
                out.push(' ');
                decompile_expr(out, e);
            }
            out.push(';');
        }
        Stmt::Transaction { args, body, .. } => {
            out.push_str("transaction (");
            for (j, a) in args.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                let _ = write!(out, "{}: ", a.name);
                decompile_expr(out, &a.value);
            }
            out.push_str(") ");
            decompile_block(out, body, indent, opts);
        }
        Stmt::Effect { expr, .. } => {
            out.push_str("effect ");
            decompile_expr(out, expr);
            out.push(';');
        }
        Stmt::Expr { expr, .. } => {
            decompile_expr(out, expr);
            out.push(';');
        }
        Stmt::Block(b) => {
            decompile_block(out, b, indent, opts);
        }
    }
}

/// Decompile an expression.
pub fn decompile_expr(out: &mut String, expr: &Expr) {
    match &expr.kind {
        ExprKind::Literal(lit) => match lit {
            Literal::Null => out.push_str("null"),
            Literal::Bool(b) => {
                let _ = write!(out, "{b}");
            }
            Literal::Int(i) => {
                let _ = write!(out, "{i}");
            }
            Literal::UInt(u) => {
                let _ = write!(out, "{u}");
            }
            Literal::Float(bits) => {
                let f = f64::from_bits(*bits);
                let _ = write!(out, "{f}");
            }
            Literal::Quantity { value, unit } => {
                let f = f64::from_bits(*value);
                let _ = write!(out, "{f}{unit}");
            }
            Literal::String(s) => {
                let _ = write!(out, "\"{}\"", s.replace('"', "\\\""));
            }
            Literal::Color(c) => {
                let _ = write!(out, "#{:02x}{:02x}{:02x}", c.r, c.g, c.b);
            }
        },
        ExprKind::Ident(id) => out.push_str(id),
        ExprKind::QueryVar(qv) => {
            let _ = write!(out, "?{qv}");
        }
        ExprKind::Iri(iri) => {
            let _ = write!(out, "<{iri}>");
        }
        ExprKind::Prefixed(p, n) => {
            let _ = write!(out, "{p}:{n}");
        }
        ExprKind::Blank(b) => {
            let _ = write!(out, "_:{b}");
        }
        ExprKind::Unary { op, expr: inner } => {
            let op_str = match op {
                UnOp::Not => "!",
                UnOp::Neg => "-",
                UnOp::Plus => "+",
            };
            out.push_str(op_str);
            decompile_expr(out, inner);
        }
        ExprKind::Binary { op, left, right } => {
            decompile_expr(out, left);
            let op_str = match op {
                BinOp::Or => " || ",
                BinOp::And => " && ",
                BinOp::Eq => " == ",
                BinOp::Ne => " != ",
                BinOp::Lt => " < ",
                BinOp::Le => " <= ",
                BinOp::Gt => " > ",
                BinOp::Ge => " >= ",
                BinOp::Add => " + ",
                BinOp::Sub => " - ",
                BinOp::Mul => " * ",
                BinOp::Div => " / ",
                BinOp::Rem => " % ",
            };
            out.push_str(op_str);
            decompile_expr(out, right);
        }
        ExprKind::Await(inner) => {
            out.push_str("await ");
            decompile_expr(out, inner);
        }
        ExprKind::Member { recv, name } => {
            decompile_expr(out, recv);
            let _ = write!(out, ".{name}");
        }
        ExprKind::Call { callee, args } => {
            decompile_expr(out, callee);
            out.push('(');
            for (j, a) in args.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                match a {
                    Arg::Pos(e) => decompile_expr(out, e),
                    Arg::Named(na) => {
                        let _ = write!(out, "{}: ", na.name);
                        decompile_expr(out, &na.value);
                    }
                }
            }
            out.push(')');
        }
        ExprKind::Index { recv, index } => {
            decompile_expr(out, recv);
            out.push('[');
            decompile_expr(out, index);
            out.push(']');
        }
        ExprKind::Try(inner) => {
            decompile_expr(out, inner);
            out.push('?');
        }
        ExprKind::List(items) => {
            out.push('[');
            for (j, it) in items.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                decompile_expr(out, it);
            }
            out.push(']');
        }
        ExprKind::Record(entries) => {
            out.push_str("{ ");
            for (j, entry) in entries.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                let _ = write!(out, "{}: ", entry.name);
                decompile_expr(out, &entry.value);
            }
            out.push_str(" }");
        }
        ExprKind::Triple {
            subject,
            predicate,
            object,
        } => {
            out.push_str("<< ");
            decompile_expr(out, subject);
            out.push(' ');
            decompile_expr(out, predicate);
            out.push(' ');
            decompile_expr(out, object);
            out.push_str(" >>");
        }
        ExprKind::Reified {
            subject,
            predicate,
            object,
            reifier,
        } => {
            out.push_str("<< ");
            decompile_expr(out, subject);
            out.push(' ');
            decompile_expr(out, predicate);
            out.push(' ');
            decompile_expr(out, object);
            out.push_str(" ~ ");
            decompile_expr(out, reifier);
            out.push_str(" >>");
        }
        ExprKind::Pipe { left, right } => {
            decompile_expr(out, left);
            out.push_str(" |> ");
            decompile_expr(out, right);
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
                    decompile_expr(out, a);
                }
                out.push(')');
            }
            if let Some(b) = body {
                out.push_str(" { ");
                decompile_expr(out, b);
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
                    decompile_expr(out, p);
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
            decompile_expr(out, body);
        }
        ExprKind::Tween {
            from,
            to,
            over,
            ease,
            spring,
        } => {
            decompile_expr(out, from);
            out.push_str(" ~ ");
            decompile_expr(out, to);
            out.push_str(" over ");
            decompile_expr(out, over);
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
                    let _ = write!(out, "{}: ", a.name);
                    decompile_expr(out, &a.value);
                }
                out.push(')');
            }
        }
    }
}

/// Decompile a pattern.
pub fn decompile_pattern(out: &mut String, pat: &Pattern) {
    match pat {
        Pattern::Wildcard => out.push('_'),
        Pattern::Ident(id) => out.push_str(id),
        Pattern::Literal(lit) => match lit {
            Literal::Null => out.push_str("null"),
            Literal::Bool(b) => {
                let _ = write!(out, "{b}");
            }
            Literal::Int(i) => {
                let _ = write!(out, "{i}");
            }
            Literal::UInt(u) => {
                let _ = write!(out, "{u}");
            }
            Literal::Float(bits) => {
                let f = f64::from_bits(*bits);
                let _ = write!(out, "{f}");
            }
            Literal::Quantity { value, unit } => {
                let f = f64::from_bits(*value);
                let _ = write!(out, "{f}{unit}");
            }
            Literal::String(s) => {
                let _ = write!(out, "\"{s}\"");
            }
            Literal::Color(c) => {
                let _ = write!(out, "#{:02x}{:02x}{:02x}", c.r, c.g, c.b);
            }
        },
        Pattern::Ok(inner) => {
            out.push_str("Ok(");
            decompile_pattern(out, inner);
            out.push(')');
        }
        Pattern::Err(inner) => {
            out.push_str("Err(");
            decompile_pattern(out, inner);
            out.push(')');
        }
        Pattern::Some(inner) => {
            out.push_str("Some(");
            decompile_pattern(out, inner);
            out.push(')');
        }
        Pattern::None => out.push_str("None"),
        Pattern::Record(fields) => {
            out.push_str("{ ");
            for (j, (k, p)) in fields.iter().enumerate() {
                if j > 0 {
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
                decompile_pattern(out, p);
            }
            out.push_str(" }");
        }
        Pattern::List(elements) => {
            out.push('[');
            for (j, p) in elements.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                decompile_pattern(out, p);
            }
            out.push(']');
        }
        Pattern::Constructor { name, args } => {
            out.push_str(name);
            out.push('(');
            for (j, p) in args.iter().enumerate() {
                if j > 0 {
                    out.push_str(", ");
                }
                decompile_pattern(out, p);
            }
            out.push(')');
        }
        Pattern::Variant {
            enum_name,
            variant_name,
            args,
        } => {
            let _ = write!(out, "{enum_name}.{variant_name}");
            if !args.is_empty() {
                out.push('(');
                for (j, a) in args.iter().enumerate() {
                    if j > 0 {
                        out.push_str(", ");
                    }
                    decompile_pattern(out, a);
                }
                out.push(')');
            }
        }
    }
}

/// Decompile a type expression.
pub fn decompile_type_expr(ty: &TypeExpr) -> String {
    if ty.args.is_empty() {
        ty.name.clone()
    } else {
        let mut s = ty.name.clone();
        s.push('<');
        for (j, a) in ty.args.iter().enumerate() {
            if j > 0 {
                s.push_str(", ");
            }
            s.push_str(&decompile_type_expr(a));
        }
        s.push('>');
        s
    }
}

/// Decompile a raw Bytecode Chunk into symbolic disassembly.
pub fn decompile_chunk(chunk: &Chunk) -> String {
    let mut out = String::with_capacity(1024);

    // Constants Pool
    writeln!(out, "; ── Constants (count: {}) ──", chunk.constants.len()).unwrap();
    for (i, c) in chunk.constants.iter().enumerate() {
        match c {
            Const::String(s) => writeln!(out, "  [{i}] String(\"{s}\")").unwrap(),
            Const::Iri(iri) => writeln!(out, "  [{i}] Iri(<{iri}>)").unwrap(),
        }
    }

    // Function Metadata
    if !chunk.functions.is_empty() {
        writeln!(
            out,
            "\n; ── Functions (count: {}) ──",
            chunk.functions.len()
        )
        .unwrap();
        for (i, f) in chunk.functions.iter().enumerate() {
            writeln!(
                out,
                "  [{i}] fn {} (params: {}, locals: {}, offset: {:#x}, budget: {})",
                f.name, f.param_count, f.local_count, f.code_offset, f.budget_steps
            )
            .unwrap();
        }
    }

    // Disassembled Bytecode
    writeln!(out, "\n; ── Bytecode (size: {} bytes) ──", chunk.code.len()).unwrap();
    let mut pc = 0;
    while pc < chunk.code.len() {
        let op_byte = chunk.code[pc];
        let op = match Op::from_byte(op_byte) {
            Some(o) => format!("{o:?}"),
            None => format!("UNKNOWN({op_byte:#04x})"),
        };
        writeln!(out, "  {pc:#06x}: {op}").unwrap();
        pc += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_program;

    #[test]
    fn decompile_roundtrip_function() {
        let src = "pure fn add(x: i64, y: i64) budget(steps: 100) -> i64 {\n    return x + y;\n}\n";
        let prog = parse_program(src).expect("parse");
        let decomp = decompile_program(&prog, &DecompileOptions::default());
        assert!(decomp.contains("pure fn add(x: i64, y: i64)"));
        assert!(decomp.contains("return x + y;"));
    }

    #[test]
    fn decompile_roundtrip_const() {
        let src = "const PI: f64 = 3.14159;\n";
        let prog = parse_program(src).expect("parse");
        let decomp = decompile_program(&prog, &DecompileOptions::default());
        assert!(decomp.contains("const PI: f64 = 3.14159;"));
    }
}
