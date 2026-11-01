//! Standard RDF typed-double lexical terms for bounded cold Q42 construction.
//! No new binary encoding: the complete UTF-8 term is retained in Q42LEX under
//! the same masked object hash used by Complete RDF ingestion. Numeric decoding
//! is explicit; a token is never interpreted as a numeric magnitude.

pub const DATATYPE: &str = "http://www.w3.org/2001/XMLSchema#double";
const SUFFIX: &str = "\"^^<http://www.w3.org/2001/XMLSchema#double>";
/// Finite authored binary64 profile; ample space for a non-exponent subnormal.
pub const MAX_TERM_BYTES: usize = 512;

/// Preserve the lexical spelling of a finite xsd:double. The admitted numeric
/// alphabet needs no RDF escaping. Other literal forms remain with the general
/// RDF parser, not this deliberately narrow numeric adapter.
pub fn from_lexical(lexical: &str) -> Option<String> {
    numeric(lexical)?;
    let length = 1usize.checked_add(lexical.len())?.checked_add(SUFFIX.len())?;
    if length > MAX_TERM_BYTES { return None; }
    Some(format!("\"{lexical}{SUFFIX}"))
}

/// Rust's shortest round-tripping decimal preserves every finite binary64,
/// including signed zero, without conversion through binary32.
pub fn from_f64(value: f64) -> Option<String> {
    if !value.is_finite() { return None; }
    from_lexical(&value.to_string())
}

/// Borrow the numeric lexical text and decode its actual finite value. This
/// does not allocate, confer source authority or qualify generic SPARQL maths.
pub fn decode(term: &str) -> Option<(&str, f64)> {
    if term.len() > MAX_TERM_BYTES { return None; }
    let lexical = term.strip_prefix('"')?.strip_suffix(SUFFIX)?;
    Some((lexical, numeric(lexical)?))
}

/// Recognize the declared RDF datatype independently of numeric validity.
/// Uses the existing Rio parser (cold metadata lookup), so INF/NaN and ill-typed
/// lexical values keep their declared datatype without becoming finite recipes.
pub fn has_declared_datatype(term: &str) -> bool {
    use rio_api::{model::{Literal,Term},parser::TriplesParser};
    if term.len() > MAX_TERM_BYTES || !term.starts_with('"') || !term.ends_with(SUFFIX) {
        return false;
    }
    let input=format!("<urn:q42:literal> <urn:q42:value> {term} .");
    let mut parser=rio_turtle::NTriplesParser::new(std::io::Cursor::new(input.as_bytes()));
    let mut count=0usize;let mut valid=false;
    let result=parser.parse_all(&mut |triple| -> Result<(),rio_turtle::TurtleError> {
        count+=1;
        valid=matches!(triple.object,Term::Literal(Literal::Typed {datatype,..}) if datatype.iri==DATATYPE);
        Ok(())
    });
    result.is_ok() && count==1 && valid
}

fn numeric(lexical: &str) -> Option<f64> {
    if lexical.is_empty() || lexical.len() > MAX_TERM_BYTES
        || !lexical.bytes().all(|b| b.is_ascii_digit() || b"+-.eE".contains(&b)) {
        return None;
    }
    let value = lexical.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

/// Canonical Complete-ingest object hash; the retained lexical term must also
/// be checked for collisions by the owning builder's bounded dictionary.
pub fn token(term: &str) -> u64 {
    crate::lexicon::generate_60bit_token(term.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn binary64_adjacent_extremes_and_signed_zero_roundtrip() {
        for value in [1.0, f64::from_bits(1.0f64.to_bits()+1), -0.0,
            f64::from_bits(1), f64::MIN_POSITIVE, f64::MAX, -f64::MAX] {
            let term=from_f64(value).unwrap();
            assert_eq!(decode(&term).unwrap().1.to_bits(),value.to_bits());
            assert_eq!(token(&term),crate::q_hash(&term)&0x0FFF_FFFF_FFFF_FFFF);
            assert_eq!(token(&term),crate::lexicon::generate_60bit_token(term.as_bytes()));
        }
        assert_ne!(token(&from_f64(1.0).unwrap()),token(&from_f64(f64::from_bits(1.0f64.to_bits()+1)).unwrap()));
    }
    #[test]
    fn unsupported_or_nonfinite_values_never_become_rounded_inline_values() {
        for value in [f64::NAN,f64::INFINITY,f64::NEG_INFINITY] {assert!(from_f64(value).is_none());}
        for lexical in ["NaN","INF","1e9999","1 2","1\"","","0x12"," 1"] {assert!(from_lexical(lexical).is_none());}
        assert!(decode("\"1\"^^<http://www.w3.org/2001/XMLSchema#float>").is_none());
        assert!(super::super::pack_typed_literal("1.0000000000000002",DATATYPE).is_none());
        assert!(super::super::pack_typed_literal("1.25","xsd:float").is_some());
    }
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn complete_ingest_lexicon_constant_and_datatype_share_actual_term() {
        use rio_api::model::{Literal,NamedNode,Subject,Term,Triple};
        use crate::sparql_ast::{literal_term_hash,LiteralTable,TextResolver};
        let lexical="1.0000000000000002";
        let raw=crate::query::ingest_formats::pack_rio_triple(Triple {
            subject:Subject::NamedNode(NamedNode {iri:"urn:s"}),
            predicate:NamedNode {iri:"urn:p"},
            object:Term::Literal(Literal::Typed {value:lexical,datatype:NamedNode {iri:DATATYPE}}),
        },0);
        assert!(raw.packed_object.is_none());
        assert_eq!(raw.object,from_lexical(lexical).unwrap());
        let hash=token(&raw.object);
        assert_eq!(hash,literal_term_hash(lexical,None,Some(DATATYPE)));
        let mut entries=std::collections::HashMap::new();entries.insert(hash,raw.object);
        let encoded=crate::q42_lex::serialize_string_lexicon(&entries).unwrap();
        let lex=crate::q42_lex::Q42LexMmap::from_bytes(&encoded).unwrap();
        let lookup=|key|lex.lookup_hash(key).map(str::to_owned);
        let literals=LiteralTable::new();
        let resolver=TextResolver::with_lexicon(&literals,&lookup);
        assert_eq!(resolver.datatype_of(hash).as_deref(),Some(DATATYPE));
        let recovered=resolver.resolve_text(hash).unwrap().parse::<f64>().unwrap();
        assert_eq!(recovered.to_bits(),lexical.parse::<f64>().unwrap().to_bits());
    }
    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn special_and_illtyped_rdf_values_retain_datatype_without_numeric_admission() {
        use rio_api::model::{Literal,NamedNode,Subject,Term,Triple};
        use crate::sparql_ast::{LiteralTable,TextResolver};
        for value in ["INF","-INF","NaN","not-a-number","a\"quoted"] {
            let raw=crate::query::ingest_formats::pack_rio_triple(Triple {
                subject:Subject::NamedNode(NamedNode {iri:"urn:s"}),predicate:NamedNode {iri:"urn:p"},
                object:Term::Literal(Literal::Typed {value,datatype:NamedNode {iri:DATATYPE}}),
            },0);
            assert!(raw.packed_object.is_none());assert!(has_declared_datatype(&raw.object));
            assert!(decode(&raw.object).is_none());
            let hash=token(&raw.object);let lookup=|key|(key==hash).then(||raw.object.clone());
            let literals=LiteralTable::new();let resolver=TextResolver::with_lexicon(&literals,&lookup);
            assert_eq!(resolver.datatype_of(hash).as_deref(),Some(DATATYPE));
        }
        assert!(!has_declared_datatype("\"1\"^^<urn:other>"));
        assert!(!has_declared_datatype("\"1\" . <urn:s> <urn:p> \"2\"^^<http://www.w3.org/2001/XMLSchema#double>"));
    }
}
