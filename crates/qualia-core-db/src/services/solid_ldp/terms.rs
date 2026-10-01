//! Lexicon-aware term writers for Solid RDF export (cold path).

use std::collections::HashMap;
use std::io::{self, Write};

use crate::query::resolver::{classify_inline_literal, resolve_hash, InlineLiteral, MSB_FLAG};

pub fn write_iri<W: Write>(
    val: u64,
    lex: Option<&HashMap<u64, String>>,
    out: &mut W,
) -> io::Result<()> {
    if let Some(map) = lex {
        if let Some(s) = map.get(&val) {
            write!(out, "<{s}>")?;
            return Ok(());
        }
    }
    if let Some(bytes) = resolve_hash(val) {
        out.write_all(b"<")?;
        out.write_all(bytes)?;
        return out.write_all(b">");
    }
    if (val & MSB_FLAG) != 0 {
        let ptr = val & !MSB_FLAG;
        return write!(out, "<did:q42:ptr/{ptr:016x}>");
    }
    write!(out, "<quin:hash/{val:016x}>")
}

pub fn write_object<W: Write>(
    val: u64,
    lex: Option<&HashMap<u64, String>>,
    out: &mut W,
) -> io::Result<()> {
    if let Some(lit) = classify_inline_literal(val) {
        return write_inline_literal(lit, out);
    }
    write_iri(val, lex, out)
}

fn write_inline_literal<W: Write>(lit: InlineLiteral, out: &mut W) -> io::Result<()> {
    write!(out, "\"{}\"^^<{}>", lit, lit.datatype_iri())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    #[test]
    fn caller_lex_beats_fallback() {
        let mut map = HashMap::new();
        map.insert(q_hash("Alice"), "https://example.org/Alice".into());
        let mut buf = Vec::new();
        write_iri(q_hash("Alice"), Some(&map), &mut buf).unwrap();
        assert_eq!(
            std::str::from_utf8(&buf).unwrap(),
            "<https://example.org/Alice>"
        );
    }

    #[test]
    fn demo_lex_resolves_knows() {
        let mut buf = Vec::new();
        write_iri(q_hash("knows"), None, &mut buf).unwrap();
        assert!(std::str::from_utf8(&buf).unwrap().contains("schema.org/knows"));
    }
}
