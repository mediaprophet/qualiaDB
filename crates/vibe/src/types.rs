//! Type names for vibe-0.1.

use crate::ast::TypeExpr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    I64,
    U64,
    F64,
    Bool,
    String,
    Bytes,
    Iri,
    BlankNode,
    Did,
    /// A continuant — a thing that persists through time (T70).
    /// Distinct from `Did` which is just an identifier. A `Did`
    /// *refers to* a `Continuant`; a `Continuant` *has a* `Did`.
    /// Continuants have worldlines and endure through time.
    Continuant,
    Hash,
    Var,
    Literal,
    TripleTerm,
    Reifier,
    Quin,
    QuinRef,
    AssetRef,
    TensorRef,
    GeometryRef,
    // ── T1–T6: Type lattice extensions ────────────────────────────────
    Instant,
    Duration,
    Quantity,
    Frame,
    Pose,
    Transform,
    FieldRef,
    MaterialRef,
    WorldLine,
    // ── T9: User-defined enum type ─────────────────────────────────────
    #[allow(dead_code)]
    Enum(String),
    // ── T33: Species and Mixture types ─────────────────────────────────
    SpeciesRef,
    Mixture,
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    List(Box<Type>),
    Record,
    Receipt,
    Stream(Box<Type>),
    Future(Box<Type>),
    Named(String),
    Unknown,
}

impl Type {
    pub fn from_ast(t: &TypeExpr) -> Type {
        match t.name.as_str() {
            "i32" => Type::I64,
            "u32" => Type::U64,
            "i64" => Type::I64,
            "f32" => Type::F64,
            "f64" => Type::F64,
            "u64" => Type::U64,
            "bool" => Type::Bool,
            "string" => Type::String,
            "bytes" => Type::Bytes,
            "Iri" => Type::Iri,
            "BlankNode" => Type::BlankNode,
            "Did" => Type::Did,
            "Continuant" => Type::Continuant,
            "Hash" => Type::Hash,
            "Var" => Type::Var,
            "Literal" => Type::Literal,
            "TripleTerm" => Type::TripleTerm,
            "Reifier" => Type::Reifier,
            "Quin" => Type::Quin,
            "QuinRef" => Type::QuinRef,
            "AssetRef" => Type::AssetRef,
            "TensorRef" => Type::TensorRef,
            "GeometryRef" => Type::GeometryRef,
            "Instant" => Type::Instant,
            "Duration" => Type::Duration,
            "Quantity" => Type::Quantity,
            "Frame" => Type::Frame,
            "Pose" => Type::Pose,
            "Transform" => Type::Transform,
            "FieldRef" => Type::FieldRef,
            "MaterialRef" => Type::MaterialRef,
            "WorldLine" => Type::WorldLine,
            "SpeciesRef" => Type::SpeciesRef,
            "Mixture" => Type::Mixture,
            "Record" => Type::Record,
            "Receipt" => Type::Receipt,
            "Option" => Type::Option(Box::new(
                t.args.first().map(Type::from_ast).unwrap_or(Type::Unknown),
            )),
            "List" => Type::List(Box::new(
                t.args.first().map(Type::from_ast).unwrap_or(Type::Unknown),
            )),
            "Stream" => Type::Stream(Box::new(
                t.args.first().map(Type::from_ast).unwrap_or(Type::Unknown),
            )),
            "Future" => Type::Future(Box::new(
                t.args.first().map(Type::from_ast).unwrap_or(Type::Unknown),
            )),
            "Result" => Type::Result(
                Box::new(t.args.first().map(Type::from_ast).unwrap_or(Type::Unknown)),
                Box::new(t.args.get(1).map(Type::from_ast).unwrap_or(Type::Unknown)),
            ),
            other => Type::Named(other.to_string()),
        }
    }

    #[allow(dead_code)]
    pub fn is_numeric(&self) -> bool {
        matches!(self, Type::I64 | Type::U64 | Type::F64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tx(name: &str) -> TypeExpr {
        TypeExpr {
            span: crate::span::Span::point(0),
            name: name.to_string(),
            args: vec![],
        }
    }

    #[test]
    fn from_ast_i32_maps_to_i64() {
        assert_eq!(Type::from_ast(&tx("i32")), Type::I64);
    }

    #[test]
    fn from_ast_u32_maps_to_u64() {
        assert_eq!(Type::from_ast(&tx("u32")), Type::U64);
    }

    #[test]
    fn from_ast_f32_maps_to_f64() {
        assert_eq!(Type::from_ast(&tx("f32")), Type::F64);
    }

    // ── T70: Continuant distinct from Did ─────────────────────────────

    #[test]
    fn continuant_type_exists() {
        assert!(matches!(Type::Continuant, Type::Continuant));
    }

    #[test]
    fn from_ast_continuant() {
        assert_eq!(Type::from_ast(&tx("Continuant")), Type::Continuant);
    }

    #[test]
    fn continuant_distinct_from_did() {
        assert_ne!(Type::Continuant, Type::Did);
    }
}
