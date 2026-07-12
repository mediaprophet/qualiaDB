//! Turtle-Star Parser for QualiaDB
//!
//! Implements RDF-Star (SPARQL 1.2) parsing for Turtle syntax with embedded triples.
//! Stack-based state machine for zero-allocation handling of deeply nested structures.
//!
//! Architecture:
//! - Fixed-size stack array [StackFrame; 16] for nested embedded triples
//! - No heap allocations in hot path
//! - Virtual IDs minted via generate_embedded_triple_id()
//! - Context stored separately in NQuin field (not in Virtual ID hash)

use crate::lexicon::{generate_60bit_token, generate_embedded_triple_id};
use crate::rdf_star::{RdfStarParseError, RdfStarParser};
use crate::NQuin;
use std::io::{BufRead, BufReader, Read};

/// Maximum nesting depth for embedded triples
const MAX_NESTING_DEPTH: usize = 16;

/// Parsing state for a single frame
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParsingState {
    ExpectSubject,
    ExpectPredicate,
    ExpectObject,
    ExpectEmbeddedEnd,
}

/// Stack frame for tracking nested embedded triple parsing
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct StackFrame {
    subject: Option<u64>,
    predicate: Option<u64>,
    object: Option<u64>,
    parsing_state: ParsingState,
}

impl StackFrame {
    fn new() -> Self {
        Self {
            subject: None,
            predicate: None,
            object: None,
            parsing_state: ParsingState::ExpectSubject,
        }
    }
}

/// Zero-allocation parser stack
#[repr(C)]
struct ParserStack {
    frames: [StackFrame; MAX_NESTING_DEPTH],
    depth: usize,
}

impl ParserStack {
    fn new() -> Self {
        Self {
            frames: [StackFrame::new(); MAX_NESTING_DEPTH],
            depth: 0,
        }
    }

    fn push(&mut self, frame: StackFrame) -> Result<(), RdfStarParseError> {
        if self.depth >= MAX_NESTING_DEPTH {
            return Err(RdfStarParseError::BufferOverflow);
        }
        self.frames[self.depth] = frame;
        self.depth += 1;
        Ok(())
    }

    fn pop(&mut self) -> Option<StackFrame> {
        if self.depth == 0 {
            return None;
        }
        self.depth -= 1;
        Some(self.frames[self.depth])
    }

    fn current(&mut self) -> &mut StackFrame {
        &mut self.frames[self.depth - 1]
    }

    fn is_empty(&self) -> bool {
        self.depth == 0
    }

    fn depth(&self) -> usize {
        self.depth
    }
}

/// Turtle-Star parser implementation
pub struct TurtleStarParser {
    /// Context hash for the current parsing session
    context_hash: u64,
    /// Stack for nested embedded triple parsing
    stack: ParserStack,
}

impl TurtleStarParser {
    /// Create a new Turtle-Star parser
    pub fn new(context_hash: u64) -> Self {
        Self {
            context_hash,
            stack: ParserStack::new(),
        }
    }

    /// Session context hash stamped onto emitted quads.
    pub fn session_context(&self) -> u64 {
        self.context_hash
    }

    /// Whether the embedded-triple stack is empty (no in-flight `<<` frames).
    pub fn stack_is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    fn expect_state(
        frame: &mut StackFrame,
        expected: ParsingState,
        next: ParsingState,
    ) -> Result<(), RdfStarParseError> {
        if frame.parsing_state != expected {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }
        frame.parsing_state = next;
        Ok(())
    }

    /// Parse a Turtle-Star token (IRI, literal, or delimiter)
    ///
    /// Returns Ok(Some(hash)) for valid IRIs/literals, Ok(None) for delimiters
    fn parse_token(&self, input: &[u8], pos: &mut usize) -> Result<Option<u64>, RdfStarParseError> {
        let bytes = &input[*pos..];

        // Skip whitespace
        let mut start = 0;
        while start < bytes.len() && bytes[start].is_ascii_whitespace() {
            start += 1;
        }

        if start >= bytes.len() {
            return Ok(None);
        }

        let ch = bytes[start];

        // Check for embedded triple start
        if ch == b'<' && start + 1 < bytes.len() && bytes[start + 1] == b'<' {
            *pos += start + 2;
            return Ok(None); // Signal embedded triple start
        }

        // Check for embedded triple end
        if ch == b'>' && start + 1 < bytes.len() && bytes[start + 1] == b'>' {
            *pos += start + 2;
            return Ok(None); // Signal embedded triple end
        }

        // Check for statement terminator
        if ch == b'.' {
            *pos += start + 1;
            return Ok(None);
        }

        // Check for semicolon (predicate separator in Turtle)
        if ch == b';' {
            *pos += start + 1;
            return Ok(None);
        }

        // Parse IRI or literal (simplified - proper Turtle would have <> delimiters)
        let mut end = start;
        while end < bytes.len()
            && !bytes[end].is_ascii_whitespace()
            && bytes[end] != b'.'
            && bytes[end] != b';'
        {
            end += 1;
        }

        if start == end {
            return Ok(None);
        }

        *pos += end;

        // Hash the token
        let token =
            std::str::from_utf8(&bytes[start..end]).map_err(|_| RdfStarParseError::InvalidUtf8)?;
        let hash = generate_60bit_token(token.as_bytes());

        Ok(Some(hash))
    }

    /// Parse an embedded triple using stack-based state machine
    fn parse_embedded_triple_internal(
        &mut self,
        input: &[u8],
        pos: &mut usize,
    ) -> Result<(u64, [u64; 3]), RdfStarParseError> {
        // Push new frame for embedded triple
        let frame = StackFrame::new();
        self.stack.push(frame)?;

        let _start_depth = self.stack.depth();

        // Parse subject
        {
            let frame = self.stack.current();
            Self::expect_state(
                frame,
                ParsingState::ExpectSubject,
                ParsingState::ExpectPredicate,
            )?;
        }
        while *pos < input.len() && input[*pos].is_ascii_whitespace() {
            *pos += 1;
        }
        if *pos + 1 < input.len() && input[*pos] == b'<' && input[*pos + 1] == b'<' {
            *pos += 2; // skip <<
            let (hash, _) = self.parse_embedded_triple_internal(input, pos)?;
            self.stack.current().subject = Some(hash);
        } else {
            match self.parse_token(input, pos)? {
                Some(hash) => self.stack.current().subject = Some(hash),
                None => return Err(RdfStarParseError::MalformedEmbeddedTriple),
            }
        }

        // Parse predicate
        {
            let frame = self.stack.current();
            Self::expect_state(
                frame,
                ParsingState::ExpectPredicate,
                ParsingState::ExpectObject,
            )?;
        }
        match self.parse_token(input, pos)? {
            Some(hash) => self.stack.current().predicate = Some(hash),
            None => return Err(RdfStarParseError::MalformedEmbeddedTriple),
        }

        // Parse object (could be another embedded triple)
        {
            let frame = self.stack.current();
            Self::expect_state(
                frame,
                ParsingState::ExpectObject,
                ParsingState::ExpectEmbeddedEnd,
            )?;
        }
        while *pos < input.len() && input[*pos].is_ascii_whitespace() {
            *pos += 1;
        }
        if *pos + 1 < input.len() && input[*pos] == b'<' && input[*pos + 1] == b'<' {
            *pos += 2; // skip <<
            let (hash, _) = self.parse_embedded_triple_internal(input, pos)?;
            self.stack.current().object = Some(hash);
        } else {
            match self.parse_token(input, pos)? {
                Some(hash) => self.stack.current().object = Some(hash),
                None => {
                    return Err(RdfStarParseError::MalformedEmbeddedTriple);
                }
            }
        }

        // Expect >> terminator
        {
            let frame = self.stack.current();
            Self::expect_state(
                frame,
                ParsingState::ExpectEmbeddedEnd,
                ParsingState::ExpectSubject,
            )?;
        }
        while *pos + 1 < input.len() && input[*pos].is_ascii_whitespace() {
            *pos += 1;
        }
        if *pos + 1 >= input.len() || input[*pos] != b'>' || input[*pos + 1] != b'>' {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }
        *pos += 2;

        // Pop frame and get components
        if !self.stack.is_empty() && self.stack.depth() > 1 {
            // Nested frames remain for future recursive `<<` support.
        }
        let frame = self
            .stack
            .pop()
            .ok_or(RdfStarParseError::MalformedEmbeddedTriple)?;

        let subject = frame
            .subject
            .ok_or(RdfStarParseError::MalformedEmbeddedTriple)?;
        let predicate = frame
            .predicate
            .ok_or(RdfStarParseError::MalformedEmbeddedTriple)?;
        let object = frame
            .object
            .ok_or(RdfStarParseError::MalformedEmbeddedTriple)?;

        // Generate Virtual ID (context-independent per architectural decision)
        let virtual_id = generate_embedded_triple_id(subject, predicate, object);

        Ok((virtual_id, [subject, predicate, object]))
    }

    /// Parse a full Turtle-Star annotation statement: `<< s p o >> annP annO [.]`.
    ///
    /// Returns `(virtual_id, [s,p,o], Option<(ann_predicate, ann_object)>)`. Unlike
    /// `parse_embedded_triple` (which stops at `>>` and drops everything after),
    /// this captures the outer predicate/object when present, so the caller can
    /// emit the linking statement `virtual_id annP annO` — reaching parity with
    /// the N-Quads-Star parser. A single parse pass handles both the annotated
    /// and the bare-embedded-triple cases (the latter yields `None` for the
    /// annotation), so there is no stateful re-parse of the same line.
    fn parse_star_annotation(
        &mut self,
        input: &[u8],
    ) -> Result<(u64, [u64; 3], Option<(u64, u64)>), RdfStarParseError> {
        let mut pos = 0;
        while pos < input.len() && input[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos + 1 < input.len() && input[pos] == b'<' && input[pos + 1] == b'<' {
            pos += 2;
        } else {
            return Err(RdfStarParseError::MalformedEmbeddedTriple);
        }
        let (virtual_id, components) = self.parse_embedded_triple_internal(input, &mut pos)?;
        // Outer predicate + object are optional: a line may be a bare `<< s p o >>`.
        let annotation = match self.parse_token(input, &mut pos)? {
            Some(ann_predicate) => self
                .parse_token(input, &mut pos)?
                .map(|ann_object| (ann_predicate, ann_object)),
            None => None,
        };
        Ok((virtual_id, components, annotation))
    }
}

impl RdfStarParser for TurtleStarParser {
    fn parse_embedded_triple(
        &mut self,
        input: &[u8],
    ) -> Result<(u64, [u64; 3]), RdfStarParseError> {
        let mut pos = 0;
        while pos < input.len() && input[pos].is_ascii_whitespace() {
            pos += 1;
        }
        if pos + 1 < input.len() && input[pos] == b'<' && input[pos + 1] == b'<' {
            pos += 2;
        }
        self.parse_embedded_triple_internal(input, &mut pos)
    }

    fn parse_triple(&mut self, input: &[u8]) -> Result<(u64, u64, u64), RdfStarParseError> {
        let mut pos = 0;

        let skip_ws = |p: &mut usize| {
            while *p < input.len() && input[*p].is_ascii_whitespace() {
                *p += 1;
            }
        };

        skip_ws(&mut pos);
        let subject = if pos + 1 < input.len() && input[pos] == b'<' && input[pos + 1] == b'<' {
            pos += 2; // skip <<
            let (hash, _) = self.parse_embedded_triple_internal(input, &mut pos)?;
            // parse_token might have stopped at '>>'
            // skip '>>'
            skip_ws(&mut pos);
            if pos + 1 < input.len() && input[pos] == b'>' && input[pos + 1] == b'>' {
                pos += 2;
            }
            hash
        } else {
            match self.parse_token(input, &mut pos)? {
                Some(h) => h,
                None => return Err(RdfStarParseError::InvalidSyntax),
            }
        };

        skip_ws(&mut pos);
        let predicate = match self.parse_token(input, &mut pos)? {
            Some(h) => h,
            None => return Err(RdfStarParseError::InvalidSyntax),
        };

        skip_ws(&mut pos);
        let object = if pos + 1 < input.len() && input[pos] == b'<' && input[pos + 1] == b'<' {
            pos += 2; // skip <<
            let (hash, _) = self.parse_embedded_triple_internal(input, &mut pos)?;
            skip_ws(&mut pos);
            if pos + 1 < input.len() && input[pos] == b'>' && input[pos + 1] == b'>' {
                pos += 2;
            }
            hash
        } else {
            match self.parse_token(input, &mut pos)? {
                Some(h) => h,
                None => return Err(RdfStarParseError::InvalidSyntax),
            }
        };

        // The object is the final token; this parser returns the triple, not the cursor,
        // so `pos`'s final position is intentionally not propagated past here.
        let _ = pos;
        Ok((subject, predicate, object))
    }

    fn parse_quad(&mut self, _input: &[u8]) -> Result<(u64, u64, u64, u64), RdfStarParseError> {
        // Turtle-Star doesn't support quads natively (use Trig-Star for that)
        Err(RdfStarParseError::UnsupportedFeature)
    }

    fn supports_quads(&self) -> bool {
        false
    }

    fn supports_named_graphs(&self) -> bool {
        false
    }

    fn format_name(&self) -> &'static str {
        "Turtle-Star"
    }
}

/// Legacy function for backward compatibility with existing ingest pipeline
///
/// TODO: This should be refactored to use the RdfStarParser trait properly
/// and integrate with the lexicon writing layer for 24-byte embedded triple storage.
pub fn parse_turtle_star_into<R: Read, S: crate::sparql_library::quin_sink::QuinSink>(
    reader: R,
    context_hash: u64,
    sink: &mut S,
) -> Result<u64, Box<dyn std::error::Error>> {
    let mut parser = TurtleStarParser::new(context_hash);
    let mut count = 0;
    let buf_reader = BufReader::new(reader);

    for line in buf_reader.lines() {
        let line = line?;
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') || l.starts_with('@') {
            continue;
        }

        // Convert to bytes for parser
        let bytes = l.as_bytes();

        // Check if line contains embedded triple marker
        if l.contains("<<") {
            // Parse the full annotation form `<< s p o >> annP annO` in one pass.
            if let Ok((virtual_id, components, annotation)) = parser.parse_star_annotation(bytes) {
                // Linking statement: the quoted triple's virtual id is the subject
                // of the outer annotation (parity with the N-Quads-Star parser).
                // This is what lets `lookup_embedded_triple` — and hence the
                // result serializer — resolve `<<s p o>>` instead of falling back
                // to a raw `<<{hex}>>` placeholder. Bare `<< s p o >>` lines with
                // no outer predicate/object yield `None` and just emit the inner
                // triple, as before.
                if let Some((ann_predicate, ann_object)) = annotation {
                    sink.push(NQuin {
                        subject: virtual_id,
                        predicate: ann_predicate,
                        object: ann_object,
                        context: context_hash,
                        metadata: 0b10 << 61,
                        parity: 0,
                    })?;
                    count += 1;
                }

                // Inner triple (its components), so the quoted triple is itself
                // a real, queryable statement in the graph.
                sink.push(NQuin {
                    subject: components[0],
                    predicate: components[1],
                    object: components[2],
                    context: context_hash, // Context in NQuin field, not Virtual ID
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;

                // NOTE: the durable `virtual_id -> [s,p,o]` lexicon entry
                // (`LexiconEntry::EmbeddedTriple`) is written by the volume
                // builder (`q42/q42_volume.rs`), which now sees the virtual id via
                // the linking statement above; the streaming sink itself has no
                // embedded-entry channel yet (a future expansion).
            }
        } else {
            // Parse regular triple
            if let Ok((subject, predicate, object)) = parser.parse_triple(bytes) {
                sink.push(NQuin {
                    subject,
                    predicate,
                    object,
                    context: context_hash,
                    metadata: 0b10 << 61,
                    parity: 0,
                })?;
                count += 1;
            }
        }
    }

    Ok(count)
}

pub fn parse_turtle_star_stream<R: Read>(
    reader: R,
    context_hash: u64,
    sorter: &mut crate::external_sort::ExternalSorter,
) -> Result<u64, Box<dyn std::error::Error>> {
    parse_turtle_star_into(reader, context_hash, sorter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rdf_star::RdfStarParser;

    #[test]
    fn test_turtle_star_parser_creation() {
        let parser = TurtleStarParser::new(0);
        assert_eq!(parser.format_name(), "Turtle-Star");
        assert!(!parser.supports_quads());
        assert!(!parser.supports_named_graphs());
        assert_eq!(parser.stack.depth(), 0);
    }

    #[test]
    fn test_parser_stack_push_pop() {
        let mut stack = ParserStack::new();
        assert!(stack.is_empty());

        let frame = StackFrame::new();
        stack.push(frame).unwrap();
        assert_eq!(stack.depth(), 1);
        assert!(!stack.is_empty());

        let popped = stack.pop();
        assert!(popped.is_some());
        assert!(stack.is_empty());
    }

    #[test]
    fn test_parser_stack_overflow() {
        let mut stack = ParserStack::new();
        let frame = StackFrame::new();

        // Fill to capacity
        for _ in 0..MAX_NESTING_DEPTH {
            stack.push(frame).unwrap();
        }

        // Should overflow
        assert!(stack.push(frame).is_err());
    }

    #[test]
    fn test_parse_simple_triple() {
        let mut parser = TurtleStarParser::new(0);
        let input = b"Alice knows Bob";
        let result = parser.parse_triple(input);
        assert!(result.is_ok());
        let (s, p, o) = result.unwrap();
        assert_ne!(s, 0);
        assert_ne!(p, 0);
        assert_ne!(o, 0);
    }

    #[test]
    fn test_parse_embedded_triple() {
        let mut parser = TurtleStarParser::new(0);
        let input = b"<< Alice knows Bob >>"; // Simplified for testing
        let result = parser.parse_embedded_triple(input);
        assert!(result.is_ok());
        let (virtual_id, components) = result.unwrap();
        assert_ne!(virtual_id, 0);
        assert_ne!(components[0], 0);
        assert_ne!(components[1], 0);
        assert_ne!(components[2], 0);

        // Verify stack was properly managed
        assert_eq!(parser.stack.depth(), 0);
    }

    struct VecSink(Vec<NQuin>);
    impl crate::sparql_library::quin_sink::QuinSink for VecSink {
        fn push(&mut self, q: NQuin) -> std::io::Result<()> {
            self.0.push(q);
            Ok(())
        }
    }

    #[test]
    fn parse_star_annotation_captures_outer_predicate_object() {
        let mut parser = TurtleStarParser::new(0);
        let (virtual_id, components, ann) = parser
            .parse_star_annotation(b"<< Alice knows Bob >> certainty high")
            .unwrap();
        assert_eq!(
            virtual_id,
            generate_embedded_triple_id(components[0], components[1], components[2])
        );
        let (ann_p, ann_o) = ann.expect("outer annotation predicate/object");
        assert_ne!(ann_p, 0);
        assert_ne!(ann_o, 0);
        assert_eq!(parser.stack.depth(), 0);
    }

    #[test]
    fn parse_turtle_star_into_emits_linking_statement_and_inner_triple() {
        let mut sink = VecSink(Vec::new());
        let count =
            parse_turtle_star_into(&b"<< Alice knows Bob >> certainty high\n"[..], 0, &mut sink)
                .unwrap();
        assert_eq!(count, 2, "expected outer linking statement + inner triple");
        // sink[0] is the outer statement; its subject must be the virtual id of
        // the inner triple (sink[1]) so lookup_embedded_triple can resolve it.
        let inner = sink.0[1];
        let vid = generate_embedded_triple_id(inner.subject, inner.predicate, inner.object);
        assert_eq!(sink.0[0].subject, vid);
    }

    #[test]
    fn parse_turtle_star_into_bare_embedded_triple_emits_only_inner() {
        let mut sink = VecSink(Vec::new());
        let count = parse_turtle_star_into(&b"<< Alice knows Bob >>\n"[..], 0, &mut sink).unwrap();
        assert_eq!(count, 1, "bare embedded triple → just the inner triple");
    }

    #[test]
    fn test_virtual_id_context_independence() {
        use crate::lexicon::TAG_EMBEDDED;

        // Same triple should generate same Virtual ID regardless of context
        let context1 = 12345u64;
        let context2 = 67890u64;

        let mut parser1 = TurtleStarParser::new(context1);
        let mut parser2 = TurtleStarParser::new(context2);

        let input = b"<< Alice knows Bob >>";
        let (vid1, _) = parser1.parse_embedded_triple(input).unwrap();
        let (vid2, _) = parser2.parse_embedded_triple(input).unwrap();

        assert_eq!(vid1, vid2, "Virtual ID should be context-independent");
        assert_ne!(vid1 & TAG_EMBEDDED, 0, "TAG_EMBEDDED bit should be set");
    }
}

/// Turtle-Star Serializer
///
/// Converts Virtual IDs and component hashes back to Turtle-Star syntax.
pub struct TurtleStarSerializer;

impl TurtleStarSerializer {
    pub fn new() -> Self {
        Self
    }
}

impl crate::rdf_star::RdfStarSerializer for TurtleStarSerializer {
    fn serialize_embedded_triple(
        &self,
        _virtual_id: u64,
        components: &[u64; 3],
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format: <<subject predicate object>>
        // For now, just return a placeholder since we need the actual string values
        // TODO: This requires lexicon lookup to get actual IRI strings
        let output = format!("<<{} {} {}>>", components[0], components[1], components[2]);
        Ok(output.into_bytes())
    }

    fn serialize_triple(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format: subject predicate object .
        let output = format!("{} {} {} .", subject, predicate, object);
        Ok(output.into_bytes())
    }

    fn serialize_quad(
        &self,
        _subject: u64,
        _predicate: u64,
        _object: u64,
        _graph: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Turtle-Star doesn't support quads natively
        Err(crate::rdf_star::RdfStarSerializeError::UnsupportedFeature)
    }

    fn supports_quads(&self) -> bool {
        false
    }

    fn format_name(&self) -> &'static str {
        "Turtle-Star"
    }
}

/// CBOR-LD Serializer for SPARQL-Star
///
/// Implements CBOR-LD tags 103-106 for embedded triples per the RDF-Star CBOR-LD spec:
/// - Tag 103: Triple (<<s p o>>)
/// - Tag 104: Subject (s of <<s p o>>)
/// - Tag 105: Predicate (p of <<s p o>>)
/// - Tag 106: Object (o of <<s p o>>)
pub struct CborLdStarSerializer;

impl CborLdStarSerializer {
    pub fn new() -> Self {
        Self
    }
}

impl crate::rdf_star::RdfStarSerializer for CborLdStarSerializer {
    fn serialize_embedded_triple(
        &self,
        _virtual_id: u64,
        components: &[u64; 3],
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // CBOR-LD Tag 103: Triple
        // Format: 103(3-array of [subject, predicate, object])
        use ciborium::ser;

        let mut buffer = Vec::new();
        let tagged = ciborium::tag::Required::<[u64; 3], 103>(*components);
        ser::into_writer(&tagged, &mut buffer)
            .map_err(|_| crate::rdf_star::RdfStarSerializeError::BufferTooSmall)?;

        Ok(buffer)
    }

    fn serialize_triple(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        use ciborium::ser;
        let mut buffer = Vec::new();
        ser::into_writer(&[subject, predicate, object], &mut buffer)
            .map_err(|_| crate::rdf_star::RdfStarSerializeError::BufferTooSmall)?;
        Ok(buffer)
    }

    fn serialize_quad(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
        graph: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        use ciborium::ser;
        let mut buffer = Vec::new();
        ser::into_writer(&[subject, predicate, object, graph], &mut buffer)
            .map_err(|_| crate::rdf_star::RdfStarSerializeError::BufferTooSmall)?;
        Ok(buffer)
    }

    fn supports_quads(&self) -> bool {
        true
    }

    fn format_name(&self) -> &'static str {
        "CBOR-LD-Star"
    }
}

#[cfg(test)]
mod cbor_serializer_tests {
    use super::*;
    use crate::rdf_star::RdfStarSerializer;

    #[test]
    fn test_cbor_serializer_creation() {
        let serializer = CborLdStarSerializer::new();
        assert_eq!(serializer.format_name(), "CBOR-LD-Star");
        assert!(serializer.supports_quads());
    }

    #[test]
    fn test_serialize_embedded_triple() {
        let serializer = CborLdStarSerializer::new();
        let components = [1u64, 2, 3];
        let result = serializer.serialize_embedded_triple(0, &components);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        // CBOR tag 103 encodes as 0xd8 (major-type 6, one-byte follows) + 0x67 (value 103)
        assert_eq!(bytes[0], 0xd8);
        assert_eq!(bytes[1], 0x67);
    }

    #[test]
    fn test_serialize_triple() {
        let serializer = CborLdStarSerializer::new();
        let result = serializer.serialize_triple(1, 2, 3);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        // Should be CBOR array of 3 integers
        assert_eq!(bytes[0], 0x83); // Array of 3 in CBOR
    }

    #[test]
    fn test_serialize_quad() {
        let serializer = CborLdStarSerializer::new();
        let result = serializer.serialize_quad(1, 2, 3, 4);
        assert!(result.is_ok());
        let bytes = result.unwrap();
        // Should be CBOR array of 4 integers
        assert_eq!(bytes[0], 0x84); // Array of 4 in CBOR
    }
}

/// N-Triples-Star Serializer
///
/// Serializes to N-Triples-Star format: <<<s p o>>> p o .
pub struct NTriplesStarSerializer;

impl NTriplesStarSerializer {
    pub fn new() -> Self {
        Self
    }
}

impl crate::rdf_star::RdfStarSerializer for NTriplesStarSerializer {
    fn serialize_embedded_triple(
        &self,
        _virtual_id: u64,
        components: &[u64; 3],
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format: <<<subject predicate object>>>
        // TODO: Should output full IRIs, not just hashes
        let output = format!(
            "<<<{} {} {}>>>",
            components[0], components[1], components[2]
        );
        Ok(output.into_bytes())
    }

    fn serialize_triple(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format: <subject> <predicate> <object> .
        let output = format!("<{}> <{}> <{}> .", subject, predicate, object);
        Ok(output.into_bytes())
    }

    fn serialize_quad(
        &self,
        _subject: u64,
        _predicate: u64,
        _object: u64,
        _graph: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // N-Triples-Star doesn't support quads natively
        Err(crate::rdf_star::RdfStarSerializeError::UnsupportedFeature)
    }

    fn supports_quads(&self) -> bool {
        false
    }

    fn format_name(&self) -> &'static str {
        "N-Triples-Star"
    }
}

/// N-Quads-Star Serializer
///
/// Serializes to N-Quads-Star format: <<<s p o>>> p o <g> .
pub struct NQuadsStarSerializer;

impl NQuadsStarSerializer {
    pub fn new() -> Self {
        Self
    }
}

impl crate::rdf_star::RdfStarSerializer for NQuadsStarSerializer {
    fn serialize_embedded_triple(
        &self,
        _virtual_id: u64,
        components: &[u64; 3],
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format: <<<subject predicate object>>>
        // TODO: Should output full IRIs, not just hashes
        let output = format!(
            "<<<{} {} {}>>>",
            components[0], components[1], components[2]
        );
        Ok(output.into_bytes())
    }

    fn serialize_triple(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format: <subject> <predicate> <object> <graph> .
        // For triple serialization, graph is 0
        let output = format!("<{}> <{}> <{}> .", subject, predicate, object);
        Ok(output.into_bytes())
    }

    fn serialize_quad(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
        graph: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format: <subject> <predicate> <object> <graph> .
        let output = format!("<{}> <{}> <{}> <{}> .", subject, predicate, object, graph);
        Ok(output.into_bytes())
    }

    fn supports_quads(&self) -> bool {
        true
    }

    fn format_name(&self) -> &'static str {
        "N-Quads-Star"
    }
}

/// JSON-LD Serializer for SPARQL-Star
///
/// Serializes to JSON-LD format with @annotation for embedded triples.
pub struct JsonLdStarSerializer;

impl JsonLdStarSerializer {
    pub fn new() -> Self {
        Self
    }
}

impl crate::rdf_star::RdfStarSerializer for JsonLdStarSerializer {
    fn serialize_embedded_triple(
        &self,
        _virtual_id: u64,
        components: &[u64; 3],
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // JSON-LD format with @annotation
        // {
        //   "@id": "_:b1",
        //   "@annotation": {
        //     "@id": "_:b2",
        //     "@type": "@id",
        //     "@value": { "subject": s, "predicate": p, "object": o }
        //   }
        // }
        // TODO: Should output full IRIs, not just hashes
        let output = format!(
            r#"{{
  "@annotation": {{
    "subject": {},
    "predicate": {},
    "object": {}
  }}
}}"#,
            components[0], components[1], components[2]
        );
        Ok(output.into_bytes())
    }

    fn serialize_triple(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Simple JSON-LD triple
        let output = format!(
            r#"{{
  "@id": "_:{}",
  "@type": "_:{}",
  "@value": "_:{}"
}}"#,
            subject, predicate, object
        );
        Ok(output.into_bytes())
    }

    fn serialize_quad(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
        graph: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // JSON-LD quad with @graph
        let output = format!(
            r#"{{
  "@id": "_:{}",
  "@type": "_:{}",
  "@value": "_:{}",
  "@graph": "_:{}"
}}"#,
            subject, predicate, object, graph
        );
        Ok(output.into_bytes())
    }

    fn supports_quads(&self) -> bool {
        true
    }

    fn format_name(&self) -> &'static str {
        "JSON-LD-Star"
    }
}

#[cfg(test)]
mod additional_serializer_tests {
    use super::*;
    use crate::rdf_star::RdfStarSerializer;

    #[test]
    fn test_ntriples_serializer() {
        let serializer = NTriplesStarSerializer::new();
        assert_eq!(serializer.format_name(), "N-Triples-Star");
        assert!(!serializer.supports_quads());

        let components = [1u64, 2, 3];
        let result = serializer.serialize_embedded_triple(0, &components);
        assert!(result.is_ok());
        let output = String::from_utf8(result.unwrap()).unwrap();
        assert!(output.starts_with("<<<"));
    }

    #[test]
    fn test_nquads_serializer() {
        let serializer = NQuadsStarSerializer::new();
        assert_eq!(serializer.format_name(), "N-Quads-Star");
        assert!(serializer.supports_quads());

        let result = serializer.serialize_quad(1, 2, 3, 4);
        assert!(result.is_ok());
        let output = String::from_utf8(result.unwrap()).unwrap();
        assert!(output.contains("<4>"));
    }

    #[test]
    fn test_jsonld_serializer() {
        let serializer = JsonLdStarSerializer::new();
        assert_eq!(serializer.format_name(), "JSON-LD-Star");
        assert!(serializer.supports_quads());

        let components = [1u64, 2, 3];
        let result = serializer.serialize_embedded_triple(0, &components);
        assert!(result.is_ok());
        let output = String::from_utf8(result.unwrap()).unwrap();
        assert!(output.contains("@annotation"));
    }
}

/// Trig-Star Serializer
///
/// Serializes to Trig-Star format with named graphs.
pub struct TrigStarSerializer {
    current_graph: u64,
}

impl TrigStarSerializer {
    pub fn new() -> Self {
        Self { current_graph: 0 }
    }

    pub fn set_current_graph(&mut self, graph_hash: u64) {
        self.current_graph = graph_hash;
    }
}

impl crate::rdf_star::RdfStarSerializer for TrigStarSerializer {
    fn serialize_embedded_triple(
        &self,
        _virtual_id: u64,
        components: &[u64; 3],
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format: <<subject predicate object>>
        let output = format!("<<{} {} {}>>", components[0], components[1], components[2]);
        Ok(output.into_bytes())
    }

    fn serialize_triple(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format: subject predicate object .
        let output = format!("{} {} {} .", subject, predicate, object);
        Ok(output.into_bytes())
    }

    fn serialize_quad(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
        graph: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format in default graph
        if graph == 0 {
            let output = format!("{} {} {} .", subject, predicate, object);
            return Ok(output.into_bytes());
        }
        // Format in named graph (would need GRAPH {} wrapper)
        let output = format!(
            "GRAPH <{}> {{ {} {} {} . }}",
            graph, subject, predicate, object
        );
        Ok(output.into_bytes())
    }

    fn supports_quads(&self) -> bool {
        true
    }

    fn format_name(&self) -> &'static str {
        "Trig-Star"
    }
}

#[cfg(test)]
mod trig_serializer_tests {
    use super::*;
    use crate::rdf_star::RdfStarSerializer;

    #[test]
    fn test_trig_serializer() {
        let serializer = TrigStarSerializer::new();
        assert_eq!(serializer.format_name(), "Trig-Star");
        assert!(serializer.supports_quads());

        let result = serializer.serialize_quad(1, 2, 3, 0);
        assert!(result.is_ok());
    }
}

/// N3-Star Serializer
///
/// Serializes to N3-Star format with formulae and rules support.
pub struct N3StarSerializer {
    /// Current variable bindings
    variables: std::collections::HashMap<u64, String>,
}

impl N3StarSerializer {
    pub fn new() -> Self {
        Self {
            variables: std::collections::HashMap::new(),
        }
    }

    pub fn bind_variable(&mut self, hash: u64, name: String) {
        self.variables.insert(hash, name);
    }
}

impl crate::rdf_star::RdfStarSerializer for N3StarSerializer {
    fn serialize_embedded_triple(
        &self,
        _virtual_id: u64,
        components: &[u64; 3],
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // Format: <<subject predicate object>>
        let output = format!("<<{} {} {}>>", components[0], components[1], components[2]);
        Ok(output.into_bytes())
    }

    fn serialize_triple(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        let s = self
            .variables
            .get(&subject)
            .cloned()
            .unwrap_or_else(|| format!("_:{}", subject));
        let p = self
            .variables
            .get(&predicate)
            .cloned()
            .unwrap_or_else(|| format!("_:{}", predicate));
        let o = self
            .variables
            .get(&object)
            .cloned()
            .unwrap_or_else(|| format!("_:{}", object));

        let output = format!("{} {} {} .", s, p, o);
        Ok(output.into_bytes())
    }

    fn serialize_quad(
        &self,
        subject: u64,
        predicate: u64,
        object: u64,
        _graph: u64,
    ) -> Result<Vec<u8>, crate::rdf_star::RdfStarSerializeError> {
        // N3 doesn't have named graphs, so serialize as triple
        self.serialize_triple(subject, predicate, object)
    }

    fn supports_quads(&self) -> bool {
        false
    }

    fn format_name(&self) -> &'static str {
        "N3-Star"
    }
}

#[cfg(test)]
mod n3_serializer_tests {
    use super::*;
    use crate::rdf_star::RdfStarSerializer;

    #[test]
    fn test_n3_serializer() {
        let serializer = N3StarSerializer::new();
        assert_eq!(serializer.format_name(), "N3-Star");
        assert!(!serializer.supports_quads());

        let result = serializer.serialize_triple(1, 2, 3);
        assert!(result.is_ok());
    }

    #[test]
    fn test_n3_serializer_variables() {
        let mut serializer = N3StarSerializer::new();
        serializer.bind_variable(1, "x".to_string());
        serializer.bind_variable(2, "knows".to_string());
        serializer.bind_variable(3, "y".to_string());

        let result = serializer.serialize_triple(1, 2, 3);
        assert!(result.is_ok());
        let output = String::from_utf8(result.unwrap()).unwrap();
        assert!(output.contains("x"));
    }
}
