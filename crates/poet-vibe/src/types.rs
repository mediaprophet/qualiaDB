//! Type names for vibe-0.1.

use crate::ast::TypeExpr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    Bool,
    String,
    Bytes,
    Iri,
    BlankNode,
    Did,
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
            "i32" => Type::I32,
            "u32" => Type::U32,
            "i64" => Type::I64,
            "f32" => Type::F32,
            "f64" => Type::F64,
            "u64" => Type::U64,
            "bool" => Type::Bool,
            "string" => Type::String,
            "bytes" => Type::Bytes,
            "Iri" => Type::Iri,
            "BlankNode" => Type::BlankNode,
            "Did" => Type::Did,
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

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            Type::I32 | Type::U32 | Type::I64 | Type::U64 | Type::F32 | Type::F64
        )
    }
}
