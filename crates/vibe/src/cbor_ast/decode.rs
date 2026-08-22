//! Decode CBOR-LD to Program.

use super::codec::CborDecoder;
use super::{DecodeError, TAG_VIBE_AST};
use crate::ast::*;
use crate::span::Span;

pub fn decode(bytes: &[u8]) -> Result<Program, DecodeError> {
    let mut dec = CborDecoder::new(bytes);
    let tag = dec.tag()?;
    if tag != TAG_VIBE_AST {
        return Err(DecodeError::UnexpectedTag(tag));
    }
    decode_program(&mut dec)
}

// â”€â”€ Encode â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€â”€
fn decode_program(dec: &mut CborDecoder) -> Result<Program, DecodeError> {
    let map_len = dec.map()?;
    let mut module = None;
    let mut imports = Vec::new();
    let mut prefixes = Vec::new();
    let mut locales = Vec::new();
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
            "locales" => locales = decode_locales(dec)?,
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
        locales,
        requires,
        items,
    })
}

fn decode_locales(dec: &mut CborDecoder) -> Result<Vec<LocaleDecl>, DecodeError> {
    let n = dec.array()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        let map_len = dec.map()?;
        let mut code = String::new();
        let mut span = Span::new(0, 0);
        for _ in 0..map_len {
            let key = dec.str()?;
            match key.as_str() {
                "code" => code = dec.str()?,
                "span" => span = decode_span(dec)?,
                _ => dec.skip()?,
            }
        }
        out.push(LocaleDecl { span, code });
    }
    Ok(out)
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

fn decode_str_list(dec: &mut CborDecoder) -> Result<Vec<String>, DecodeError> {
    let n = dec.array()?;
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        out.push(dec.str()?);
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
            let mut ret = None;
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
                    ("ret", ItemField::TypeExpr(t)) => ret = t,
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
                ret,
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
                    ("ty", ItemField::TypeExpr(t)) => {
                        if let Some(t) = t {
                            ty = t;
                        }
                    }
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
        "Cell" => {
            let mut name = String::new();
            let mut span = Span::new(0, 0);
            let mut effect = None;
            let mut params = Vec::new();
            let mut expr = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut when = None;
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", ItemField::Str(s)) => name = s,
                    ("span", ItemField::Span(sp)) => span = sp,
                    ("effect", ItemField::Effect(eff)) => effect = eff,
                    ("params", ItemField::Params(ps)) => params = ps,
                    ("expr", ItemField::Expr(e)) => expr = e,
                    ("when", ItemField::Expr(e)) => when = Some(e),
                    _ => {}
                }
            }
            Ok(Item::Cell(CellDecl {
                span,
                effect,
                name,
                params,
                expr,
                when,
            }))
        }
        "Present" => {
            let mut name = String::new();
            let mut properties = Vec::new();
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", ItemField::Str(s)) => name = s,
                    ("properties", ItemField::NamedArgs(ps)) => properties = ps,
                    _ => {}
                }
            }
            Ok(Item::Present(crate::ast::PresentDecl {
                span: Span::new(0, 0),
                name,
                properties,
            }))
        }
        "Bind" => {
            let mut left = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut right = left.clone();
            let mut resolve = crate::ast::BindResolve::Latest;
            let mut clamp_lo = None;
            let mut clamp_hi = None;
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("left", ItemField::Expr(e)) => left = e,
                    ("right", ItemField::Expr(e)) => right = e,
                    ("resolve", ItemField::Str(s)) => {
                        resolve = match s.as_str() {
                            "left" => crate::ast::BindResolve::Left,
                            "right" => crate::ast::BindResolve::Right,
                            _ => crate::ast::BindResolve::Latest,
                        };
                    }
                    ("clamp_lo", ItemField::Expr(e)) => clamp_lo = Some(e),
                    ("clamp_hi", ItemField::Expr(e)) => clamp_hi = Some(e),
                    _ => {}
                }
            }
            let clamp = match (clamp_lo, clamp_hi) {
                (Some(lo), Some(hi)) => Some((lo, hi)),
                _ => None,
            };
            Ok(Item::Bind(crate::ast::BindDecl {
                span: Span::new(0, 0),
                left,
                right,
                clamp,
                resolve,
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
    TypeExpr(Option<TypeExpr>),
}

fn decode_item_field(key: &str, dec: &mut CborDecoder) -> Result<ItemField, DecodeError> {
    match key {
        "name" => Ok(ItemField::Str(dec.str()?)),
        "path" => {
            if (dec.peek()? >> 5) == 4 {
                let n = dec.array()?;
                let mut parts = Vec::new();
                for _ in 0..n {
                    parts.push(dec.str()?);
                }
                Ok(ItemField::StrList(parts))
            } else {
                Ok(ItemField::Str(dec.str()?))
            }
        }
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
        "ret" | "ty" => {
            if dec.is_null()? {
                dec.skip()?;
                Ok(ItemField::TypeExpr(None))
            } else {
                Ok(ItemField::TypeExpr(Some(decode_type_expr(dec)?)))
            }
        }
        "value" | "expr" | "condition" | "consequence" | "when" => {
            if dec.is_null()? {
                dec.skip()?;
                Ok(ItemField::Expr(Expr {
                    span: Span::new(0, 0),
                    kind: ExprKind::Literal(Literal::Null),
                }))
            } else {
                Ok(ItemField::Expr(decode_expr(dec)?))
            }
        }
        "body" | "then" | "block" => Ok(ItemField::Block(decode_block(dec)?)),
        "params" => Ok(ItemField::Params(decode_params(dec)?)),
        "properties" => Ok(ItemField::NamedArgs(decode_named_args(dec)?)),
        "budget" | "args" => {
            // "args" in Item context could be NamedArgs or Args â€” for Function/Hook
            // budget it's NamedArgs. For Call args it's Args. But we're in decode_item_field,
            // so this is for Item-level fields only.
            if key == "budget" {
                Ok(ItemField::NamedArgs(decode_named_args(dec)?))
            } else {
                // Skip unknown "args" â€” will be handled by Stmt decoding
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
        "unit" => {
            if dec.is_null()? {
                dec.skip()?;
                Ok(ItemField::Str(String::new()))
            } else {
                Ok(ItemField::Str(dec.str()?))
            }
        }
        "support" | "representation" | "resolve" => Ok(ItemField::Str(dec.str()?)),
        "left" | "right" | "clamp_lo" | "clamp_hi" => {
            if dec.is_null()? {
                dec.skip()?;
                Ok(ItemField::Str(String::new()))
            } else {
                Ok(ItemField::Expr(decode_expr(dec)?))
            }
        }
        _ => {
            dec.skip()?;
            Ok(ItemField::Str(String::new()))
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
    // ExprKind is encoded as a single-key map where the key is the variant name,
    // or as {"type": "VariantName", ...}.
    let map_len = dec.map()?;
    if map_len < 1 {
        return Err(DecodeError::InvalidCbor("empty ExprKind map".into()));
    }
    let first_key = dec.str()?;
    let (variant, remaining_pairs) = if first_key == "type" {
        (dec.str()?, map_len - 1)
    } else {
        (first_key, map_len - 1)
    };
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
            for _ in 0..remaining_pairs {
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
            for _ in 0..remaining_pairs {
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
            for _ in 0..remaining_pairs {
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
            for _ in 0..remaining_pairs {
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
            for _ in 0..remaining_pairs {
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
            for _ in 0..remaining_pairs {
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
            for _ in 0..remaining_pairs {
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
            for _ in 0..remaining_pairs {
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
            for _ in 0..remaining_pairs {
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
            for _ in 0..remaining_pairs {
                dec.skip()?;
            }
            ExprKind::Prefixed(prefix, local)
        }
        "Interpolate" => {
            let n = dec.array()?;
            let mut parts = Vec::with_capacity(n as usize);
            for _ in 0..n {
                parts.push(decode_expr(dec)?);
            }
            ExprKind::Interpolate(parts)
        }
        "Lambda" => {
            let mut params = Vec::new();
            let mut body = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for _ in 0..remaining_pairs {
                let k = dec.str()?;
                match k.as_str() {
                    "params" => params = decode_str_list(dec)?,
                    "body" => body = decode_expr(dec)?,
                    _ => dec.skip()?,
                }
            }
            ExprKind::Lambda {
                params,
                body: Box::new(body),
            }
        }
        "Tween" => {
            let mut from = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            let mut to = from.clone();
            let mut over = from.clone();
            let mut ease = None;
            let mut spring = None;
            for _ in 0..remaining_pairs {
                let k = dec.str()?;
                match k.as_str() {
                    "from" => from = decode_expr(dec)?,
                    "to" => to = decode_expr(dec)?,
                    "over" => over = decode_expr(dec)?,
                    "ease" => {
                        if dec.is_null()? {
                            dec.skip()?;
                        } else {
                            ease = Some(dec.str()?);
                        }
                    }
                    "spring" => {
                        if dec.is_null()? {
                            dec.skip()?;
                        } else {
                            spring = Some(decode_named_args(dec)?);
                        }
                    }
                    _ => dec.skip()?,
                }
            }
            ExprKind::Tween {
                from: Box::new(from),
                to: Box::new(to),
                over: Box::new(over),
                ease,
                spring,
            }
        }
        _ => {
            for _ in 0..remaining_pairs {
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
    let mut r = 0u64;
    let mut g = 0u64;
    let mut b = 0u64;
    let mut a = 255u64;
    for _ in 0..map_len {
        let key = dec.str()?;
        match key.as_str() {
            "type" => lit_type = dec.str()?,
            "r" => r = dec.uint()?,
            "g" => g = dec.uint()?,
            "b" => b = dec.uint()?,
            "a" => a = dec.uint()?,
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
        "Color" => Ok(Literal::Color(crate::ast::ColorLit {
            r: r as u8,
            g: g as u8,
            b: b as u8,
            a: a as u8,
        })),
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
            let mut ty = None;
            let mut value = None;
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("name", StmtVal::Str(s)) => name = s,
                    ("mutable", StmtVal::Bool(b)) => mutable = b,
                    ("ty", StmtVal::TypeExpr(t)) => ty = t,
                    ("value", StmtVal::Expr(e)) => value = Some(e),
                    _ => {}
                }
            }
            Ok(Stmt::Let {
                span: Span::new(0, 0),
                mutable,
                name,
                ty,
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
        "LetPat" => {
            let mut pattern = Pattern::Wildcard;
            let mut mutable = false;
            let mut ty = None;
            let mut value = Expr {
                span: Span::new(0, 0),
                kind: ExprKind::Literal(Literal::Null),
            };
            for (k, v) in fields {
                match (k.as_str(), v) {
                    ("pattern", StmtVal::Pattern(p)) => pattern = p,
                    ("mutable", StmtVal::Bool(b)) => mutable = b,
                    ("ty", StmtVal::TypeExpr(t)) => ty = t,
                    ("value", StmtVal::Expr(e)) => value = e,
                    _ => {}
                }
            }
            Ok(Stmt::LetPat {
                span: Span::new(0, 0),
                mutable,
                pattern,
                ty,
                value,
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
    Pattern(Pattern),
    TypeExpr(Option<TypeExpr>),
    NamedArgs(Vec<NamedArg>),
    Arms(Vec<MatchArm>),
}

fn decode_stmt_field(key: &str, dec: &mut CborDecoder) -> Result<StmtVal, DecodeError> {
    match key {
        "name" => Ok(StmtVal::Str(dec.str()?)),
        "mutable" => Ok(StmtVal::Bool(dec.bool()?)),
        "pattern" => Ok(StmtVal::Pattern(decode_pattern(dec)?)),
        "ty" => {
            if dec.is_null()? {
                dec.skip()?;
                Ok(StmtVal::TypeExpr(None))
            } else {
                Ok(StmtVal::TypeExpr(Some(decode_type_expr(dec)?)))
            }
        }
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
    let mut pat_type = String::new();
    let mut name = String::new();
    let mut enum_name = String::new();
    let mut variant_name = String::new();
    let mut lit = None;
    let mut inner = None;
    let mut fields = Vec::new();
    let mut elements = Vec::new();
    let mut args = Vec::new();

    for _ in 0..map_len {
        let key = dec.str()?;
        match key.as_str() {
            "type" => pat_type = dec.str()?,
            "name" => name = dec.str()?,
            "enum" => enum_name = dec.str()?,
            "variant" => variant_name = dec.str()?,
            "lit" => lit = Some(decode_literal(dec)?),
            "inner" => inner = Some(Box::new(decode_pattern(dec)?)),
            "fields" => {
                let n = dec.array()?;
                for _ in 0..n {
                    let field_map = dec.map()?;
                    let mut fname = String::new();
                    let mut fpat = Pattern::Wildcard;
                    for _ in 0..field_map {
                        match dec.str()?.as_str() {
                            "name" => fname = dec.str()?,
                            "pattern" => fpat = decode_pattern(dec)?,
                            _ => dec.skip()?,
                        }
                    }
                    fields.push((fname, fpat));
                }
            }
            "elements" => {
                let n = dec.array()?;
                for _ in 0..n {
                    elements.push(decode_pattern(dec)?);
                }
            }
            "args" => {
                let n = dec.array()?;
                for _ in 0..n {
                    args.push(decode_pattern(dec)?);
                }
            }
            _ => dec.skip()?,
        }
    }

    match pat_type.as_str() {
        "Wildcard" => Ok(Pattern::Wildcard),
        "None" => Ok(Pattern::None),
        "Ident" => Ok(Pattern::Ident(name)),
        "Literal" => Ok(Pattern::Literal(lit.unwrap_or(Literal::Null))),
        "Ok" => Ok(Pattern::Ok(inner.unwrap_or_else(|| Box::new(Pattern::Wildcard)))),
        "Err" => Ok(Pattern::Err(inner.unwrap_or_else(|| Box::new(Pattern::Wildcard)))),
        "Some" => Ok(Pattern::Some(inner.unwrap_or_else(|| Box::new(Pattern::Wildcard)))),
        "Record" => Ok(Pattern::Record(fields)),
        "List" => Ok(Pattern::List(elements)),
        "Constructor" => Ok(Pattern::Constructor { name, args }),
        "Variant" => Ok(Pattern::Variant {
            enum_name,
            variant_name,
            args,
        }),
        _ => Err(DecodeError::UnexpectedType("Pattern")),
    }
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
