//! Zero-allocation N-Triples pattern compiler.
//!
//! Accepts a single N-Triples pattern line such as
//!   `<http://example.org/Alice> <schema:knows> ?who .`
//! and compiles it into a flat bytecode program that `webizen_bytecode::execute_program`
//! can run directly against a `&[NQuin]` slice without any heap allocation.
//!
//! # Bytecode encoding
//! Each instruction occupies either 1 or 9 bytes:
//!
//! | Opcode byte | Operand (bytes 1-8) | Meaning                       |
//! |-------------|---------------------|-------------------------------|
//! | 0x01        | u64 LE hash         | MatchSubject(hash)            |
//! | 0x02        | u64 LE hash         | MatchPredicate(hash)          |
//! | 0x03        | u64 LE hash         | MatchObject(hash)             |
//! | 0x04        | —                   | HaltIfFalse (skip this Quin)  |
//! | 0x00        | —                   | End / accept this Quin        |
//!
//! Variable tokens (`?name`) produce no instruction — the corresponding
//! Quin vector is treated as a wildcard.

pub const OP_END: u8 = 0x00;
pub const OP_MATCH_SUBJECT: u8 = 0x01;
pub const OP_MATCH_PREDICATE: u8 = 0x02;
pub const OP_MATCH_OBJECT: u8 = 0x03;
pub const OP_HALT_IF_FALSE: u8 = 0x04;

// Sentinel Deontic Logic ISA Extensions
pub const OP_EVAL_PERMIT: u8 = 0x50;
pub const OP_EVAL_OBLIGATE: u8 = 0x51;
pub const OP_EVAL_FORBID: u8 = 0x52;
pub const OP_HALT_VIOLATION: u8 = 0x53;

#[derive(Debug, PartialEq)]
pub enum ParseError {
    /// The query bytes are not valid UTF-8 or contain no recognisable tokens.
    Malformed,
    /// The compiled program would overflow the 1 KiB fixed program buffer.
    ProgramTooLarge,
}

/// Compile a single N-Triples pattern line into `program`.
///
/// Returns the number of bytes written to `program` on success.
/// Wildcards (`?var`) are silently elided; bound terms are hashed with FNV-1a
/// so the VM can compare directly against `NQuin` field values.
pub fn compile_ntriples_to_bytecode(
    query: &[u8],
    program: &mut [u8; 1024],
) -> Result<usize, ParseError> {
    let text = core::str::from_utf8(query).map_err(|_| ParseError::Malformed)?;

    let mut pos = 0usize; // write cursor into program
    let mut token_idx = 0usize; // 0 = subject, 1 = predicate, 2 = object

    for token in text.split_whitespace() {
        if token == "." {
            break;
        }
        if token_idx > 2 {
            break;
        }

        let opcode = match token_idx {
            0 => OP_MATCH_SUBJECT,
            1 => OP_MATCH_PREDICATE,
            _ => OP_MATCH_OBJECT,
        };
        token_idx += 1;

        // Variables are wildcards — no constraint emitted.
        if token.starts_with('?') {
            continue;
        }

        let hash = hash_token(token);

        // 9 bytes for the match instruction + 1 byte for HaltIfFalse + 1 byte reserved for END.
        if pos + 11 > program.len() {
            return Err(ParseError::ProgramTooLarge);
        }

        program[pos] = opcode;
        program[pos + 1..pos + 9].copy_from_slice(&hash.to_le_bytes());
        pos += 9;

        program[pos] = OP_HALT_IF_FALSE;
        pos += 1;
    }

    if token_idx == 0 {
        return Err(ParseError::Malformed);
    }

    program[pos] = OP_END;
    pos += 1;
    Ok(pos)
}

/// Resolve a single N-Triples token to its 64-bit Quin vector value.
///
/// Routing rules (applied after bracket-stripping):
/// 1. If the inner URI starts with `did:q42:` → route through
///    [`crate::identifier::parse_did_q42`], which sets bit 63 to mark the
///    result as a topological hardware pointer.
/// 2. All other URIs and literals → standard `q_hash` (bit 63 = 0).
///
/// This function is `pub` so the CLI ingest pipeline and the WASM bridge can
/// hash tokens with identical logic without duplicating the dispatch table.
#[inline(always)]
pub fn hash_token(token: &str) -> u64 {
    // Strip angle-bracket URI delimiters and double-quote literal delimiters.
    // For literals we scan for the first unescaped closing `"` so that
    // language-tagged (`"val"@lang`) and datatype-tagged (`"val"^^<dt>`)
    // literals are treated identically to plain `"val"` — only the value
    // itself is hashed.  This matches hash.js `hashToken` exactly.
    let inner = if token.starts_with('<') && token.ends_with('>') {
        &token[1..token.len() - 1]
    } else if token.starts_with('"') {
        let bytes = token.as_bytes();
        let mut i = 1;
        while i < bytes.len() {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'"' {
                break;
            }
            i += 1;
        }
        &token[1..i]
    } else {
        token
    };

    // Route `did:q42:` coordinates through the identifier module so the MSB
    // flag is applied, distinguishing them from plain dictionary hashes.
    if inner.as_bytes().starts_with(b"did:q42:") {
        if let Ok(pointer) = crate::identifier::parse_did_q42(inner.as_bytes()) {
            return pointer;
        }
        // Malformed did:q42 URI — fall through to standard hash so the query
        // can still execute; the Webizen VM will simply find no match.
    }

    crate::q_hash(inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_bound_triple() {
        let mut prog = [0u8; 1024];
        let n = compile_ntriples_to_bytecode(b"<Alice> <knows> <Bob> .", &mut prog).unwrap();
        // 3 × (9 match + 1 halt) + 1 end = 31 bytes
        assert_eq!(n, 31);
        assert_eq!(prog[0], OP_MATCH_SUBJECT);
        assert_eq!(prog[9], OP_HALT_IF_FALSE);
        assert_eq!(prog[10], OP_MATCH_PREDICATE);
        assert_eq!(prog[19], OP_HALT_IF_FALSE);
        assert_eq!(prog[20], OP_MATCH_OBJECT);
        assert_eq!(prog[29], OP_HALT_IF_FALSE);
        assert_eq!(prog[30], OP_END);
    }

    #[test]
    fn compile_wildcard_subject() {
        let mut prog = [0u8; 1024];
        let n = compile_ntriples_to_bytecode(b"?who <knows> <Bob> .", &mut prog).unwrap();
        // Subject is wildcard → 2 × (9 + 1) + 1 = 21 bytes
        assert_eq!(n, 21);
        assert_eq!(prog[0], OP_MATCH_PREDICATE);
    }

    #[test]
    fn compile_empty_fails() {
        let mut prog = [0u8; 1024];
        assert_eq!(
            compile_ntriples_to_bytecode(b"   ", &mut prog),
            Err(ParseError::Malformed)
        );
    }

    #[test]
    fn hashes_strip_brackets() {
        assert_eq!(hash_token("<Alice>"), hash_token("Alice"));
        assert_eq!(hash_token("\"hello\""), hash_token("hello"));
    }

    #[test]
    fn did_q42_sets_msb_in_compiled_bytecode() {
        let mut prog = [0u8; 1024];
        // The subject is a did:q42 coordinate; predicate and object are plain URIs.
        compile_ntriples_to_bytecode(b"<did:q42:z6MkpTHR8VNs> <knows> <Bob> .", &mut prog).unwrap();

        // Subject hash is in bytes 1–8 (immediately after OP_MATCH_SUBJECT).
        let subject_hash = u64::from_le_bytes(prog[1..9].try_into().unwrap());
        assert_eq!(
            subject_hash >> 63,
            1,
            "did:q42 subject must have MSB set in compiled bytecode"
        );

        // The predicate hash must equal q_hash("knows") | (1<<63) when that hash
        // already has MSB set, or just q_hash("knows") when it does not.
        // The contract we verify here is only about the subject's MSB flag;
        // FNV-1a can naturally produce MSB=1 for any input, so we do not assert
        // MSB=0 for plain URI tokens.
        let subject_expected = crate::q_hash("z6MkpTHR8VNs") | (1u64 << 63);
        assert_eq!(subject_hash, subject_expected);
    }

    #[test]
    fn did_q42_in_object_position() {
        let mut prog = [0u8; 1024];
        compile_ntriples_to_bytecode(b"?who <knows> <did:q42:z6MkAbCd> .", &mut prog).unwrap();
        // With wildcard subject: OP_MATCH_PREDICATE at 0, object at 10.
        let object_hash = u64::from_le_bytes(prog[11..19].try_into().unwrap());
        assert_eq!(object_hash >> 63, 1, "did:q42 object must have MSB set");
    }
}
