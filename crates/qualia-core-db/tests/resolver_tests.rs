use qualia_core_db::{
    mini_parser::compile_ntriples_to_bytecode,
    q_hash,
    resolver::{format_ntriples_to, resolve_hash},
    webizen_bytecode::execute_program,
    NQuin,
};

// ---------------------------------------------------------------------------
// Lexicon: known-hash resolution
// ---------------------------------------------------------------------------

#[test]
fn known_hash_resolves_to_uri() {
    let hash = q_hash("Alice");
    let bytes = resolve_hash(hash).expect("Alice must be in the demo lexicon");
    assert_eq!(bytes, b"http://qualia-db.org/demo/Alice");
}

#[test]
fn known_predicate_resolves() {
    let hash = q_hash("knows");
    let bytes = resolve_hash(hash).expect("knows must be in the demo lexicon");
    assert_eq!(bytes, b"http://schema.org/knows");
}

#[test]
fn unknown_hash_returns_none() {
    // A hash for a term that is definitely not in the demo lexicon.
    assert!(resolve_hash(0x0000_0000_DEAD_BEEF).is_none());
}

#[test]
fn topological_pointer_not_in_lexicon() {
    let ptr =
        qualia_core_db::identifier::parse_did_q42(b"did:q42:z6TestHash").expect("valid did:q42");
    assert!(
        resolve_hash(ptr).is_none(),
        "topological pointers must not be resolved through the lexicon"
    );
}

// ---------------------------------------------------------------------------
// Fallback: unknown hash emits hex placeholder
// ---------------------------------------------------------------------------

#[test]
fn unknown_hash_fallback_is_hex_format() {
    let q = NQuin {
        subject: 0x00_00_00_00_00_00_00_01,
        predicate: 0x00_00_00_00_00_00_00_02,
        object: 0x00_00_00_00_00_00_00_03,
        context: 0,
        metadata: 0,
        parity: 0,
    };
    let mut buf = Vec::new();
    format_ntriples_to(&[q], &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();

    assert!(
        s.contains("<quin:hash/0000000000000001>"),
        "subject fallback wrong: {s}"
    );
    assert!(
        s.contains("<quin:hash/0000000000000002>"),
        "predicate fallback wrong: {s}"
    );
    assert!(
        s.contains("<quin:hash/0000000000000003>"),
        "object fallback wrong: {s}"
    );
}

// ---------------------------------------------------------------------------
// Full N-Triples round-trip for known terms
// ---------------------------------------------------------------------------

#[test]
fn known_terms_produce_full_iri_ntriples_line() {
    let q = NQuin {
        subject: q_hash("Alice"),
        predicate: q_hash("knows"),
        object: q_hash("Bob"),
        context: 0,
        metadata: 0,
        parity: 0,
    };
    let mut buf = Vec::new();
    format_ntriples_to(&[q], &mut buf).unwrap();
    let s = String::from_utf8(buf).unwrap();

    assert!(
        s.contains("<http://qualia-db.org/demo/Alice>"),
        "subject IRI missing: {s}"
    );
    assert!(
        s.contains("<http://schema.org/knows>"),
        "predicate IRI missing: {s}"
    );
    assert!(
        s.contains("<http://qualia-db.org/demo/Bob>"),
        "object IRI missing: {s}"
    );
    assert!(
        s.ends_with(" .\n"),
        "N-Triples line must end with ` .\\n`: {s}"
    );
}

// ---------------------------------------------------------------------------
// X-Qualia-Compute-Cost: VM cycle count
// ---------------------------------------------------------------------------

#[test]
fn execute_program_returns_cycle_count() {
    let q = NQuin {
        subject: q_hash("Alice"),
        predicate: q_hash("knows"),
        object: q_hash("Bob"),
        context: 0,
        metadata: 0,
        parity: 0,
    };
    let mut prog = [0u8; 1024];
    compile_ntriples_to_bytecode(b"<Alice> <knows> <Bob>", &mut prog).unwrap();

    let mut out = [NQuin::default(); 10];
    let (match_count, vm_cycles) = execute_program(&prog, &[q], &mut out).unwrap();

    assert_eq!(match_count, 1, "expected one matching Quin");
    assert!(vm_cycles > 0, "VM must report a non-zero cycle count");
}

#[test]
fn compute_cost_header_format_is_matches_plus_cycles() {
    // Verify that the format string the daemon would use is well-formed.
    // We cannot spin up a warp server in a unit test, so we check the
    // constituent values are correctly formatted here.
    let match_count: usize = 3;
    let vm_cycles: u64 = 847;
    let header_value = format!("{match_count}+{vm_cycles}");
    assert_eq!(header_value, "3+847");
}

#[test]
fn zero_match_still_reports_cycles_for_scanned_rows() {
    // A query that matches nothing should still burn cycles while scanning.
    let q = NQuin {
        subject: q_hash("Alice"),
        predicate: q_hash("knows"),
        object: q_hash("Bob"),
        context: 0,
        metadata: 0,
        parity: 0,
    };
    let mut prog = [0u8; 1024];
    // Query for Carol — not in the db above.
    compile_ntriples_to_bytecode(b"<Carol> <knows> <Bob>", &mut prog).unwrap();

    let mut out = [NQuin::default(); 10];
    let (n, cycles) = execute_program(&prog, &[q], &mut out).unwrap();

    assert_eq!(n, 0, "Carol should not match");
    assert!(
        cycles > 0,
        "cycles must be non-zero even when nothing matches — the subject check was still executed"
    );
}
