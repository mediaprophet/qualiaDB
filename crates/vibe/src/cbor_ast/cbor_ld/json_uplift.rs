//! Provider/TLD-Scoped Namespace Uplift for Incoming JSON into W3C CBOR-LD (`0xD9 0x0501`).

use std::collections::BTreeMap;

use super::terms::TAG_CBOR_LD_COMPRESSED;
use super::value_codec::encode_value_cbor_ld;
use crate::value::Value;

/// Derives `(normalized_host, effective_tld, namespace_iri)` from a provider domain, URL, or TLD.
pub fn derive_provider_namespace(
    provider_host_or_tld: &str,
) -> Result<(String, String, String), String> {
    let raw = provider_host_or_tld.trim().to_ascii_lowercase();
    if raw.is_empty() {
        return Err("provider host or TLD must not be empty".into());
    }
    let is_tld_only = raw.starts_with('.');
    let without_scheme = raw
        .strip_prefix("https://")
        .or_else(|| raw.strip_prefix("http://"))
        .or_else(|| raw.strip_prefix("did:web:"))
        .unwrap_or(&raw);
    let authority = without_scheme
        .split(&['/', '?', '#'][..])
        .next()
        .unwrap_or("");
    let host_part = authority.split(':').next().unwrap_or("").trim_matches('.');
    if host_part.is_empty() {
        return Err(format!("invalid provider host or TLD: '{provider_host_or_tld}'"));
    }
    let labels: Vec<&str> = host_part.split('.').filter(|s| !s.is_empty()).collect();
    if labels.is_empty() {
        return Err(format!("invalid provider labels in '{provider_host_or_tld}'"));
    }
    let effective_tld = labels.last().copied().unwrap().to_string();
    let normalized_host = if is_tld_only || labels.len() == 1 {
        format!("tld.{host_part}")
    } else {
        host_part.to_string()
    };
    let namespace_iri = format!("https://{normalized_host}/ns#");
    Ok((normalized_host, effective_tld, namespace_iri))
}

/// Parses raw JSON, uplifts all bare property keys into the provider's domain/TLD namespace
/// (`https://{host}/ns#{key}`), and encodes the uplifted structure into compressed CBOR-LD (`0x0501`).
pub fn ingest_json_with_provider_ns(
    json_src: &str,
    provider_host_or_tld: &str,
) -> Result<Value, String> {
    let (host, tld, ns_iri) = derive_provider_namespace(provider_host_or_tld)?;
    let mut parser = JsonUpliftParser::new(json_src, &ns_iri);
    let mut root = parser.parse_value()?;
    parser.skip_ws();
    if parser.pos < parser.bytes.len() {
        return Err("trailing characters after JSON value".into());
    }

    match &mut root {
        Value::Record(map) => {
            map.entry("@context".to_string())
                .or_insert_with(|| Value::String(ns_iri.clone()));
            map.insert("@provider_host".to_string(), Value::String(host.clone()));
            map.insert("@provider_tld".to_string(), Value::String(tld.clone()));
        }
        other => {
            let mut wrapper = BTreeMap::new();
            wrapper.insert("@context".to_string(), Value::String(ns_iri.clone()));
            wrapper.insert("@provider_host".to_string(), Value::String(host.clone()));
            wrapper.insert("@provider_tld".to_string(), Value::String(tld.clone()));
            wrapper.insert(format!("{ns_iri}value"), other.clone());
            root = Value::Record(wrapper);
        }
    }

    let cbor_bytes = encode_value_cbor_ld(&root, &ns_iri);
    let mut out = BTreeMap::new();
    out.insert("__cbor_ld".to_string(), Value::Bool(true));
    out.insert("tag".to_string(), Value::I64(TAG_CBOR_LD_COMPRESSED as i64));
    out.insert("context".to_string(), Value::String(ns_iri));
    out.insert("provider_host".to_string(), Value::String(host));
    out.insert("provider_tld".to_string(), Value::String(tld));
    out.insert("byte_len".to_string(), Value::I64(cbor_bytes.len() as i64));
    out.insert("hex".to_string(), Value::String(super::hex_encode(&cbor_bytes)));
    out.insert(
        "bytes".to_string(),
        Value::List(cbor_bytes.iter().map(|&b| Value::I64(b as i64)).collect()),
    );
    out.insert("uplifted".to_string(), root);
    Ok(Value::Record(out))
}

fn uplift_key(raw_key: &str, ns_iri: &str) -> String {
    if raw_key.starts_with('@') || raw_key.contains(':') {
        raw_key.to_string()
    } else {
        format!("{ns_iri}{raw_key}")
    }
}

struct JsonUpliftParser<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
    ns_iri: &'a str,
}

impl<'a> JsonUpliftParser<'a> {
    fn new(src: &'a str, ns_iri: &'a str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            ns_iri,
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.pos).copied()
    }

    fn parse_value(&mut self) -> Result<Value, String> {
        let b = self.peek().ok_or_else(|| "unexpected EOF in JSON".to_string())?;
        match b {
            b'{' => self.parse_object(),
            b'[' => self.parse_array(),
            b'"' => Ok(Value::String(self.parse_string()?)),
            b't' | b'f' => self.parse_bool(),
            b'n' => self.parse_null(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            _ => Err(format!("unexpected JSON byte '{}'", b as char)),
        }
    }

    fn parse_object(&mut self) -> Result<Value, String> {
        self.pos += 1; // '{'
        let mut map = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Record(map));
        }
        loop {
            if self.peek() != Some(b'"') {
                return Err("expected string key in JSON object".into());
            }
            let raw_key = self.parse_string()?;
            let key = uplift_key(&raw_key, self.ns_iri);
            if self.peek() != Some(b':') {
                return Err("expected ':' after JSON object key".into());
            }
            self.pos += 1;
            let val = self.parse_value()?;
            map.insert(key, val);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err("expected ',' or '}' in JSON object".into()),
            }
        }
        Ok(Value::Record(map))
    }

    fn parse_array(&mut self) -> Result<Value, String> {
        self.pos += 1; // '['
        let mut items = Vec::new();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::List(items));
        }
        loop {
            items.push(self.parse_value()?);
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    break;
                }
                _ => return Err("expected ',' or ']' in JSON array".into()),
            }
        }
        Ok(Value::List(items))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_ws();
        if self.bytes.get(self.pos) != Some(&b'"') {
            return Err("expected '\"'".into());
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            self.pos += 1;
            match b {
                b'"' => return Ok(out),
                b'\\' => {
                    let esc = *self.bytes.get(self.pos).ok_or("EOF in string escape")?;
                    self.pos += 1;
                    match esc {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        _ => out.push(esc as char),
                    }
                }
                _ => out.push(b as char),
            }
        }
        Err("unterminated JSON string".into())
    }

    fn parse_bool(&mut self) -> Result<Value, String> {
        if self.src[self.pos..].starts_with("true") {
            self.pos += 4;
            Ok(Value::Bool(true))
        } else if self.src[self.pos..].starts_with("false") {
            self.pos += 5;
            Ok(Value::Bool(false))
        } else {
            Err("invalid boolean literal in JSON".into())
        }
    }

    fn parse_null(&mut self) -> Result<Value, String> {
        if self.src[self.pos..].starts_with("null") {
            self.pos += 4;
            Ok(Value::Null)
        } else {
            Err("invalid null literal in JSON".into())
        }
    }

    fn parse_number(&mut self) -> Result<Value, String> {
        self.skip_ws();
        let start = self.pos;
        let mut is_float = false;
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'0'..=b'9' | b'-' | b'+' => self.pos += 1,
                b'.' | b'e' | b'E' => {
                    is_float = true;
                    self.pos += 1;
                }
                _ => break,
            }
        }
        let token = &self.src[start..self.pos];
        if is_float {
            let f: f64 = token.parse().map_err(|_| format!("invalid float '{token}'"))?;
            Ok(Value::F64(f))
        } else {
            let n: i64 = token.parse().map_err(|_| format!("invalid int '{token}'"))?;
            Ok(Value::I64(n))
        }
    }
}
