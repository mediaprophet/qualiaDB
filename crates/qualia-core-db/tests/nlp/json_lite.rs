//! Minimal JSON value parser for NLP-004 manifests and receipts.
//!
//! Integration tests cannot take `serde_json` without editing Cargo.toml,
//! which is frozen for this lane. This parser covers the manifest subset:
//! objects, arrays, strings, integers, floats, booleans, and null.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
    Array(Vec<Json>),
    Object(BTreeMap<String, Json>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError(pub String);

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for JsonError {}

pub fn parse_json(input: &str) -> Result<Json, JsonError> {
    let mut p = Parser {
        bytes: input.as_bytes(),
        i: 0,
    };
    p.skip_ws();
    let value = p.parse_value()?;
    p.skip_ws();
    if p.i != p.bytes.len() {
        return Err(p.err("trailing content after top-level JSON value"));
    }
    Ok(value)
}

impl Json {
    pub fn as_object(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Json::Object(m) => Some(m),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Json]> {
        match self {
            Json::Array(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Json::Int(n) => Some(*n),
            Json::Float(x)
                if x.fract() == 0.0 && *x >= i64::MIN as f64 && *x <= i64::MAX as f64 =>
            {
                Some(*x as i64)
            }
            _ => None,
        }
    }

    pub fn field<'a>(&'a self, key: &str) -> Option<&'a Json> {
        self.as_object()?.get(key)
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Json::Null)
    }
}

struct Parser<'a> {
    bytes: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn err(&self, msg: &str) -> JsonError {
        JsonError(format!("{msg} at byte {}", self.i))
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.i += 1;
        Some(b)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.i += 1;
        }
    }

    fn parse_value(&mut self) -> Result<Json, JsonError> {
        self.skip_ws();
        match self.peek() {
            Some(b'n') => self.parse_ident("null").map(|()| Json::Null),
            Some(b't') => self.parse_ident("true").map(|()| Json::Bool(true)),
            Some(b'f') => self.parse_ident("false").map(|()| Json::Bool(false)),
            Some(b'"') => self.parse_string().map(Json::String),
            Some(b'[') => self.parse_array(),
            Some(b'{') => self.parse_object(),
            Some(b'-') | Some(b'0'..=b'9') => self.parse_number(),
            Some(b) => Err(self.err(&format!("unexpected byte {b}"))),
            None => Err(self.err("unexpected end of JSON")),
        }
    }

    fn parse_ident(&mut self, expected: &str) -> Result<(), JsonError> {
        for c in expected.bytes() {
            if self.bump() != Some(c) {
                return Err(self.err(&format!("expected {expected:?}")));
            }
        }
        Ok(())
    }

    fn parse_array(&mut self) -> Result<Json, JsonError> {
        if self.bump() != Some(b'[') {
            return Err(self.err("expected '['"));
        }
        let mut items = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(b']') {
                self.i += 1;
                break;
            }
            if !items.is_empty() {
                if self.bump() != Some(b',') {
                    return Err(self.err("expected ',' or ']' in array"));
                }
                self.skip_ws();
                if self.peek() == Some(b']') {
                    return Err(self.err("trailing comma in array"));
                }
            }
            items.push(self.parse_value()?);
        }
        Ok(Json::Array(items))
    }

    fn parse_object(&mut self) -> Result<Json, JsonError> {
        if self.bump() != Some(b'{') {
            return Err(self.err("expected '{'"));
        }
        let mut map = BTreeMap::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(b'}') {
                self.i += 1;
                break;
            }
            if !map.is_empty() {
                if self.bump() != Some(b',') {
                    return Err(self.err("expected ',' or '}' in object"));
                }
                self.skip_ws();
                if self.peek() == Some(b'}') {
                    return Err(self.err("trailing comma in object"));
                }
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.bump() != Some(b':') {
                return Err(self.err("expected ':' after object key"));
            }
            let value = self.parse_value()?;
            map.insert(key, value);
        }
        Ok(Json::Object(map))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        if self.bump() != Some(b'"') {
            return Err(self.err("expected string"));
        }
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err(self.err("unterminated string")),
                Some(b'"') => return Ok(out),
                Some(b'\\') => match self.bump() {
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'/') => out.push('/'),
                    Some(b'n') => out.push('\n'),
                    Some(b'r') => out.push('\r'),
                    Some(b't') => out.push('\t'),
                    Some(b'u') => {
                        let hex = self.read_hex4()?;
                        let cp = u32::from_str_radix(&hex, 16)
                            .map_err(|_| self.err("invalid unicode escape"))?;
                        let ch =
                            char::from_u32(cp).ok_or_else(|| self.err("invalid unicode scalar"))?;
                        out.push(ch);
                    }
                    Some(other) => {
                        return Err(self.err(&format!("unknown string escape {other}")));
                    }
                    None => return Err(self.err("unterminated string escape")),
                },
                Some(b) => {
                    // JSON strings are UTF-8; copy the rest of this scalar.
                    if b < 0x80 {
                        out.push(b as char);
                    } else {
                        self.i -= 1;
                        let rest = std::str::from_utf8(&self.bytes[self.i..])
                            .map_err(|_| self.err("invalid UTF-8 in string"))?;
                        let ch = rest.chars().next().ok_or_else(|| self.err("empty UTF-8"))?;
                        out.push(ch);
                        self.i += ch.len_utf8();
                    }
                }
            }
        }
    }

    fn read_hex4(&mut self) -> Result<String, JsonError> {
        let start = self.i;
        for _ in 0..4 {
            match self.bump() {
                Some(b) if b.is_ascii_hexdigit() => {}
                _ => return Err(self.err("expected 4 hex digits")),
            }
        }
        Ok(std::str::from_utf8(&self.bytes[start..self.i])
            .unwrap()
            .to_string())
    }

    fn parse_number(&mut self) -> Result<Json, JsonError> {
        let start = self.i;
        if self.peek() == Some(b'-') {
            self.i += 1;
        }
        if self.peek() == Some(b'0') {
            self.i += 1;
        } else {
            if !matches!(self.peek(), Some(b'1'..=b'9')) {
                return Err(self.err("expected digit"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        let mut is_float = false;
        if self.peek() == Some(b'.') {
            is_float = true;
            self.i += 1;
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("expected digit after decimal point"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            is_float = true;
            self.i += 1;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if !matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.err("expected exponent digit"));
            }
            while matches!(self.peek(), Some(b'0'..=b'9')) {
                self.i += 1;
            }
        }
        let slice = std::str::from_utf8(&self.bytes[start..self.i])
            .map_err(|_| self.err("invalid UTF-8 in number"))?;
        if is_float {
            let x: f64 = slice.parse().map_err(|_| self.err("float parse failed"))?;
            Ok(Json::Float(x))
        } else {
            let n: i64 = slice
                .parse()
                .map_err(|_| self.err("integer parse failed"))?;
            Ok(Json::Int(n))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_subset() {
        let v = parse_json(r#"{"id":"a","seed":42,"on":true,"n":null,"xs":["b"]}"#).unwrap();
        assert_eq!(v.field("id").and_then(Json::as_str), Some("a"));
        assert_eq!(v.field("seed").and_then(Json::as_i64), Some(42));
        assert_eq!(v.field("on").and_then(Json::as_bool), Some(true));
        assert!(v.field("n").unwrap().is_null());
        assert_eq!(v.field("xs").and_then(Json::as_array).unwrap().len(), 1);
    }

    #[test]
    fn rejects_trailing_comma() {
        assert!(parse_json(r#"{"a":1,}"#).is_err());
    }
}
