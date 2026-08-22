//! Encode Program to CBOR-LD.

use super::codec::CborEncoder;
use super::TAG_VIBE_AST;
use crate::ast::*;
use crate::span::Span;

pub fn encode(program: &Program) -> Vec<u8> {
    let mut enc = CborEncoder::new();
    enc.tag(TAG_VIBE_AST);
    encode_program(&mut enc, program);
    enc.finish()
}

// â”€â”€ Encode â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€

fn encode_program(enc: &mut CborEncoder, p: &Program) {
    enc.map(8);
    enc.str("type");
    enc.str("Program");
    enc.str("span");
    encode_span(enc, p.span);
    enc.str("module");
    encode_opt_module(enc, &p.module);
    enc.str("imports");
    encode_imports(enc, &p.imports);
    enc.str("prefixes");
    encode_prefixes(enc, &p.prefixes);
    enc.str("locales");
    encode_locales(enc, &p.locales);
    enc.str("requires");
    encode_caps(enc, &p.requires);
    enc.str("items");
    encode_items(enc, &p.items);
}

fn encode_locales(enc: &mut CborEncoder, locales: &[LocaleDecl]) {
    enc.array(locales.len() as u64);
    for loc in locales {
        enc.map(2);
        enc.str("code");
        enc.str(&loc.code);
        enc.str("span");
        encode_span(enc, loc.span);
    }
}

fn encode_opt_module(enc: &mut CborEncoder, m: &Option<ModuleDecl>) {
    match m {
        Some(m) => {
            enc.map(2);
            enc.str("name");
            encode_name(enc, &m.name);
            enc.str("span");
            encode_span(enc, m.span);
        }
        None => enc.null(),
    }
}

fn encode_name(enc: &mut CborEncoder, n: &Name) {
    match n {
        Name::Ident(s) => {
            enc.map(1);
            enc.str("Ident");
            enc.str(s);
        }
        Name::Iri(s) => {
            enc.map(1);
            enc.str("Iri");
            enc.str(s);
        }
    }
}

fn encode_imports(enc: &mut CborEncoder, imports: &[ImportDecl]) {
    enc.array(imports.len() as u64);
    for imp in imports {
        enc.map(3);
        enc.str("path");
        enc.str(&imp.path);
        enc.str("alias");
        match &imp.alias {
            Some(a) => enc.str(a),
            None => enc.null(),
        };
        enc.str("span");
        encode_span(enc, imp.span);
    }
}

fn encode_prefixes(enc: &mut CborEncoder, prefixes: &[PrefixDecl]) {
    enc.array(prefixes.len() as u64);
    for pre in prefixes {
        enc.map(3);
        enc.str("prefix");
        enc.str(&pre.prefix);
        enc.str("iri");
        enc.str(&pre.iri);
        enc.str("span");
        encode_span(enc, pre.span);
    }
}

fn encode_caps(enc: &mut CborEncoder, caps: &[CapSpec]) {
    enc.array(caps.len() as u64);
    for cap in caps {
        enc.map(3);
        enc.str("id");
        enc.str(&cap.id);
        enc.str("args");
        encode_named_args(enc, &cap.args);
        enc.str("span");
        encode_span(enc, cap.span);
    }
}

fn encode_named_args(enc: &mut CborEncoder, args: &[NamedArg]) {
    enc.array(args.len() as u64);
    for arg in args {
        enc.map(3);
        enc.str("name");
        enc.str(&arg.name);
        enc.str("value");
        encode_expr(enc, &arg.value);
        enc.str("span");
        encode_span(enc, arg.span);
    }
}

fn encode_items(enc: &mut CborEncoder, items: &[Item]) {
    enc.array(items.len() as u64);
    for item in items {
        encode_item(enc, item);
    }
}
fn encode_item(enc: &mut CborEncoder, item: &Item) {
    match item {
        Item::Function(f) => {
            enc.map(8);
            enc.str("type");
            enc.str("Function");
            enc.str("name");
            enc.str(&f.name);
            enc.str("effect");
            encode_effect(enc, &f.effect);
            enc.str("async");
            enc.bool(f.is_async);
            enc.str("params");
            encode_params(enc, &f.params);
            enc.str("budget");
            encode_named_args(enc, &f.budget);
            enc.str("ret");
            match &f.ret {
                Some(t) => encode_type_expr(enc, t),
                None => enc.null(),
            };
            enc.str("body");
            encode_block(enc, &f.body);
        }
        Item::Hook(h) => {
            enc.map(4);
            enc.str("type");
            enc.str("Hook");
            enc.str("path");
            encode_str_list(enc, &h.path);
            enc.str("params");
            encode_params(enc, &h.params);
            enc.str("body");
            encode_block(enc, &h.body);
        }
        Item::Const(c) => {
            enc.map(4);
            enc.str("type");
            enc.str("Const");
            enc.str("name");
            enc.str(&c.name);
            enc.str("value");
            encode_expr(enc, &c.value);
            enc.str("span");
            encode_span(enc, c.span);
        }
        Item::Statement(s) => {
            enc.map(2);
            enc.str("type");
            enc.str("Statement");
            enc.str("stmt");
            encode_stmt(enc, s);
        }
        Item::Enum(e) => {
            enc.map(3);
            enc.str("type");
            enc.str("Enum");
            enc.str("name");
            enc.str(&e.name);
            enc.str("variants");
            {
                enc.array(e.variants.len() as u64);
                for v in &e.variants {
                    enc.map(2);
                    enc.str("name");
                    enc.str(&v.name);
                    enc.str("payload");
                    {
                        enc.array(v.payload.len() as u64);
                        for t in &v.payload {
                            encode_type_expr(enc, t);
                        }
                    }
                }
            }
        }
        Item::Field(f) => {
            enc.map(6);
            enc.str("type");
            enc.str("Field");
            enc.str("name");
            enc.str(&f.name);
            enc.str("ty");
            encode_type_expr(enc, &f.ty);
            enc.str("unit");
            match &f.unit {
                Some(u) => enc.str(u),
                None => enc.null(),
            };
            enc.str("support");
            enc.str(match f.support {
                FieldSupport::Region => "region",
                FieldSupport::Point => "point",
                FieldSupport::Continuant => "continuant",
                FieldSupport::Stream => "stream",
            });
            enc.str("representation");
            enc.str(match f.representation {
                FieldRepresentation::Grid => "grid",
                FieldRepresentation::Mesh => "mesh",
                FieldRepresentation::Particles => "particles",
                FieldRepresentation::Analytic => "analytic",
                FieldRepresentation::Sampled => "sampled",
            });
        }
        Item::Material(m) => {
            enc.map(3);
            enc.str("type");
            enc.str("Material");
            enc.str("name");
            enc.str(&m.name);
            enc.str("properties");
            encode_named_args(enc, &m.properties);
        }
        Item::Law(l) => {
            enc.map(4);
            enc.str("type");
            enc.str("Law");
            enc.str("name");
            enc.str(&l.name);
            enc.str("condition");
            encode_expr(enc, &l.condition);
            enc.str("consequence");
            encode_expr(enc, &l.consequence);
        }
        Item::Cell(c) => {
            enc.map(7);
            enc.str("type");
            enc.str("Cell");
            enc.str("span");
            encode_span(enc, c.span);
            enc.str("effect");
            encode_effect(enc, &c.effect);
            enc.str("name");
            enc.str(&c.name);
            enc.str("params");
            encode_params(enc, &c.params);
            enc.str("expr");
            encode_expr(enc, &c.expr);
            enc.str("when");
            match &c.when {
                Some(w) => encode_expr(enc, w),
                None => enc.null(),
            };
        }
        Item::Present(p) => {
            enc.map(3);
            enc.str("type");
            enc.str("Present");
            enc.str("name");
            enc.str(&p.name);
            enc.str("properties");
            encode_named_args(enc, &p.properties);
        }
        Item::Bind(b) => {
            enc.map(7);
            enc.str("type");
            enc.str("Bind");
            enc.str("left");
            encode_expr(enc, &b.left);
            enc.str("right");
            encode_expr(enc, &b.right);
            enc.str("resolve");
            enc.str(match b.resolve {
                crate::ast::BindResolve::Latest => "latest",
                crate::ast::BindResolve::Left => "left",
                crate::ast::BindResolve::Right => "right",
            });
            enc.str("clamp_lo");
            match &b.clamp {
                Some((lo, _)) => encode_expr(enc, lo),
                None => enc.null(),
            };
            enc.str("clamp_hi");
            match &b.clamp {
                Some((_, hi)) => encode_expr(enc, hi),
                None => enc.null(),
            };
        }
    }
}

fn encode_effect(enc: &mut CborEncoder, e: &Option<EffectClass>) {
    match e {
        Some(ec) => {
            let s = match ec {
                EffectClass::Pure => "Pure",
                EffectClass::Hot => "Hot",
                EffectClass::Cold => "Cold",
                EffectClass::Async => "Async",
                EffectClass::External => "External",
            };
            enc.str(s);
        }
        None => enc.null(),
    }
}

fn encode_params(enc: &mut CborEncoder, params: &[Param]) {
    enc.array(params.len() as u64);
    for p in params {
        enc.map(3);
        enc.str("name");
        enc.str(&p.name);
        enc.str("ty");
        encode_type_expr(enc, &p.ty);
        enc.str("span");
        encode_span(enc, p.span);
    }
}

fn encode_type_expr(enc: &mut CborEncoder, t: &TypeExpr) {
    enc.map(3);
    enc.str("name");
    enc.str(&t.name);
    enc.str("args");
    {
        enc.array(t.args.len() as u64);
        for a in &t.args {
            encode_type_expr(enc, a);
        }
    };
    enc.str("span");
    encode_span(enc, t.span);
}

fn encode_block(enc: &mut CborEncoder, b: &Block) {
    enc.map(2);
    enc.str("stmts");
    {
        enc.array(b.stmts.len() as u64);
        for s in &b.stmts {
            encode_stmt(enc, s);
        }
    };
    enc.str("span");
    encode_span(enc, b.span);
}

fn encode_stmt(enc: &mut CborEncoder, s: &Stmt) {
    match s {
        Stmt::Let {
            span,
            mutable,
            name,
            ty,
            value,
        } => {
            enc.map(5);
            enc.str("type");
            enc.str("Let");
            enc.str("name");
            enc.str(name);
            enc.str("mutable");
            enc.bool(*mutable);
            enc.str("ty");
            match ty {
                Some(t) => encode_type_expr(enc, t),
                None => enc.null(),
            };
            enc.str("value");
            match value {
                Some(v) => encode_expr(enc, v),
                None => enc.null(),
            };
            let _ = span;
        }
        Stmt::LetPat {
            span,
            mutable,
            pattern,
            ty,
            value,
        } => {
            enc.map(5);
            enc.str("type");
            enc.str("LetPat");
            enc.str("mutable");
            enc.bool(*mutable);
            enc.str("pattern");
            encode_pattern(enc, pattern);
            enc.str("ty");
            match ty {
                Some(t) => encode_type_expr(enc, t),
                None => enc.null(),
            };
            enc.str("value");
            encode_expr(enc, value);
            let _ = span;
        }
        Stmt::Assign {
            span,
            target,
            value,
        } => {
            enc.map(3);
            enc.str("type");
            enc.str("Assign");
            enc.str("target");
            encode_expr(enc, target);
            enc.str("value");
            encode_expr(enc, value);
            let _ = span;
        }
        Stmt::If {
            span,
            cond,
            then_block,
            else_block,
        } => {
            enc.map(4);
            enc.str("type");
            enc.str("If");
            enc.str("cond");
            encode_expr(enc, cond);
            enc.str("then");
            encode_block(enc, then_block);
            enc.str("else");
            match else_block {
                Some(b) => encode_stmt(enc, b),
                None => enc.null(),
            };
            let _ = span;
        }
        Stmt::For {
            span,
            name,
            iter,
            body,
        } => {
            enc.map(4);
            enc.str("type");
            enc.str("For");
            enc.str("name");
            enc.str(name);
            enc.str("iter");
            encode_expr(enc, iter);
            enc.str("body");
            encode_block(enc, body);
            let _ = span;
        }
        Stmt::While { span, cond, body } => {
            enc.map(3);
            enc.str("type");
            enc.str("While");
            enc.str("cond");
            encode_expr(enc, cond);
            enc.str("body");
            encode_block(enc, body);
            let _ = span;
        }
        Stmt::Match {
            span,
            scrutinee,
            arms,
        } => {
            enc.map(3);
            enc.str("type");
            enc.str("Match");
            enc.str("scrutinee");
            encode_expr(enc, scrutinee);
            enc.str("arms");
            {
                enc.array(arms.len() as u64);
                for arm in arms {
                    enc.map(3);
                    enc.str("pattern");
                    encode_pattern(enc, &arm.pattern);
                    enc.str("body");
                    encode_arm_body(enc, &arm.body);
                    enc.str("span");
                    encode_span(enc, arm.span);
                }
            };
            let _ = span;
        }
        Stmt::Return { span, value } => {
            enc.map(2);
            enc.str("type");
            enc.str("Return");
            enc.str("value");
            match value {
                Some(v) => encode_expr(enc, v),
                None => enc.null(),
            };
            let _ = span;
        }
        Stmt::Yield { span, value } => {
            enc.map(2);
            enc.str("type");
            enc.str("Yield");
            enc.str("value");
            match value {
                Some(v) => encode_expr(enc, v),
                None => enc.null(),
            };
            let _ = span;
        }
        Stmt::Transaction { span, args, body } => {
            enc.map(3);
            enc.str("type");
            enc.str("Transaction");
            enc.str("args");
            encode_named_args(enc, args);
            enc.str("body");
            encode_block(enc, body);
            let _ = span;
        }
        Stmt::Effect { span, expr } => {
            enc.map(2);
            enc.str("type");
            enc.str("Effect");
            enc.str("expr");
            encode_expr(enc, expr);
            let _ = span;
        }
        Stmt::Expr { span, expr } => {
            enc.map(2);
            enc.str("type");
            enc.str("Expr");
            enc.str("expr");
            encode_expr(enc, expr);
            let _ = span;
        }
        Stmt::Block(b) => {
            enc.map(2);
            enc.str("type");
            enc.str("Block");
            enc.str("block");
            encode_block(enc, b);
        }
    }
}

fn encode_pattern(enc: &mut CborEncoder, p: &Pattern) {
    match p {
        Pattern::Wildcard => {
            enc.map(1);
            enc.str("type");
            enc.str("Wildcard");
        }
        Pattern::Ident(s) => {
            enc.map(2);
            enc.str("type");
            enc.str("Ident");
            enc.str("name");
            enc.str(s);
        }
        Pattern::Literal(l) => {
            enc.map(2);
            enc.str("type");
            enc.str("Literal");
            enc.str("lit");
            encode_literal(enc, l);
        }
        Pattern::Ok(p) => {
            enc.map(2);
            enc.str("type");
            enc.str("Ok");
            enc.str("inner");
            encode_pattern(enc, p);
        }
        Pattern::Err(p) => {
            enc.map(2);
            enc.str("type");
            enc.str("Err");
            enc.str("inner");
            encode_pattern(enc, p);
        }
        Pattern::Some(p) => {
            enc.map(2);
            enc.str("type");
            enc.str("Some");
            enc.str("inner");
            encode_pattern(enc, p);
        }
        Pattern::None => {
            enc.map(1);
            enc.str("type");
            enc.str("None");
        }
        Pattern::Variant {
            enum_name,
            variant_name,
            args,
        } => {
            enc.map(4);
            enc.str("type");
            enc.str("Variant");
            enc.str("enum");
            enc.str(enum_name);
            enc.str("variant");
            enc.str(variant_name);
            enc.str("args");
            {
                enc.array(args.len() as u64);
                for a in args {
                    encode_pattern(enc, a);
                }
            }
        }
        Pattern::Record(fields) => {
            enc.map(2);
            enc.str("type");
            enc.str("Record");
            enc.str("fields");
            enc.array(fields.len() as u64);
            for (name, pat) in fields {
                enc.map(2);
                enc.str("name");
                enc.str(name);
                enc.str("pattern");
                encode_pattern(enc, pat);
            }
        }
        Pattern::List(elements) => {
            enc.map(2);
            enc.str("type");
            enc.str("List");
            enc.str("elements");
            enc.array(elements.len() as u64);
            for p in elements {
                encode_pattern(enc, p);
            }
        }
        Pattern::Constructor { name, args } => {
            enc.map(3);
            enc.str("type");
            enc.str("Constructor");
            enc.str("name");
            enc.str(name);
            enc.str("args");
            enc.array(args.len() as u64);
            for a in args {
                encode_pattern(enc, a);
            }
        }
    }
}

fn encode_arm_body(enc: &mut CborEncoder, b: &ArmBody) {
    match b {
        ArmBody::Block(blk) => {
            enc.map(1);
            enc.str("Block");
            encode_block(enc, blk);
        }
        ArmBody::Expr(e) => {
            enc.map(1);
            enc.str("Expr");
            encode_expr(enc, e);
        }
    }
}

fn encode_expr(enc: &mut CborEncoder, e: &Expr) {
    enc.map(2);
    enc.str("kind");
    encode_expr_kind(enc, &e.kind);
    enc.str("span");
    encode_span(enc, e.span);
}

fn encode_expr_kind(enc: &mut CborEncoder, k: &ExprKind) {
    match k {
        ExprKind::Literal(l) => {
            enc.map(1);
            enc.str("Literal");
            encode_literal(enc, l);
        }
        ExprKind::Ident(s) => {
            enc.map(1);
            enc.str("Ident");
            enc.str(s);
        }
        ExprKind::QueryVar(s) => {
            enc.map(1);
            enc.str("QueryVar");
            enc.str(s);
        }
        ExprKind::Iri(s) => {
            enc.map(1);
            enc.str("Iri");
            enc.str(s);
        }
        ExprKind::Prefixed(p, l) => {
            enc.map(2);
            enc.str("prefix");
            enc.str(p);
            enc.str("local");
            enc.str(l);
        }
        ExprKind::Blank(s) => {
            enc.map(1);
            enc.str("Blank");
            enc.str(s);
        }
        ExprKind::Binary { op, left, right } => {
            enc.map(4);
            enc.str("type");
            enc.str("Binary");
            enc.str("op");
            encode_binop(enc, op);
            enc.str("left");
            encode_expr(enc, left);
            enc.str("right");
            encode_expr(enc, right);
        }
        ExprKind::Unary { op, expr } => {
            enc.map(3);
            enc.str("type");
            enc.str("Unary");
            enc.str("op");
            encode_unop(enc, op);
            enc.str("expr");
            encode_expr(enc, expr);
        }
        ExprKind::Await(e) => {
            enc.map(1);
            enc.str("Await");
            encode_expr(enc, e);
        }
        ExprKind::Member { recv, name } => {
            enc.map(3);
            enc.str("type");
            enc.str("Member");
            enc.str("recv");
            encode_expr(enc, recv);
            enc.str("name");
            enc.str(name);
        }
        ExprKind::Call { callee, args } => {
            enc.map(3);
            enc.str("type");
            enc.str("Call");
            enc.str("callee");
            encode_expr(enc, callee);
            enc.str("args");
            encode_args(enc, args);
        }
        ExprKind::Index { recv, index } => {
            enc.map(3);
            enc.str("type");
            enc.str("Index");
            enc.str("recv");
            encode_expr(enc, recv);
            enc.str("index");
            encode_expr(enc, index);
        }
        ExprKind::Try(e) => {
            enc.map(1);
            enc.str("Try");
            encode_expr(enc, e);
        }
        ExprKind::List(es) => {
            enc.map(1);
            enc.str("List");
            enc.array(es.len() as u64);
            for e in es {
                encode_expr(enc, e);
            }
        }
        ExprKind::Record(args) => {
            enc.map(1);
            enc.str("Record");
            encode_named_args(enc, args);
        }
        ExprKind::Triple {
            subject,
            predicate,
            object,
        } => {
            enc.map(4);
            enc.str("type");
            enc.str("Triple");
            enc.str("subject");
            encode_expr(enc, subject);
            enc.str("predicate");
            encode_expr(enc, predicate);
            enc.str("object");
            encode_expr(enc, object);
        }
        ExprKind::Reified {
            subject,
            predicate,
            object,
            reifier,
        } => {
            enc.map(5);
            enc.str("type");
            enc.str("Reified");
            enc.str("subject");
            encode_expr(enc, subject);
            enc.str("predicate");
            encode_expr(enc, predicate);
            enc.str("object");
            encode_expr(enc, object);
            enc.str("reifier");
            encode_expr(enc, reifier);
        }
        ExprKind::Pipe { left, right } => {
            enc.map(2);
            enc.str("left");
            encode_expr(enc, left);
            enc.str("right");
            encode_expr(enc, right);
        }
        ExprKind::GraphQuery { is_ask, pattern, variables } => {
            enc.map(3);
            enc.str("is_ask");
            enc.bool(*is_ask);
            enc.str("pattern");
            enc.str(pattern);
            enc.str("variables");
            enc.array(variables.len() as u64);
            for v in variables {
                enc.str(v);
            }
        }
        ExprKind::ModalLogic { modality, args, body } => {
            enc.map(3);
            enc.str("modality");
            enc.str(match modality {
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
            });
            enc.str("args");
            enc.array(args.len() as u64);
            for a in args {
                encode_expr(enc, a);
            }
            enc.str("has_body");
            enc.bool(body.is_some());
        }
        ExprKind::Interpolate(parts) => {
            enc.map(1);
            enc.str("Interpolate");
            enc.array(parts.len() as u64);
            for p in parts {
                encode_expr(enc, p);
            }
        }
        ExprKind::Lambda { params, body } => {
            enc.map(3);
            enc.str("type");
            enc.str("Lambda");
            enc.str("params");
            encode_str_list(enc, params);
            enc.str("body");
            encode_expr(enc, body);
        }
        ExprKind::Tween {
            from,
            to,
            over,
            ease,
            spring,
        } => {
            enc.map(6);
            enc.str("type");
            enc.str("Tween");
            enc.str("from");
            encode_expr(enc, from);
            enc.str("to");
            encode_expr(enc, to);
            enc.str("over");
            encode_expr(enc, over);
            enc.str("ease");
            match ease {
                Some(s) => enc.str(s),
                None => enc.null(),
            };
            enc.str("spring");
            match spring {
                Some(args) => encode_named_args(enc, args),
                None => enc.null(),
            };
        }
    }
}

fn encode_args(enc: &mut CborEncoder, args: &[Arg]) {
    enc.array(args.len() as u64);
    for arg in args {
        match arg {
            Arg::Pos(e) => {
                enc.map(1);
                enc.str("Pos");
                encode_expr(enc, e);
            }
            Arg::Named(na) => {
                enc.map(1);
                enc.str("Named");
                encode_named_arg(enc, na);
            }
        }
    }
}

fn encode_named_arg(enc: &mut CborEncoder, na: &NamedArg) {
    enc.map(3);
    enc.str("name");
    enc.str(&na.name);
    enc.str("value");
    encode_expr(enc, &na.value);
    enc.str("span");
    encode_span(enc, na.span);
}

fn encode_binop(enc: &mut CborEncoder, op: &BinOp) {
    let s = match op {
        BinOp::Or => "Or",
        BinOp::And => "And",
        BinOp::Eq => "Eq",
        BinOp::Ne => "Ne",
        BinOp::Lt => "Lt",
        BinOp::Le => "Le",
        BinOp::Gt => "Gt",
        BinOp::Ge => "Ge",
        BinOp::Add => "Add",
        BinOp::Sub => "Sub",
        BinOp::Mul => "Mul",
        BinOp::Div => "Div",
        BinOp::Rem => "Rem",
    };
    enc.str(s);
}

fn encode_unop(enc: &mut CborEncoder, op: &UnOp) {
    let s = match op {
        UnOp::Not => "Not",
        UnOp::Neg => "Neg",
        UnOp::Plus => "Plus",
    };
    enc.str(s);
}

fn encode_literal(enc: &mut CborEncoder, l: &Literal) {
    match l {
        Literal::Null => {
            enc.map(1);
            enc.str("type");
            enc.str("Null");
        }
        Literal::Bool(b) => {
            enc.map(2);
            enc.str("type");
            enc.str("Bool");
            enc.str("value");
            enc.bool(*b);
        }
        Literal::Int(n) => {
            enc.map(2);
            enc.str("type");
            enc.str("Int");
            enc.str("value");
            enc.int(*n);
        }
        Literal::UInt(n) => {
            enc.map(2);
            enc.str("type");
            enc.str("UInt");
            enc.str("value");
            enc.uint(*n);
        }
        Literal::Float(bits) => {
            enc.map(2);
            enc.str("type");
            enc.str("Float");
            enc.str("bits");
            enc.uint(*bits);
        }
        Literal::Quantity { value, unit } => {
            enc.map(3);
            enc.str("type");
            enc.str("Quantity");
            enc.str("value");
            enc.uint(*value);
            enc.str("unit");
            enc.str(unit);
        }
        Literal::String(s) => {
            enc.map(2);
            enc.str("type");
            enc.str("String");
            enc.str("value");
            enc.str(s);
        }
        Literal::Color(c) => {
            enc.map(5);
            enc.str("type");
            enc.str("Color");
            enc.str("r");
            enc.uint(c.r as u64);
            enc.str("g");
            enc.uint(c.g as u64);
            enc.str("b");
            enc.uint(c.b as u64);
            enc.str("a");
            enc.uint(c.a as u64);
        }
    }
}

fn encode_span(enc: &mut CborEncoder, s: Span) {
    enc.array(2);
    enc.uint(s.start as u64);
    enc.uint(s.end as u64);
}

fn encode_str_list(enc: &mut CborEncoder, list: &[String]) {
    enc.array(list.len() as u64);
    for s in list {
        enc.str(s);
    }
}
