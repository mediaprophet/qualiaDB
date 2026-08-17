//! Runtime values.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    String(String),
    Iri(String),
    Blank(String),
    Prefixed(String, String),
    Var(String),
    List(Vec<Value>),
    Record(BTreeMap<String, Value>),
    Triple(Box<Value>, Box<Value>, Box<Value>),
    Reified {
        s: Box<Value>,
        p: Box<Value>,
        o: Box<Value>,
        r: Box<Value>,
    },
    Quin {
        subject: u64,
        predicate: u64,
        object: u64,
        context: u64,
    },
    Receipt,
    Ok(Box<Value>),
    Err(Box<Value>),
    /// Temporary namespace/ctor name during postfix eval.
    Identish(String),
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::Null => false,
            Value::I64(0) | Value::U64(0) => false,
            _ => true,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::I64(n) => Some(*n as f64),
            Value::U64(n) => Some(*n as f64),
            Value::F64(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::I64(n) => Some(*n),
            Value::U64(n) if *n <= i64::MAX as u64 => Some(*n as i64),
            Value::F64(n) if n.fract() == 0.0 => Some(*n as i64),
            _ => None,
        }
    }
}
