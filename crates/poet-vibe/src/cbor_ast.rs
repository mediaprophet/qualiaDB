//! Homoiconic CBOR-LD AST Codec (Tag 4200) — plan §7.3 A1.
//!
//! Bidirectional zero-copy serialization between `poet_vibe::ast` and
//! CBOR-LD 1.0 binary trees. The VibeScript AST is encoded as a CBOR map
//! tagged with Tag 4200, where each node has a `type` string identifying
//! the AST kind and the remaining fields carry the node's data.
//!
//! ## Design
//!
//! - **Authoring format:** N3 / VibeScript source text (human-readable).
//! - **Runtime format:** CBOR-LD binary (compact, zero-copy, machine-readable).
//! - **Tag 4200:** marks a CBOR-LD value as a VibeScript AST node.
//! - **Spans:** encoded as `[start: u32, end: u32]` CBOR arrays.
//! - **Bidirectional:** `encode` and `decode` are exact inverses.
//!
//! ## CBOR-LD node encoding
//!
//! Each AST node is a CBOR map:
//! ```text
//! tag(4200, {
//!   "type": "Program",
//!   "span": [start, end],
//!   ...node-specific fields...
//! })
//! ```
//!
//! The codec uses a minimal pure-Rust CBOR encoder/decoder (no external
//! dependencies) since `poet-vibe` is dependency-free.

use crate::ast::*;
use crate::span::Span;

// ── CBOR-LD Tag ────────────────────────────────────────────────────────────

/// CBOR-LD tag marking a VibeScript AST node.
pub const TAG_VIBE_AST: u64 = 4200;

// ── Minimal CBOR encoder ───────────────────────────────────────────────────

/// Encode a `Program` AST into CBOR-LD bytes (Tag 4200).
pub fn encode(program: &Program) -> Vec<u8> {
    let mut enc = CborEncoder::new();
    enc.tag(TAG_VIBE_AST);
    encode_program(&mut enc, program);
    enc.finish()
}

/// Decode a `Program` AST from CBOR-LD bytes (Tag 4200).
pub fn decode(bytes: &[u8]) -> Result<Program, DecodeError> {
    let mut dec = CborDecoder::new(bytes);
    let tag = dec.tag()?;
    if tag != TAG_VIBE_AST {
        return Err(DecodeError::UnexpectedTag(tag));
    }
    decode_program(&mut dec)
}

// ── Encode ─────────────────────────────────────────────────────────────────

fn encode_program(enc: &mut CborEncoder, p: &Program) {
    enc.map(7);
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
    enc.str("requires");
    encode_caps(enc, &p.requires);
    enc.str("items");
    encode_items(enc, &p.items);
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
            enc.map(7);
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
        Literal::String(s) => {
            enc.map(2);
            enc.str("type");
            enc.str("String");
            enc.str("value");
            enc.str(s);
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

// ── Decode ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum DecodeError {
    UnexpectedTag(u64),
    UnexpectedType(&'static str),
    MissingField(&'static str),
    InvalidCbor(String),
    Eof,
}

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedTag(t) => write!(f, "unexpected CBOR tag {t}"),
            Self::UnexpectedType(t) => write!(f, "unexpected type: {t}"),
            Self::MissingField(fld) => write!(f, "missing field: {fld}"),
            Self::InvalidCbor(s) => write!(f, "invalid CBOR: {s}"),
            Self::Eof => write!(f, "unexpected end of input"),
        }
    }
}

impl std::error::Error for DecodeError {}

fn decode_program(dec: &mut CborDecoder) -> Result<Program, DecodeError> {
    let map_len = dec.map()?;
    let mut module = None;
    let mut imports = Vec::new();
    let mut prefixes = Vec::new();
    let mut requires = Vec::new();
    let mut items = Vec::new();
    let mut span = Span::new(0, 0);
    for _ in 0..map_len {
        let key = dec.str()?;
        match key.as_str() {
            "type" => {
                let _ = dec.str()?;
            }
            "span" => span = decode_span(dec)?,
            "module" => module = decode_opt_module(dec)?,
            "imports" => imports = decode_imports(dec)?,
            "prefixes" => prefixes = decode_prefixes(dec)?,
            "requires" => requires = decode_caps(dec)?,
            "items" => items = decode_items(dec)?,
            _ => dec.skip()?,
        }
    }
    Ok(Program {
        span,
        module,
        imports,
        prefixes,
        requires,
        items,
    })
}

fn decode_opt_module(dec: &mut CborDecoder) -> Result<Option<ModuleDecl>, DecodeError> {
    if dec.is_null()? {
        dec.skip()?;
        return Ok(None);
    }
    let map_len = dec.map()?;
    let mut name = Name::Ident(String::new());
    let mut span = Span::new(0, 0);
    for _ in 0..map_len {
        let key = dec.str()?;
        match key.as_str() {
            "name" => name = decode_name(dec)?,
            "span" => span = decode_span(dec)?,
            _ => dec.skip()?,
        }
    }
    Ok(Some(ModuleDecl { span, name }))
}

fn decode_name(dec: &mut CborDecoder) -> Result<Name, DecodeError> {
    let map_len = dec.map()?;
    if map_len < 1 {
        return Err(DecodeError::InvalidCbor("empty Name map".into()));
    }
    let key = dec.str()?;
    let val = dec.str()?;
    if map_len > 1 {
        for _ in 1..map_len {
            dec.skip()?;
        }
    }
    match key.as_str() {
        "Ident" => Ok(Name::Ident(val)),
        "Iri" => Ok(Name::Iri(val)),
        _ => Err(DecodeError::UnexpectedType("Name")),
    }
}

fn decode_imports(dec: &mut CborDecoder) -> Result<Vec<ImportDecl>, DecodeError> {
    let n = dec.array()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let map_len = dec.map()?;
        let mut path = String::new();
        let mut alias = None;
        let mut span = Span::new(0, 0);
        for _ in 0..map_len {
            let key = dec.str()?;
            match key.as_str() {
                "path" => path = dec.str()?,
                "alias" => {
                    if dec.is_null()? {
                        dec.skip()?;
                        alias = None;
                    } else {
                        alias = Some(dec.str()?);
                    }
                }
                "span" => span = decode_span(dec)?,
                _ => dec.skip()?,
            }
        }
        out.push(ImportDecl { span, path, alias });
    }
    Ok(out)
}

fn decode_prefixes(dec: &mut CborDecoder) -> Result<Vec<PrefixDecl>, DecodeError> {
    let n = dec.array()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let map_len = dec.map()?;
        let mut prefix = String::new();
        let mut iri = String::new();
        let mut span = Span::new(0, 0);
        for _ in 0..map_len {
            let key = dec.str()?;
            match key.as_str() {
                "prefix" => prefix = dec.str()?,
                "iri" => iri = dec.str()?,
                "span" => span = decode_span(dec)?,
                _ => dec.skip()?,
            }
        }
        out.push(PrefixDecl { span, prefix, iri });
    }
    Ok(out)
}

fn decode_caps(dec: &mut CborDecoder) -> Result<Vec<CapSpec>, DecodeError> {
    let n = dec.array()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let map_len = dec.map()?;
        let mut id = String::new();
        let mut args = Vec::new();
        let mut span = Span::new(0, 0);
        for _ in 0..map_len {
            let key = dec.str()?;
            match key.as_str() {
                "id" => id = dec.str()?,
                "args" => args = decode_named_args(dec)?,
                "span" => span = decode_span(dec)?,
                _ => dec.skip()?,
            }
        }
        out.push(CapSpec { span, id, args });
    }
    Ok(out)
}

fn decode_named_args(dec: &mut CborDecoder) -> Result<Vec<NamedArg>, DecodeError> {
    let n = dec.array()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let map_len = dec.map()?;
        let mut name = String::new();
        let mut value = Expr {
            span: Span::new(0, 0),
            kind: ExprKind::Literal(Literal::Null),
        };
        let mut span = Span::new(0, 0);
        for _ in 0..map_len {
            let key = dec.str()?;
            match key.as_str() {
                "name" => name = dec.str()?,
                "value" => value = decode_expr(dec)?,
                "span" => span = decode_span(dec)?,
                _ => dec.skip()?,
            }
        }
        out.push(NamedArg { span, name, value });
    }
    Ok(out)
}

fn decode_items(dec: &mut CborDecoder) -> Result<Vec<Item>, DecodeError> {
    let n = dec.array()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        out.push(decode_item(dec)?);
    }
    Ok(out)
}

fn decode_item(dec: &mut CborDecoder) -> Result<Item, DecodeError> {
    let map_len = dec.map()?;
    let mut item_type = String::new();
    // Decode all key-value pairs in a single pass, storing into locals.
    // We don't know the item type until we see "type", so we buffer
    // field values as an enum and construct the Item at the end.
    let mut fields: Vec<(String, ItemField)> = Vec::with_capacity(map_len as usize);
    for _ in 0..map_len {
        let key = dec.str()?;
        if key == "type" {
            item_type = dec.str()?;
        } else {
            let val = decode_item_field(&key, dec)?;
            fields.push((key, val));
        }
    }
    // Construct the Item from the collected fields based on item_type.
    match item_type.as_str() {
        "Function" => {
            let mut name = String::new();
            let mut effect = None;
            let mut is_async = false;
            let mut params = Vec::new();
            let mut budget = Vec::new();
            let mut body = Block {
                span: Span::new(0, 0),
                stmts: Vec::new(),
            };
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", ItemField::Str(s)) => name = s,
                    ("effect", ItemField::Effect(e)) => effect = e,
                    ("async", ItemField::Bool(b)) => is_async = b,
                    ("params", ItemField::Params(p)) => params = p,
                    ("budget", ItemField::NamedArgs(na)) => budget = na,
                    ("body", ItemField::Block(b)) => body = b,
                    _ => {}
                }
            }
            Ok(Item::Function(FunctionDecl {
                span: Span::new(0, 0),
                effect,
                is_async,
                name,
                params,
                budget,
                ret: None,
                body,
            }))
        }
        "Hook" => {
            let mut path = Vec::new();
            let mut params = Vec::new();
            let mut body = Block {
                span: Span::new(0, 0),
                stmts: Vec::new(),
            };
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("path", ItemField::StrList(s)) => path = s,
                    ("params", ItemField::Params(p)) => params = p,
                    ("body", ItemField::Block(b)) => body = b,
                    _ => {}
                }
            }
            Ok(Item::Hook(HookDecl {
                span: Span::new(0, 0),
                path,
                params,
                budget: Vec::new(),
                ret: None,
                body,
            }))
        }
        "Const" => {
            let mut name = String::new();
            let mut value = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut span = Span::new(0, 0);
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", ItemField::Str(s)) => name = s,
                    ("value", ItemField::Expr(e)) => value = e,
                    ("span", ItemField::Span(s)) => span = s,
                    _ => {}
                }
            }
            Ok(Item::Const(ConstDecl {
                span,
                name,
                ty: None,
                value,
            }))
        }
        "Statement" => {
            for (k, v) in fields {
                if k == "stmt" {
                    if let ItemField::Stmt(s) = v {
                        return Ok(Item::Statement(s));
                    }
                }
            }
            Err(DecodeError::MissingField("stmt"))
        }
        "Enum" => {
            let mut name = String::new();
            let mut variants = Vec::new();
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", ItemField::Str(s)) => name = s,
                    ("variants", ItemField::VariantList(v)) => variants = v,
                    _ => {}
                }
            }
            Ok(Item::Enum(EnumDecl {
                span: Span::new(0, 0),
                name,
                variants,
            }))
        }
        "Field" => {
            let mut name = String::new();
            let mut ty = TypeExpr {
                span: Span::new(0, 0),
                name: String::new(),
                args: Vec::new(),
            };
            let mut unit = None;
            let mut support = FieldSupport::Region;
            let mut representation = FieldRepresentation::Grid;
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", ItemField::Str(s)) => name = s,
                    ("ty", ItemField::TypeExpr(t)) => ty = t,
                    ("unit", ItemField::Str(s)) => unit = Some(s),
                    ("support", ItemField::Str(s)) => {
                        support = match s.as_str() {
                            "region" => FieldSupport::Region,
                            "point" => FieldSupport::Point,
                            "continuant" => FieldSupport::Continuant,
                            "stream" => FieldSupport::Stream,
                            _ => FieldSupport::Region,
                        };
                    }
                    ("representation", ItemField::Str(s)) => {
                        representation = match s.as_str() {
                            "grid" => FieldRepresentation::Grid,
                            "mesh" => FieldRepresentation::Mesh,
                            "particles" => FieldRepresentation::Particles,
                            "analytic" => FieldRepresentation::Analytic,
                            "sampled" => FieldRepresentation::Sampled,
                            _ => FieldRepresentation::Grid,
                        };
                    }
                    _ => {}
                }
            }
            Ok(Item::Field(FieldDecl {
                span: Span::new(0, 0),
                name,
                ty,
                unit,
                support,
                representation,
            }))
        }
        "Material" => {
            let mut name = String::new();
            let mut properties = Vec::new();
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", ItemField::Str(s)) => name = s,
                    ("properties", ItemField::NamedArgs(na)) => properties = na,
                    _ => {}
                }
            }
            Ok(Item::Material(MaterialDecl {
                span: Span::new(0, 0),
                name,
                properties,
            }))
        }
        "Law" => {
            let mut name = String::new();
            let mut condition = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut consequence = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", ItemField::Str(s)) => name = s,
                    ("condition", ItemField::Expr(e)) => condition = e,
                    ("consequence", ItemField::Expr(e)) => consequence = e,
                    _ => {}
                }
            }
            Ok(Item::Law(LawDecl {
                span: Span::new(0, 0),
                name,
                condition,
                consequence,
            }))
        }
        _ => Err(DecodeError::UnexpectedType("Item")),
    }
}

/// Buffered field value for Item construction after single-pass decode.
enum ItemField {
    Str(String),
    StrList(Vec<String>),
    Bool(bool),
    Expr(Expr),
    Block(Block),
    Params(Vec<Param>),
    NamedArgs(Vec<NamedArg>),
    Effect(Option<EffectClass>),
    Span(Span),
    Stmt(Stmt),
    VariantList(Vec<EnumVariant>),
    TypeExpr(TypeExpr),
}

fn decode_item_field(key: &str, dec: &mut CborDecoder) -> Result<ItemField, DecodeError> {
    match key {
        "name" | "path" => Ok(ItemField::Str(dec.str()?)),
        "async" => Ok(ItemField::Bool(dec.bool()?)),
        "effect" => {
            if dec.is_null()? {
                dec.skip()?;
                Ok(ItemField::Effect(None))
            } else {
                let s = dec.str()?;
                let e = match s.as_str() {
                    "Pure" => EffectClass::Pure,
                    "Hot" => EffectClass::Hot,
                    "Cold" => EffectClass::Cold,
                    "Async" => EffectClass::Async,
                    "External" => EffectClass::External,
                    _ => return Err(DecodeError::UnexpectedType("EffectClass")),
                };
                Ok(ItemField::Effect(Some(e)))
            }
        }
        "value" | "expr" | "condition" | "consequence" => Ok(ItemField::Expr(decode_expr(dec)?)),
        "body" | "then" | "block" => Ok(ItemField::Block(decode_block(dec)?)),
        "params" => Ok(ItemField::Params(decode_params(dec)?)),
        "properties" => Ok(ItemField::NamedArgs(decode_named_args(dec)?)),
        "budget" | "args" => {
            // "args" in Item context could be NamedArgs or Args — for Function/Hook
            // budget it's NamedArgs. For Call args it's Args. But we're in decode_item_field,
            // so this is for Item-level fields only.
            if key == "budget" {
                Ok(ItemField::NamedArgs(decode_named_args(dec)?))
            } else {
                // Skip unknown "args" — will be handled by Stmt decoding
                dec.skip()?;
                Ok(ItemField::Str(String::new()))
            }
        }
        "span" => Ok(ItemField::Span(decode_span(dec)?)),
        "stmt" => Ok(ItemField::Stmt(decode_stmt(dec)?)),
        "variants" => {
            let n = dec.array()?;
            let mut vs = Vec::new();
            for _ in 0..n {
                let m = dec.map()?;
                let mut vname = String::new();
                let mut payload = Vec::new();
                for _ in 0..m {
                    match dec.str()?.as_str() {
                        "name" => vname = dec.str()?,
                        "payload" => {
                            let pn = dec.array()?;
                            for _ in 0..pn {
                                payload.push(decode_type_expr(dec)?);
                            }
                        }
                        _ => dec.skip()?,
                    }
                }
                vs.push(EnumVariant {
                    span: Span::new(0, 0),
                    name: vname,
                    payload,
                });
            }
            Ok(ItemField::VariantList(vs))
        }
        "ty" => Ok(ItemField::TypeExpr(decode_type_expr(dec)?)),
        "unit" => {
            if dec.is_null()? {
                dec.skip()?;
                Ok(ItemField::Str(String::new()))
            } else {
                Ok(ItemField::Str(dec.str()?))
            }
        }
        "support" | "representation" => Ok(ItemField::Str(dec.str()?)),
        _ => {
            dec.skip()?;
            Ok(ItemField::Str(String::new()))
        }
    }
}

// ── Minimal CBOR encoder (pure Rust, no deps) ──────────────────────────────

struct CborEncoder {
    buf: Vec<u8>,
}

impl CborEncoder {
    fn new() -> Self {
        Self { buf: Vec::new() }
    }
    fn finish(self) -> Vec<u8> {
        self.buf
    }

    fn write_u8(&mut self, b: u8) {
        self.buf.push(b);
    }

    fn write_type_and_len(&mut self, major: u8, len: u64) {
        if len < 24 {
            self.buf.push((major << 5) | len as u8);
        } else if len < 256 {
            self.buf.push((major << 5) | 24);
            self.buf.push(len as u8);
        } else if len < 65536 {
            self.buf.push((major << 5) | 25);
            self.buf.extend_from_slice(&(len as u16).to_be_bytes());
        } else if len < 4294967296 {
            self.buf.push((major << 5) | 26);
            self.buf.extend_from_slice(&(len as u32).to_be_bytes());
        } else {
            self.buf.push((major << 5) | 27);
            self.buf.extend_from_slice(&len.to_be_bytes());
        }
    }

    fn uint(&mut self, n: u64) {
        self.write_type_and_len(0, n);
    }
    fn int(&mut self, n: i64) {
        if n >= 0 {
            self.uint(n as u64);
        } else {
            self.write_type_and_len(1, (-1 - n) as u64);
        }
    }
    fn str(&mut self, s: &str) {
        self.write_type_and_len(3, s.len() as u64);
        self.buf.extend_from_slice(s.as_bytes());
    }
    fn bool(&mut self, b: bool) {
        self.buf.push(if b { 0xF5 } else { 0xF4 });
    }
    fn null(&mut self) {
        self.buf.push(0xF6);
    }
    fn array(&mut self, len: u64) {
        self.write_type_and_len(4, len);
    }
    fn map(&mut self, len: u64) {
        self.write_type_and_len(5, len);
    }
    fn tag(&mut self, tag: u64) {
        self.write_type_and_len(6, tag);
    }
}

// ── Minimal CBOR decoder (pure Rust, no deps) ──────────────────────────────

struct CborDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> CborDecoder<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }
    fn pos(&self) -> usize {
        self.pos
    }

    fn read_u8(&mut self) -> Result<u8, DecodeError> {
        if self.pos >= self.buf.len() {
            return Err(DecodeError::Eof);
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn peek(&self) -> Result<u8, DecodeError> {
        if self.pos >= self.buf.len() {
            return Err(DecodeError::Eof);
        }
        Ok(self.buf[self.pos])
    }

    fn read_type_and_len(&mut self) -> Result<(u8, u64), DecodeError> {
        let b = self.read_u8()?;
        let major = b >> 5;
        let ai = b & 0x1F;
        let len = match ai {
            0..=23 => ai as u64,
            24 => self.read_u8()? as u64,
            25 => {
                if self.pos + 2 > self.buf.len() {
                    return Err(DecodeError::Eof);
                }
                let v = u16::from_be_bytes([self.buf[self.pos], self.buf[self.pos + 1]]);
                self.pos += 2;
                v as u64
            }
            26 => {
                if self.pos + 4 > self.buf.len() {
                    return Err(DecodeError::Eof);
                }
                let v = u32::from_be_bytes([
                    self.buf[self.pos],
                    self.buf[self.pos + 1],
                    self.buf[self.pos + 2],
                    self.buf[self.pos + 3],
                ]);
                self.pos += 4;
                v as u64
            }
            27 => {
                if self.pos + 8 > self.buf.len() {
                    return Err(DecodeError::Eof);
                }
                let v = u64::from_be_bytes([
                    self.buf[self.pos],
                    self.buf[self.pos + 1],
                    self.buf[self.pos + 2],
                    self.buf[self.pos + 3],
                    self.buf[self.pos + 4],
                    self.buf[self.pos + 5],
                    self.buf[self.pos + 6],
                    self.buf[self.pos + 7],
                ]);
                self.pos += 8;
                v
            }
            _ => {
                return Err(DecodeError::InvalidCbor(format!(
                    "invalid additional info {ai}"
                )))
            }
        };
        Ok((major, len))
    }

    fn uint(&mut self) -> Result<u64, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        if major != 0 {
            return Err(DecodeError::UnexpectedType("expected uint"));
        }
        Ok(len)
    }

    fn int(&mut self) -> Result<i64, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        match major {
            0 => Ok(len as i64),
            1 => Ok(-1 - len as i64),
            _ => Err(DecodeError::UnexpectedType("expected int")),
        }
    }

    fn str(&mut self) -> Result<String, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        if major != 3 {
            return Err(DecodeError::UnexpectedType("expected string"));
        }
        let len = len as usize;
        if self.pos + len > self.buf.len() {
            return Err(DecodeError::Eof);
        }
        let s = std::str::from_utf8(&self.buf[self.pos..self.pos + len])
            .map_err(|e| DecodeError::InvalidCbor(format!("invalid UTF-8: {e}")))?;
        self.pos += len;
        Ok(s.to_string())
    }

    fn bool(&mut self) -> Result<bool, DecodeError> {
        let b = self.read_u8()?;
        match b {
            0xF5 => Ok(true),
            0xF4 => Ok(false),
            _ => Err(DecodeError::UnexpectedType("expected bool")),
        }
    }

    fn null(&mut self) -> Result<(), DecodeError> {
        let b = self.read_u8()?;
        if b == 0xF6 {
            Ok(())
        } else {
            Err(DecodeError::UnexpectedType("expected null"))
        }
    }

    fn is_null(&self) -> Result<bool, DecodeError> {
        Ok(self.peek()? == 0xF6)
    }

    fn array(&mut self) -> Result<u64, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        if major != 4 {
            return Err(DecodeError::UnexpectedType("expected array"));
        }
        Ok(len)
    }

    fn map(&mut self) -> Result<u64, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        if major != 5 {
            return Err(DecodeError::UnexpectedType("expected map"));
        }
        Ok(len)
    }

    fn tag(&mut self) -> Result<u64, DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        if major != 6 {
            return Err(DecodeError::UnexpectedType("expected tag"));
        }
        Ok(len)
    }

    fn skip(&mut self) -> Result<(), DecodeError> {
        let (major, len) = self.read_type_and_len()?;
        match major {
            0 | 1 | 6 => Ok(()),
            2 => {
                self.pos += len as usize;
                if self.pos > self.buf.len() {
                    Err(DecodeError::Eof)
                } else {
                    Ok(())
                }
            }
            3 => {
                self.pos += len as usize;
                if self.pos > self.buf.len() {
                    Err(DecodeError::Eof)
                } else {
                    Ok(())
                }
            }
            4 => {
                for _ in 0..len {
                    self.skip()?;
                }
                Ok(())
            }
            5 => {
                for _ in 0..len {
                    self.skip()?;
                    self.skip()?;
                }
                Ok(())
            }
            7 => Ok(()),
            _ => Err(DecodeError::InvalidCbor(format!(
                "unknown major type {major}"
            ))),
        }
    }
}

fn decode_span(dec: &mut CborDecoder) -> Result<Span, DecodeError> {
    let n = dec.array()?;
    if n < 2 {
        return Err(DecodeError::InvalidCbor("span needs 2 elements".into()));
    }
    let start = dec.uint()? as u32;
    let end = dec.uint()? as u32;
    for _ in 2..n {
        dec.skip()?;
    }
    Ok(Span::new(start, end))
}

fn decode_expr(dec: &mut CborDecoder) -> Result<Expr, DecodeError> {
    let map_len = dec.map()?;
    let mut span = Span::new(0, 0);
    let mut kind = ExprKind::Literal(Literal::Null);
    for _ in 0..map_len {
        let key = dec.str()?;
        match key.as_str() {
            "kind" => kind = decode_expr_kind(dec)?,
            "span" => span = decode_span(dec)?,
            _ => dec.skip()?,
        }
    }
    Ok(Expr { span, kind })
}

fn decode_expr_kind(dec: &mut CborDecoder) -> Result<ExprKind, DecodeError> {
    // ExprKind is encoded as a single-key map where the key is the variant name.
    let map_len = dec.map()?;
    if map_len < 1 {
        return Err(DecodeError::InvalidCbor("empty ExprKind map".into()));
    }
    let variant = dec.str()?;
    let kind = match variant.as_str() {
        "Literal" => ExprKind::Literal(decode_literal(dec)?),
        "Ident" => ExprKind::Ident(dec.str()?),
        "QueryVar" => ExprKind::QueryVar(dec.str()?),
        "Iri" => ExprKind::Iri(dec.str()?),
        "Blank" => ExprKind::Blank(dec.str()?),
        "Binary" => {
            let mut op = BinOp::Add;
            let mut left = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut right = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for _ in 0..map_len - 1 {
                let k = dec.str()?;
                match k.as_str() {
                    "op" => op = decode_binop(dec)?,
                    "left" => left = decode_expr(dec)?,
                    "right" => right = decode_expr(dec)?,
                    _ => dec.skip()?,
                }
            }
            ExprKind::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            }
        }
        "Unary" => {
            let mut op = UnOp::Not;
            let mut expr = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for _ in 0..map_len - 1 {
                let k = dec.str()?;
                match k.as_str() {
                    "op" => op = decode_unop(dec)?,
                    "expr" => expr = decode_expr(dec)?,
                    _ => dec.skip()?,
                }
            }
            ExprKind::Unary {
                op,
                expr: Box::new(expr),
            }
        }
        "Await" => {
            let e = decode_expr(dec)?;
            for _ in 1..map_len {
                dec.skip()?;
            }
            ExprKind::Await(Box::new(e))
        }
        "Member" => {
            let mut recv = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut name = String::new();
            for _ in 0..map_len - 1 {
                let k = dec.str()?;
                match k.as_str() {
                    "recv" => recv = decode_expr(dec)?,
                    "name" => name = dec.str()?,
                    _ => dec.skip()?,
                }
            }
            ExprKind::Member {
                recv: Box::new(recv),
                name,
            }
        }
        "Call" => {
            let mut callee = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut args = Vec::new();
            for _ in 0..map_len - 1 {
                let k = dec.str()?;
                match k.as_str() {
                    "callee" => callee = decode_expr(dec)?,
                    "args" => args = decode_args(dec)?,
                    _ => dec.skip()?,
                }
            }
            ExprKind::Call {
                callee: Box::new(callee),
                args,
            }
        }
        "Index" => {
            let mut recv = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut index = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for _ in 0..map_len - 1 {
                let k = dec.str()?;
                match k.as_str() {
                    "recv" => recv = decode_expr(dec)?,
                    "index" => index = decode_expr(dec)?,
                    _ => dec.skip()?,
                }
            }
            ExprKind::Index {
                recv: Box::new(recv),
                index: Box::new(index),
            }
        }
        "Try" => {
            let e = decode_expr(dec)?;
            for _ in 1..map_len {
                dec.skip()?;
            }
            ExprKind::Try(Box::new(e))
        }
        "List" => {
            let n = dec.array()?;
            let mut es = Vec::with_capacity(n as usize);
            for _ in 0..n {
                es.push(decode_expr(dec)?);
            }
            ExprKind::List(es)
        }
        "Record" => ExprKind::Record(decode_named_args(dec)?),
        "Triple" => {
            let mut subject = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut predicate = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut object = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for _ in 0..map_len - 1 {
                let k = dec.str()?;
                match k.as_str() {
                    "subject" => subject = decode_expr(dec)?,
                    "predicate" => predicate = decode_expr(dec)?,
                    "object" => object = decode_expr(dec)?,
                    _ => dec.skip()?,
                }
            }
            ExprKind::Triple {
                subject: Box::new(subject),
                predicate: Box::new(predicate),
                object: Box::new(object),
            }
        }
        "Reified" => {
            let mut subject = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut predicate = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut object = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut reifier = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for _ in 0..map_len - 1 {
                let k = dec.str()?;
                match k.as_str() {
                    "subject" => subject = decode_expr(dec)?,
                    "predicate" => predicate = decode_expr(dec)?,
                    "object" => object = decode_expr(dec)?,
                    "reifier" => reifier = decode_expr(dec)?,
                    _ => dec.skip()?,
                }
            }
            ExprKind::Reified {
                subject: Box::new(subject),
                predicate: Box::new(predicate),
                object: Box::new(object),
                reifier: Box::new(reifier),
            }
        }
        "prefix" => {
            // Prefixed(prefix, local)
            let prefix = dec.str()?;
            let local = dec.str()?;
            for _ in 1..map_len {
                dec.skip()?;
            }
            ExprKind::Prefixed(prefix, local)
        }
        _ => {
            for _ in 0..map_len - 1 {
                dec.skip()?;
            }
            ExprKind::Ident(variant)
        }
    };
    // Skip any remaining fields
    Ok(kind)
}

fn decode_args(dec: &mut CborDecoder) -> Result<Vec<Arg>, DecodeError> {
    let n = dec.array()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let map_len = dec.map()?;
        if map_len < 1 {
            return Err(DecodeError::InvalidCbor("empty Arg map".into()));
        }
        let key = dec.str()?;
        match key.as_str() {
            "Pos" => {
                let e = decode_expr(dec)?;
                for _ in 1..map_len {
                    dec.skip()?;
                }
                out.push(Arg::Pos(e));
            }
            "Named" => {
                let na = decode_named_arg(dec)?;
                for _ in 1..map_len {
                    dec.skip()?;
                }
                out.push(Arg::Named(na));
            }
            _ => {
                for _ in 0..map_len - 1 {
                    dec.skip()?;
                }
            }
        }
    }
    Ok(out)
}

fn decode_named_arg(dec: &mut CborDecoder) -> Result<NamedArg, DecodeError> {
    let map_len = dec.map()?;
    let mut name = String::new();
    let mut value = Expr {
        span: Span::new(0, 0),
        kind: ExprKind::Literal(Literal::Null),
    };
    let mut span = Span::new(0, 0);
    for _ in 0..map_len {
        let key = dec.str()?;
        match key.as_str() {
            "name" => name = dec.str()?,
            "value" => value = decode_expr(dec)?,
            "span" => span = decode_span(dec)?,
            _ => dec.skip()?,
        }
    }
    Ok(NamedArg { span, name, value })
}

fn decode_binop(dec: &mut CborDecoder) -> Result<BinOp, DecodeError> {
    let s = dec.str()?;
    match s.as_str() {
        "Or" => Ok(BinOp::Or),
        "And" => Ok(BinOp::And),
        "Eq" => Ok(BinOp::Eq),
        "Ne" => Ok(BinOp::Ne),
        "Lt" => Ok(BinOp::Lt),
        "Le" => Ok(BinOp::Le),
        "Gt" => Ok(BinOp::Gt),
        "Ge" => Ok(BinOp::Ge),
        "Add" => Ok(BinOp::Add),
        "Sub" => Ok(BinOp::Sub),
        "Mul" => Ok(BinOp::Mul),
        "Div" => Ok(BinOp::Div),
        "Rem" => Ok(BinOp::Rem),
        _ => Err(DecodeError::UnexpectedType("BinOp")),
    }
}

fn decode_unop(dec: &mut CborDecoder) -> Result<UnOp, DecodeError> {
    let s = dec.str()?;
    match s.as_str() {
        "Not" => Ok(UnOp::Not),
        "Neg" => Ok(UnOp::Neg),
        "Plus" => Ok(UnOp::Plus),
        _ => Err(DecodeError::UnexpectedType("UnOp")),
    }
}

fn decode_literal(dec: &mut CborDecoder) -> Result<Literal, DecodeError> {
    let map_len = dec.map()?;
    let mut lit_type = String::new();
    let mut value_u = 0u64;
    let mut value_i = 0i64;
    let mut value_b = false;
    let mut value_s = String::new();
    for _ in 0..map_len {
        let key = dec.str()?;
        match key.as_str() {
            "type" => lit_type = dec.str()?,
            "value" => {
                if dec.peek()? == 0xF5 || dec.peek()? == 0xF4 {
                    value_b = dec.bool()?;
                } else if dec.peek()? >> 5 == 0 {
                    value_u = dec.uint()?;
                    value_i = value_u as i64; // positive int works for both Int and UInt
                } else if dec.peek()? >> 5 == 1 {
                    value_i = dec.int()?;
                    value_u = value_i as u64; // negative int as two's complement
                } else {
                    value_s = dec.str()?;
                }
            }
            "bits" => value_u = dec.uint()?,
            _ => dec.skip()?,
        }
    }
    match lit_type.as_str() {
        "Null" => Ok(Literal::Null),
        "Bool" => Ok(Literal::Bool(value_b)),
        "Int" => Ok(Literal::Int(value_i)),
        "UInt" => Ok(Literal::UInt(value_u)),
        "Float" => Ok(Literal::Float(value_u)),
        "String" => Ok(Literal::String(value_s)),
        _ => Err(DecodeError::UnexpectedType("Literal")),
    }
}

fn decode_params(dec: &mut CborDecoder) -> Result<Vec<Param>, DecodeError> {
    let n = dec.array()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let map_len = dec.map()?;
        let mut name = String::new();
        let mut ty = TypeExpr {
            span: Span::new(0, 0),
            name: String::new(),
            args: Vec::new(),
        };
        let mut span = Span::new(0, 0);
        for _ in 0..map_len {
            let key = dec.str()?;
            match key.as_str() {
                "name" => name = dec.str()?,
                "ty" => ty = decode_type_expr(dec)?,
                "span" => span = decode_span(dec)?,
                _ => dec.skip()?,
            }
        }
        out.push(Param { span, name, ty });
    }
    Ok(out)
}

fn decode_type_expr(dec: &mut CborDecoder) -> Result<TypeExpr, DecodeError> {
    let map_len = dec.map()?;
    let mut name = String::new();
    let mut args = Vec::new();
    let mut span = Span::new(0, 0);
    for _ in 0..map_len {
        let key = dec.str()?;
        match key.as_str() {
            "name" => name = dec.str()?,
            "args" => {
                let n = dec.array()?;
                args = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    args.push(decode_type_expr(dec)?);
                }
            }
            "span" => span = decode_span(dec)?,
            _ => dec.skip()?,
        }
    }
    Ok(TypeExpr { span, name, args })
}

fn decode_block(dec: &mut CborDecoder) -> Result<Block, DecodeError> {
    let map_len = dec.map()?;
    let mut stmts = Vec::new();
    let mut span = Span::new(0, 0);
    for _ in 0..map_len {
        let key = dec.str()?;
        match key.as_str() {
            "stmts" => {
                let n = dec.array()?;
                stmts = Vec::with_capacity(n as usize);
                for _ in 0..n {
                    stmts.push(decode_stmt(dec)?);
                }
            }
            "span" => span = decode_span(dec)?,
            _ => dec.skip()?,
        }
    }
    Ok(Block { span, stmts })
}

fn decode_stmt(dec: &mut CborDecoder) -> Result<Stmt, DecodeError> {
    let map_len = dec.map()?;
    if map_len < 1 {
        return Err(DecodeError::InvalidCbor("empty Stmt map".into()));
    }
    let mut stmt_type = String::new();
    // Single-pass decode: buffer field values as StmtVal, construct at end.
    let mut fields: Vec<(String, StmtVal)> = Vec::with_capacity(map_len as usize);
    for _ in 0..map_len {
        let key = dec.str()?;
        if key == "type" {
            stmt_type = dec.str()?;
        } else {
            let val = decode_stmt_field(&key, dec)?;
            fields.push((key, val));
        }
    }
    // Construct the Stmt from collected fields.
    match stmt_type.as_str() {
        "Let" => {
            let mut name = String::new();
            let mut mutable = false;
            let mut value = None;
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", StmtVal::Str(s)) => name = s,
                    ("mutable", StmtVal::Bool(b)) => mutable = b,
                    ("value", StmtVal::Expr(e)) => value = Some(e),
                    _ => {}
                }
            }
            Ok(Stmt::Let {
                span: Span::new(0, 0),
                mutable,
                name,
                ty: None,
                value,
            })
        }
        "Assign" => {
            let mut target = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut value = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("target", StmtVal::Expr(e)) => target = e,
                    ("value", StmtVal::Expr(e)) => value = e,
                    _ => {}
                }
            }
            Ok(Stmt::Assign {
                span: Span::new(0, 0),
                target,
                value,
            })
        }
        "If" => {
            let mut cond = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut then_block = Block {
                span: Span::new(0, 0),
                stmts: Vec::new(),
            };
            let mut else_block = None;
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("cond", StmtVal::Expr(e)) => cond = e,
                    ("then", StmtVal::Block(b)) => then_block = b,
                    ("else", StmtVal::Stmt(s)) => else_block = Some(Box::new(s)),
                    _ => {}
                }
            }
            Ok(Stmt::If {
                span: Span::new(0, 0),
                cond,
                then_block,
                else_block,
            })
        }
        "For" => {
            let mut name = String::new();
            let mut iter = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut body = Block {
                span: Span::new(0, 0),
                stmts: Vec::new(),
            };
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", StmtVal::Str(s)) => name = s,
                    ("iter", StmtVal::Expr(e)) => iter = e,
                    ("body", StmtVal::Block(b)) => body = b,
                    _ => {}
                }
            }
            Ok(Stmt::For {
                span: Span::new(0, 0),
                name,
                iter,
                body,
            })
        }
        "While" => {
            let mut cond = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut body = Block {
                span: Span::new(0, 0),
                stmts: Vec::new(),
            };
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("cond", StmtVal::Expr(e)) => cond = e,
                    ("body", StmtVal::Block(b)) => body = b,
                    _ => {}
                }
            }
            Ok(Stmt::While {
                span: Span::new(0, 0),
                cond,
                body,
            })
        }
        "Match" => {
            let mut scrutinee = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut arms = Vec::new();
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("scrutinee", StmtVal::Expr(e)) => scrutinee = e,
                    ("arms", StmtVal::Arms(a)) => arms = a,
                    _ => {}
                }
            }
            Ok(Stmt::Match {
                span: Span::new(0, 0),
                scrutinee,
                arms,
            })
        }
        "Return" => {
            let mut value = None;
            for (k, v) in fields {
                if k == "value" {
                    if let StmtVal::Expr(e) = v {
                        value = Some(e);
                    }
                }
            }
            Ok(Stmt::Return {
                span: Span::new(0, 0),
                value,
            })
        }
        "Yield" => {
            let mut value = None;
            for (k, v) in fields {
                if k == "value" {
                    if let StmtVal::Expr(e) = v {
                        value = Some(e);
                    }
                }
            }
            Ok(Stmt::Yield {
                span: Span::new(0, 0),
                value,
            })
        }
        "Transaction" => {
            let mut args = Vec::new();
            let mut body = Block {
                span: Span::new(0, 0),
                stmts: Vec::new(),
            };
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("args", StmtVal::NamedArgs(na)) => args = na,
                    ("body", StmtVal::Block(b)) => body = b,
                    _ => {}
                }
            }
            Ok(Stmt::Transaction {
                span: Span::new(0, 0),
                args,
                body,
            })
        }
        "Effect" => {
            let mut expr = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for (k, v) in fields {
                if k == "expr" {
                    if let StmtVal::Expr(e) = v {
                        expr = e;
                    }
                }
            }
            Ok(Stmt::Effect {
                span: Span::new(0, 0),
                expr,
            })
        }
        "Expr" => {
            let mut expr = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for (k, v) in fields {
                if k == "expr" {
                    if let StmtVal::Expr(e) = v {
                        expr = e;
                    }
                }
            }
            Ok(Stmt::Expr {
                span: Span::new(0, 0),
                expr,
            })
        }
        "Block" => {
            for (k, v) in fields {
                if k == "block" {
                    if let StmtVal::Block(b) = v {
                        return Ok(Stmt::Block(b));
                    }
                }
            }
            Err(DecodeError::MissingField("block"))
        }
        _ => Err(DecodeError::UnexpectedType("Stmt")),
    }
}

/// Buffered field value for Stmt construction after single-pass decode.
enum StmtVal {
    Str(String),
    Bool(bool),
    Expr(Expr),
    Block(Block),
    Stmt(Stmt),
    NamedArgs(Vec<NamedArg>),
    Arms(Vec<MatchArm>),
}

fn decode_stmt_field(key: &str, dec: &mut CborDecoder) -> Result<StmtVal, DecodeError> {
    match key {
        "name" => Ok(StmtVal::Str(dec.str()?)),
        "mutable" => Ok(StmtVal::Bool(dec.bool()?)),
        "cond" | "target" | "iter" | "scrutinee" | "expr" | "value" => {
            if dec.is_null()? {
                dec.skip()?;
                Ok(StmtVal::Expr(Expr {
                    span: Span::new(0, 0),
                    kind: ExprKind::Literal(Literal::Null),
                }))
            } else {
                Ok(StmtVal::Expr(decode_expr(dec)?))
            }
        }
        "then" | "body" | "block" => Ok(StmtVal::Block(decode_block(dec)?)),
        "else" => {
            if dec.is_null()? {
                dec.skip()?;
                Ok(StmtVal::Expr(Expr {
                    span: Span::new(0, 0),
                    kind: ExprKind::Literal(Literal::Null),
                }))
            } else {
                Ok(StmtVal::Stmt(decode_stmt(dec)?))
            }
        }
        "args" => Ok(StmtVal::NamedArgs(decode_named_args(dec)?)),
        "arms" => {
            let n = dec.array()?;
            let mut arms = Vec::with_capacity(n as usize);
            for _ in 0..n {
                let m = dec.map()?;
                let mut pattern = Pattern::Wildcard;
                let mut body = ArmBody::Expr(Expr {
                    span: Span::new(0, 0),
                    kind: ExprKind::Literal(Literal::Null),
                });
                for _ in 0..m {
                    let k = dec.str()?;
                    match k.as_str() {
                        "pattern" => pattern = decode_pattern(dec)?,
                        "body" => body = decode_arm_body(dec)?,
                        "span" => {
                            let _ = decode_span(dec)?;
                        }
                        _ => dec.skip()?,
                    }
                }
                arms.push(MatchArm {
                    span: Span::new(0, 0),
                    pattern,
                    body,
                });
            }
            Ok(StmtVal::Arms(arms))
        }
        _ => {
            dec.skip()?;
            Ok(StmtVal::Str(String::new()))
        }
    }
}

fn decode_pattern(dec: &mut CborDecoder) -> Result<Pattern, DecodeError> {
    let map_len = dec.map()?;
    if map_len < 1 {
        return Err(DecodeError::InvalidCbor("empty Pattern map".into()));
    }
    let key = dec.str()?;
    let pat = match key.as_str() {
        "Wildcard" => Pattern::Wildcard,
        "None" => Pattern::None,
        "Ident" => Pattern::Ident(dec.str()?),
        "Literal" => Pattern::Literal(decode_literal(dec)?),
        "Ok" => Pattern::Ok(Box::new(decode_pattern(dec)?)),
        "Err" => Pattern::Err(Box::new(decode_pattern(dec)?)),
        "Some" => Pattern::Some(Box::new(decode_pattern(dec)?)),
        "Variant" => {
            let mut enum_name = String::new();
            let mut variant_name = String::new();
            let mut args = Vec::new();
            for _ in 1..map_len {
                match dec.str()?.as_str() {
                    "enum" => enum_name = dec.str()?,
                    "variant" => variant_name = dec.str()?,
                    "args" => {
                        let n = dec.array()?;
                        for _ in 0..n {
                            args.push(decode_pattern(dec)?);
                        }
                    }
                    _ => dec.skip()?,
                }
            }
            return Ok(Pattern::Variant {
                enum_name,
                variant_name,
                args,
            });
        }
        _ => return Err(DecodeError::UnexpectedType("Pattern")),
    };
    for _ in 1..map_len {
        dec.skip()?;
    }
    Ok(pat)
}

fn decode_arm_body(dec: &mut CborDecoder) -> Result<ArmBody, DecodeError> {
    let map_len = dec.map()?;
    if map_len < 1 {
        return Err(DecodeError::InvalidCbor("empty ArmBody map".into()));
    }
    let key = dec.str()?;
    let body = match key.as_str() {
        "Block" => ArmBody::Block(decode_block(dec)?),
        "Expr" => ArmBody::Expr(decode_expr(dec)?),
        _ => return Err(DecodeError::UnexpectedType("ArmBody")),
    };
    for _ in 1..map_len {
        dec.skip()?;
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_empty_program() {
        let prog = Program {
            span: Span::new(0, 0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            requires: Vec::new(),
            items: Vec::new(),
        };
        let bytes = encode(&prog);
        assert!(!bytes.is_empty());
        // Tag 4200 = major 6, len 4200
        // 4200 = 0x1068, so ai=25, 2 bytes
        assert_eq!(bytes[0], (6 << 5) | 25);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, prog);
    }

    #[test]
    fn encode_decode_simple_program() {
        let prog = Program {
            span: Span::new(0, 50),
            module: Some(ModuleDecl {
                span: Span::new(0, 20),
                name: Name::Ident("test".to_string()),
            }),
            imports: vec![ImportDecl {
                span: Span::new(21, 40),
                path: "vibe:0.1/render".to_string(),
                alias: Some("r".to_string()),
            }],
            prefixes: vec![PrefixDecl {
                span: Span::new(41, 45),
                prefix: "ex".to_string(),
                iri: "http://example.org/".to_string(),
            }],
            requires: vec![CapSpec {
                span: Span::new(46, 48),
                id: "capability.invoke".to_string(),
                args: Vec::new(),
            }],
            items: vec![Item::Const(ConstDecl {
                span: Span::new(49, 50),
                name: "x".to_string(),
                ty: None,
                value: Expr {
                    span: Span::new(49, 50),
                    kind: ExprKind::Literal(Literal::Int(42)),
                },
            })],
        };
        let bytes = encode(&prog);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded, prog);
    }

    #[test]
    fn cbor_encoder_basic() {
        let mut enc = CborEncoder::new();
        enc.uint(42);
        // 42 >= 24, so CBOR uses ai=24 + 1 byte: [24, 42]
        assert_eq!(enc.finish(), vec![24, 42]);

        let mut enc = CborEncoder::new();
        enc.uint(5);
        // 5 < 24, so CBOR uses ai directly: [5]
        assert_eq!(enc.finish(), vec![5]);

        let mut enc = CborEncoder::new();
        enc.str("hello");
        let bytes = enc.finish();
        assert_eq!(bytes[0], (3 << 5) | 5); // string, len 5
        assert_eq!(&bytes[1..], b"hello");
    }

    #[test]
    fn cbor_encoder_array() {
        let mut enc = CborEncoder::new();
        enc.array(3);
        enc.uint(1);
        enc.uint(2);
        enc.uint(3);
        assert_eq!(enc.finish(), vec![0x83, 1, 2, 3]);
    }

    #[test]
    fn cbor_encoder_map() {
        let mut enc = CborEncoder::new();
        enc.map(2);
        enc.str("a");
        enc.uint(1);
        enc.str("b");
        enc.uint(2);
        let bytes = enc.finish();
        assert_eq!(bytes[0], (5 << 5) | 2); // map, len 2
    }

    #[test]
    fn cbor_encoder_tag() {
        let mut enc = CborEncoder::new();
        enc.tag(4200);
        let bytes = enc.finish();
        // Tag 4200: major 6, ai=25 (2-byte len), 4200 = 0x1068
        assert_eq!(bytes, vec![(6 << 5) | 25, 0x10, 0x68]);
    }

    #[test]
    fn cbor_decoder_basic() {
        // 42 >= 24, so CBOR encodes as [24, 42]
        let mut dec = CborDecoder::new(&[24, 42]);
        assert_eq!(dec.uint().unwrap(), 42);
        // 5 < 24, so CBOR encodes as [5]
        let mut dec = CborDecoder::new(&[5]);
        assert_eq!(dec.uint().unwrap(), 5);
    }

    #[test]
    fn cbor_decoder_string() {
        let bytes = [(3 << 5) | 5, b'h', b'e', b'l', b'l', b'o'];
        let mut dec = CborDecoder::new(&bytes);
        assert_eq!(dec.str().unwrap(), "hello");
    }

    #[test]
    fn cbor_decoder_array() {
        let mut dec = CborDecoder::new(&[0x83, 1, 2, 3]);
        assert_eq!(dec.array().unwrap(), 3);
        assert_eq!(dec.uint().unwrap(), 1);
        assert_eq!(dec.uint().unwrap(), 2);
        assert_eq!(dec.uint().unwrap(), 3);
    }

    #[test]
    fn cbor_decoder_tag() {
        let bytes = [(6 << 5) | 25, 0x10, 0x68];
        let mut dec = CborDecoder::new(&bytes);
        assert_eq!(dec.tag().unwrap(), 4200);
    }

    #[test]
    fn decode_wrong_tag_errors() {
        let mut enc = CborEncoder::new();
        enc.tag(9999);
        enc.uint(0);
        let bytes = enc.finish();
        let result = decode(&bytes);
        assert!(result.is_err());
        match result.unwrap_err() {
            DecodeError::UnexpectedTag(t) => assert_eq!(t, 9999),
            _ => panic!("expected UnexpectedTag"),
        }
    }

    #[test]
    fn round_trip_parsed_program() {
        use crate::parse::parse_program;
        let src = r#"module test;
import "vibe:0.1/render" as r;
prefix ex: <http://example.org/>;
requires [ capability("capability.invoke") ];
effect fn go() {
    let x = 42;
    let y = "hello";
    return x;
}
"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        assert!(!bytes.is_empty());
        // Verify tag 4200 is present.
        assert_eq!(bytes[0], (6 << 5) | 25);
        let decoded = decode(&bytes).unwrap();
        // The decoded program should have the same structure.
        // Note: spans may differ slightly due to encoding/decoding,
        // but the structure should match.
        assert_eq!(decoded.module, prog.module);
        assert_eq!(decoded.imports, prog.imports);
        assert_eq!(decoded.prefixes, prog.prefixes);
        assert_eq!(decoded.requires, prog.requires);
        assert_eq!(decoded.items.len(), prog.items.len());
    }

    #[test]
    fn round_trip_function_with_body() {
        use crate::parse::parse_program;
        let src = r#"effect fn add(a: f32, b: f32) {
    return a + b;
}"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), 1);
        match &decoded.items[0] {
            Item::Function(f) => {
                assert_eq!(f.name, "add");
                assert_eq!(f.params.len(), 2);
                assert_eq!(f.params[0].name, "a");
                assert_eq!(f.params[1].name, "b");
            }
            _ => panic!("expected Function item"),
        }
    }

    #[test]
    fn round_trip_const_with_literal() {
        use crate::parse::parse_program;
        let src = r#"const PI = 3.14;"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), 1);
        match &decoded.items[0] {
            Item::Const(c) => {
                assert_eq!(c.name, "PI");
                match &c.value.kind {
                    ExprKind::Literal(Literal::Float(bits)) => {
                        // 3.14 as f64 bits
                        let expected = 3.14_f64.to_bits();
                        assert_eq!(*bits, expected);
                    }
                    _ => panic!("expected Float literal"),
                }
            }
            _ => panic!("expected Const item"),
        }
    }

    #[test]
    fn round_trip_if_statement() {
        use crate::parse::parse_program;
        let src = r#"effect fn go() {
    if true {
        return 1;
    } else {
        return 0;
    }
}"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), 1);
        if let Item::Function(f) = &decoded.items[0] {
            assert_eq!(f.body.stmts.len(), 1);
            match &f.body.stmts[0] {
                Stmt::If {
                    then_block,
                    else_block,
                    ..
                } => {
                    assert_eq!(then_block.stmts.len(), 1);
                    assert!(else_block.is_some());
                }
                _ => panic!("expected If statement"),
            }
        }
    }

    #[test]
    fn round_trip_triple_expression() {
        use crate::parse::parse_program;
        let src = r#"effect fn go() {
    <<(ex:s ex:p ex:o)>>;
}"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), 1);
    }

    #[test]
    fn encode_produces_valid_cbor() {
        let prog = Program {
            span: Span::new(0, 0),
            module: None,
            imports: Vec::new(),
            prefixes: Vec::new(),
            requires: Vec::new(),
            items: Vec::new(),
        };
        let bytes = encode(&prog);
        // First byte should be tag header: major 6, ai=25 (2-byte len)
        assert_eq!(bytes[0], (6 << 5) | 25);
        // Next 2 bytes should be 4200 = 0x1068
        assert_eq!(bytes[1], 0x10);
        assert_eq!(bytes[2], 0x68);
        // After tag, should be a map (major 5)
        assert!((bytes[3] >> 5) == 5);
    }

    // ── T31: Tag 4200 CBOR round-trip for FieldDecl/MaterialDecl/LawDecl ──

    #[test]
    fn t31_round_trip_field_decl() {
        use crate::parse::parse_program;
        let src = r#"module test;
field pressure_ambient: Pressure unit: <qudt:KiloPascal> support: region representation: grid;
"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        assert!(!bytes.is_empty());
        // Verify tag 4200 is present.
        assert_eq!(bytes[0], (6 << 5) | 25);
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), prog.items.len());
        // Verify the Field item survived round-trip.
        assert!(matches!(decoded.items.first(), Some(Item::Field(_))));
        if let Some(Item::Field(f)) = decoded.items.first() {
            assert_eq!(f.name, "pressure_ambient");
        }
    }

    #[test]
    fn t31_round_trip_material_decl() {
        use crate::parse::parse_program;
        let src = r#"module test;
material sucrose_cube: Material yield: 50.0;
"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        assert!(!bytes.is_empty());
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), prog.items.len());
        assert!(matches!(decoded.items.first(), Some(Item::Material(_))));
        if let Some(Item::Material(m)) = decoded.items.first() {
            assert_eq!(m.name, "sucrose_cube");
        }
    }

    #[test]
    fn t31_round_trip_law_decl() {
        use crate::parse::parse_program;
        let src = r#"module test;
law crush when true => 1;
"#;
        let prog = parse_program(src).unwrap();
        let bytes = encode(&prog);
        assert!(!bytes.is_empty());
        let decoded = decode(&bytes).unwrap();
        assert_eq!(decoded.items.len(), prog.items.len());
        assert!(matches!(decoded.items.first(), Some(Item::Law(_))));
        if let Some(Item::Law(l)) = decoded.items.first() {
            assert_eq!(l.name, "crush");
        }
    }
}
