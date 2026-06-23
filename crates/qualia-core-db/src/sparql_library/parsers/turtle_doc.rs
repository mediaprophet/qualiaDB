//! Turtle-document parser for QualiaDB ingest.
//!
//! The other parsers in this directory (`n3_star`, `turtle_star`) are line-oriented:
//! they split each physical line on whitespace and keep the first three tokens. That
//! is fine for N-Triples-shaped input but **shreds real Turtle** — multi-line
//! predicate-object lists (`;`), object lists (`,`), and multi-word quoted literals
//! all break, and `@prefix` is never expanded (CURIEs get hashed verbatim, so two
//! documents' `doc:` terms collide).
//!
//! This parser handles the Turtle subset the values-credentials corpus actually uses:
//!
//! * `@prefix pfx: <iri> .` / `@base <iri> .` directives, expanded into full IRIs
//!   before hashing (so a query with `PREFIX`/full `<IRI>` matches the stored hash,
//!   and each instrument's `doc:` namespace is unique).
//! * statements spanning multiple lines, terminated by `.`
//! * `;` (repeat subject) and `,` (repeat subject + predicate) lists
//! * `a` as a synonym for `rdf:type`
//! * quoted literals — including spaces, escapes, `"""…"""`, and trailing
//!   `@lang` / `^^datatype` tags (the tag is dropped; the lexical value is hashed)
//!
//! Terms are hashed with [`generate_60bit_token`] over the **expanded IRI** (or the
//! literal's lexical form), matching the SPARQL query path.

use crate::lexicon::generate_60bit_token;
use crate::sparql_library::quin_sink::QuinSink;
use crate::NQuin;
use std::collections::HashMap;
use std::io::Read;

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

#[derive(Debug, Clone)]
enum Tok {
    Iri(String),       // <…>  (content, brackets stripped)
    Pname(String),     // prefixed name or bare word (may end in ':')
    A,                 // the `a` keyword (rdf:type)
    Lit(String),       // quoted literal lexical value
    Directive(String), // @prefix / @base (keyword without '@')
    Semi,
    Comma,
    Dot,
}

fn tokenize(s: &str) -> Vec<Tok> {
    let b = s.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        // Classify on the raw byte, never `byte as char`: a 0xA0 continuation byte is
        // whitespace under Latin-1 and would wrongly split a multibyte char. UTF-8 is
        // self-synchronising, so ASCII delimiters never occur inside a multibyte sequence.
        if c.is_ascii_whitespace() {
            i += 1;
            continue;
        }
        match c {
            b'#' => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'<' => {
                let st = i + 1;
                let mut j = st;
                while j < b.len() && b[j] != b'>' {
                    j += 1;
                }
                out.push(Tok::Iri(s[st..j].to_string()));
                i = j + 1;
            }
            b'"' => {
                let (lit, next) = read_literal(s, i);
                out.push(Tok::Lit(lit));
                i = next;
                // Drop an optional language tag (@en) or datatype (^^xsd:string / ^^<iri>).
                while i < b.len() {
                    let d = b[i];
                    if d == b'@' || d == b'^' || d == b':' || d.is_ascii_alphanumeric()
                        || d == b'-' || d == b'<' || d == b'>' || d == b'/' || d == b'.'
                        || d == b'#'
                    {
                        i += 1;
                    } else {
                        break;
                    }
                }
            }
            b';' => {
                out.push(Tok::Semi);
                i += 1;
            }
            b',' => {
                out.push(Tok::Comma);
                i += 1;
            }
            b'.' => {
                out.push(Tok::Dot);
                i += 1;
            }
            b'@' => {
                let st = i + 1;
                let mut j = st;
                while j < b.len() && b[j].is_ascii_alphabetic() {
                    j += 1;
                }
                out.push(Tok::Directive(s[st..j].to_string()));
                i = j;
            }
            _ => {
                let st = i;
                while i < b.len() {
                    let d = b[i];
                    if d.is_ascii_whitespace()
                        || d == b';'
                        || d == b','
                        || d == b'<'
                        || d == b'"'
                        || d == b'#'
                    {
                        break;
                    }
                    // A '.' terminates a word only when trailing (followed by ws/EOF/term);
                    // CURIE local names here contain no internal dots.
                    if d == b'.' {
                        let nxt = b.get(i + 1).copied();
                        if nxt.is_none()
                            || nxt.unwrap().is_ascii_whitespace()
                            || matches!(nxt, Some(b';') | Some(b',') | Some(b'#'))
                        {
                            break;
                        }
                    }
                    i += 1;
                }
                let w = &s[st..i];
                if w == "a" {
                    out.push(Tok::A);
                } else if !w.is_empty() {
                    out.push(Tok::Pname(w.to_string()));
                }
            }
        }
    }
    out
}

/// Read a `"…"` or `"""…"""` literal starting at `start` (the opening quote). Returns the
/// unescaped lexical value and the byte index just past the closing quote(s).
///
/// The closing delimiter (`"`, escaped by ASCII `\`) is found by a byte scan — safe under
/// UTF-8 self-synchronisation — and the inner text is then taken as a `&str` SLICE, never
/// reconstructed byte-by-byte, so non-ASCII content (Arabic, CJK, Ge'ez, em-dashes, …) is
/// preserved exactly. Slice bounds land on ASCII quote positions, i.e. valid char boundaries.
fn read_literal(s: &str, start: usize) -> (String, usize) {
    let b = s.as_bytes();
    // Triple-quoted: """ … """
    if b[start..].starts_with(b"\"\"\"") {
        let inner = start + 3;
        let mut j = inner;
        while j + 3 <= b.len() && !(b[j] == b'"' && b[j + 1] == b'"' && b[j + 2] == b'"') {
            j += 1;
        }
        let end = if j + 3 <= b.len() { j } else { b.len() };
        return (unescape(&s[inner..end]), (end + 3).min(b.len()));
    }
    // Single-quoted: " … " — scan to the first unescaped closing quote.
    let inner = start + 1;
    let mut j = inner;
    while j < b.len() {
        match b[j] {
            b'\\' if j + 1 < b.len() => j += 2, // skip the (ASCII) escape pair
            b'"' => break,
            _ => j += 1,
        }
    }
    (unescape(&s[inner..j]), (j + 1).min(b.len()))
}

/// Unicode-safe unescape over a `&str` — iterates chars, never bytes.
fn unescape(s: &str) -> String {
    if !s.contains('\\') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

fn expand(pname: &str, prefixes: &HashMap<String, String>) -> String {
    if let Some((pfx, local)) = pname.split_once(':') {
        if let Some(base) = prefixes.get(pfx) {
            return format!("{base}{local}");
        }
    }
    pname.to_string()
}

/// Resolve a non-directive token to `(hash, canonical lexical string)`. `None` for
/// punctuation. The string is what gets recorded in the lexicon for recovery: the
/// expanded full IRI, the rdf:type IRI for `a`, or the literal's lexical value.
fn resolve(tok: &Tok, prefixes: &HashMap<String, String>, base: &str) -> Option<(u64, String)> {
    let s = match tok {
        Tok::A => RDF_TYPE.to_string(),
        Tok::Iri(iri) if !base.is_empty() && !iri.contains("://") && !iri.is_empty() => {
            format!("{base}{iri}")
        }
        Tok::Iri(iri) => iri.clone(),
        Tok::Pname(p) => expand(p, prefixes),
        Tok::Lit(l) => l.clone(),
        _ => return None,
    };
    Some((generate_60bit_token(s.as_bytes()), s))
}

/// Parse a Turtle document into `sink`, returning the number of triples emitted.
pub fn parse_turtle_doc_into<R: Read, S: QuinSink>(
    mut reader: R,
    context_hash: u64,
    sink: &mut S,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut text = String::new();
    reader.read_to_string(&mut text)?;
    let toks = tokenize(&text);

    let mut prefixes: HashMap<String, String> = HashMap::new();
    let mut base = String::new();
    let mut subject: Option<u64> = None;
    let mut predicate: Option<u64> = None;
    let mut count = 0u64;

    let mut i = 0;
    while i < toks.len() {
        match &toks[i] {
            Tok::Directive(kw) => {
                if kw.eq_ignore_ascii_case("prefix") {
                    if let (Some(Tok::Pname(lbl)), Some(Tok::Iri(iri))) =
                        (toks.get(i + 1), toks.get(i + 2))
                    {
                        prefixes.insert(lbl.trim_end_matches(':').to_string(), iri.clone());
                    }
                } else if kw.eq_ignore_ascii_case("base") {
                    if let Some(Tok::Iri(iri)) = toks.get(i + 1) {
                        base = iri.clone();
                    }
                }
                // Skip to the directive-terminating Dot.
                while i < toks.len() && !matches!(toks[i], Tok::Dot) {
                    i += 1;
                }
                i += 1;
                subject = None;
                predicate = None;
            }
            Tok::Dot => {
                subject = None;
                predicate = None;
                i += 1;
            }
            Tok::Semi => {
                predicate = None;
                i += 1;
            }
            Tok::Comma => {
                i += 1;
            }
            tok => {
                let (h, lex) = match resolve(tok, &prefixes, &base) {
                    Some(pair) => pair,
                    None => {
                        i += 1;
                        continue;
                    }
                };
                sink.push_lex(h, &lex);
                if subject.is_none() {
                    subject = Some(h);
                } else if predicate.is_none() {
                    predicate = Some(h);
                } else {
                    let (s, p) = (subject.unwrap(), predicate.unwrap());
                    sink.push(NQuin {
                        subject: s,
                        predicate: p,
                        object: h,
                        context: context_hash,
                        metadata: 0,
                        parity: s ^ p ^ h ^ context_hash,
                    })?;
                    count += 1;
                }
                i += 1;
            }
        }
    }
    Ok(count)
}

/// Streaming entry point used by the CLI ingest pipeline (writes via `ExternalSorter`).
pub fn parse_turtle_doc_stream<R: Read>(
    reader: R,
    context_hash: u64,
    sorter: &mut crate::external_sort::ExternalSorter,
) -> Result<u64, Box<dyn std::error::Error>> {
    parse_turtle_doc_into(reader, context_hash, sorter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexicon::generate_60bit_token as h;

    /// Collect every emitted triple into a Vec (test sink).
    #[derive(Default)]
    struct VecSink(Vec<NQuin>);
    impl QuinSink for VecSink {
        fn push(&mut self, q: NQuin) -> std::io::Result<()> {
            self.0.push(q);
            Ok(())
        }
    }

    fn parse(doc: &str) -> Vec<NQuin> {
        let mut sink = VecSink::default();
        parse_turtle_doc_into(doc.as_bytes(), 0, &mut sink).unwrap();
        sink.0
    }

    #[test]
    fn multiline_predicate_object_list_with_prefix_and_literals() {
        let doc = r#"
@prefix dc:     <http://purl.org/dc/terms/> .
@prefix values: <https://ns.webcivics.net/values/> .
@prefix doc:    <https://ns.webcivics.net/values/inst#> .

doc:article-1 a values:Undertaking ;
    dc:title "Article 1" ;
    values:partOf doc:Instrument ;
    values:originalText "Each Member undertakes to suppress forced labour." .
"#;
        let quins = parse(doc);
        assert_eq!(quins.len(), 4, "subject reused across `;` → four triples");

        let art1 = h(b"https://ns.webcivics.net/values/inst#article-1");
        let rdf_type = h(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let undertaking = h(b"https://ns.webcivics.net/values/Undertaking");
        let dc_title = h(b"http://purl.org/dc/terms/title");
        let part_of = h(b"https://ns.webcivics.net/values/partOf");

        // `a` expands to rdf:type; CURIEs expand via @prefix.
        assert!(quins.iter().any(|q| q.subject == art1 && q.predicate == rdf_type && q.object == undertaking));
        // dc:title is a PREDICATE on a continuation line, subject carried over — the bug we fixed.
        assert!(quins.iter().any(|q| q.subject == art1 && q.predicate == dc_title));
        // Multi-word literal hashes as ONE object (not shredded into words).
        let title = h(b"Article 1");
        assert!(quins.iter().any(|q| q.predicate == dc_title && q.object == title));
        // partOf links to the doc:-expanded Instrument (now namespace-unique).
        let instrument = h(b"https://ns.webcivics.net/values/inst#Instrument");
        assert!(quins.iter().any(|q| q.subject == art1 && q.predicate == part_of && q.object == instrument));
    }

    #[test]
    fn object_list_comma_repeats_subject_and_predicate() {
        let doc = r#"
@prefix values: <https://ns.webcivics.net/values/> .
values:State values:bears values:DutyA , values:DutyB , values:DutyC .
"#;
        let quins = parse(doc);
        assert_eq!(quins.len(), 3, "`,` repeats subject+predicate → three triples");
        let state = h(b"https://ns.webcivics.net/values/State");
        let bears = h(b"https://ns.webcivics.net/values/bears");
        assert!(quins.iter().all(|q| q.subject == state && q.predicate == bears));
    }

    /// Regression: non-ASCII literals (Arabic, CJK, em-dash, curly quotes) must survive intact.
    /// A correct object hash proves the lexical string was preserved byte-exact — a byte-by-byte
    /// `byte as char` reconstruction would hash differently (and could panic mid-codepoint).
    #[test]
    fn non_ascii_literals_roundtrip_intact() {
        let doc = "@prefix v: <https://ns.webcivics.net/values/> .\n\
                   v:x v:label \"صحة\" ; v:note \"健康 — wellbeing's “root”\" .";
        let quins = parse(doc);
        let label = h(b"https://ns.webcivics.net/values/label");
        let note = h(b"https://ns.webcivics.net/values/note");
        assert!(
            quins.iter().any(|q| q.predicate == label && q.object == h("صحة".as_bytes())),
            "Arabic literal must hash to its exact UTF-8 bytes"
        );
        assert!(
            quins
                .iter()
                .any(|q| q.predicate == note && q.object == h("健康 — wellbeing's “root”".as_bytes())),
            "CJK + em-dash + curly-quote literal must round-trip intact"
        );
    }
}
