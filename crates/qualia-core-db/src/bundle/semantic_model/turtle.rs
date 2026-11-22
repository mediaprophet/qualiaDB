//! Strict Turtle import for offline Solid LDP sources.

use super::{invalid, valid_iri, SemanticModelBuilder};
use rio_api::{model::{Subject, Term}, parser::TriplesParser};
use rio_turtle::TurtleParser;
use std::{io::Cursor, sync::atomic::{AtomicBool, Ordering}};

impl SemanticModelBuilder {
    /// Parse an RDF Source with its final LDP resource IRI as the base and graph.
    /// Preserve literals with datatype/language information, and scope blank node
    /// labels to the source document before hashing them.
    pub fn seal_solid_turtle(
        mut self,
        source_iri: &str,
        payload: &str,
        cancel: &AtomicBool,
    ) -> std::io::Result<(Vec<u8>, usize)> {
        if !valid_iri(source_iri) || payload.len() > self.limit {
            return Err(invalid("invalid Solid Turtle base IRI or source exceeds byte budget"));
        }
        let base = oxiri::Iri::parse(source_iri.to_owned())
            .map_err(|error| invalid(error.to_string()))?;
        let scope = format!("q42{:015x}_", crate::q_hash(source_iri));
        let mut parser = TurtleParser::new(Cursor::new(payload.as_bytes()), Some(base));
        parser.parse_all(&mut |triple| -> Result<(), Box<dyn std::error::Error>> {
            if cancel.load(Ordering::Relaxed) {
                return Err(std::io::Error::new(std::io::ErrorKind::Interrupted, "Turtle import cancelled").into());
            }
            let subject = match triple.subject {
                Subject::NamedNode(value) => value.iri.to_owned(),
                Subject::BlankNode(value) => format!("_:{scope}{}", value.id),
                Subject::Triple(_) => return Err(invalid("Solid LDP Turtle source uses an RDF-star subject").into()),
            };
            let object = match triple.object {
                Term::NamedNode(value) => value.iri.to_owned(),
                Term::BlankNode(value) => format!("_:{scope}{}", value.id),
                Term::Literal(value) => value.to_string(),
                Term::Triple(_) => return Err(invalid("Solid LDP Turtle source uses an RDF-star object").into()),
            };
            self.fact(&subject, triple.predicate.iri, &object, source_iri)?;
            Ok(())
        }).map_err(|error| invalid(format!("Solid Turtle parse: {error}")))?;
        let count = self.quins.len();
        self.seal_volume().map(|bytes| (bytes, count))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{q_hash, q42_volume::{Q42RangeQueryCursor, Q42RangeQueryPattern, Q42RangeQueryPlan,
        WasmSliceQ42Reader, MAX_COMPRESSED_SUPERBLOCK_SIZE, SUPERBLOCK_SIZE}, NQuin};

    #[test]
    fn imports_base_relative_iris_and_language_tagged_objects() {
        let cancel = AtomicBool::new(false);
        let base = "https://example.test/pod/graph.ttl";
        let turtle = r#"@prefix ex: <https://example.test/vocab#> .
<part> ex:name "軸"@ja ; ex:parent <assembly> ."#;
        let (bytes, count) = SemanticModelBuilder::new(1024 * 1024)
            .seal_solid_turtle(base, turtle, &cancel).unwrap();
        assert_eq!(count, 2);
        let model = WasmSliceQ42Reader::open(&bytes).unwrap();
        let mut compressed = vec![0; MAX_COMPRESSED_SUPERBLOCK_SIZE];
        let mut decoded = vec![0; SUPERBLOCK_SIZE];
        let mut rows = [NQuin::default(); 4];
        let plan = Q42RangeQueryPlan::for_pattern(Q42RangeQueryPattern {
            subject: Some(q_hash("https://example.test/pod/part")),
            context: Some(q_hash(base)),
            ..Q42RangeQueryPattern::default()
        });
        let page = model.execute_query_page_into(plan, Q42RangeQueryCursor::default(),
            &mut compressed, &mut decoded, &mut rows).unwrap();
        assert_eq!(page.returned, 2);
        assert!(rows[..page.returned].iter().any(|row| row.object == q_hash("\"軸\"@ja")));
        assert!(SemanticModelBuilder::new(1024 * 1024)
            .seal_solid_turtle(base, "<x> <y> .", &cancel).is_err());
    }
}
