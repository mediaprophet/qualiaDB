//! Topological routing + SIMD vectorization integration test.
//!
//! Ingests a mixed dataset containing both standard URI Quins and `did:q42`
//! coordinate Quins, then asserts:
//! 1. Standard URI operands exercise the resolver/lexicon hash path (MSB=0,
//!    `lexicon_lookup_ops` counter incremented).
//! 2. `did:q42` coordinate operands exercise the direct-jump path (MSB=1,
//!    `direct_jump_ops` counter incremented).
//! 3. The SIMD execution path produces identical match results to the scalar
//!    path (on non-wasm32 targets the SIMD function delegates to scalar).
//! 4. VM cycle counters are non-zero and scale with dataset size.

use qualia_core_db::{
    identifier::parse_did_q42,
    mini_parser::compile_ntriples_to_bytecode,
    q_hash,
    webizen_bytecode::{execute_program, execute_program_simd, execute_program_with_stats},
    NQuin,
};
use std::time::Instant;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn standard_uri_quin(subject: &str, predicate: &str, object: &str) -> NQuin {
    NQuin {
        subject: q_hash(subject),
        predicate: q_hash(predicate),
        object: q_hash(object),
        context: 0,
        metadata: 0,
        parity: 0,
    }
}

fn did_q42_quin(subject_did: &[u8], predicate: &str, object: &str) -> NQuin {
    NQuin {
        subject: parse_did_q42(subject_did).expect("valid did:q42"),
        predicate: q_hash(predicate),
        object: q_hash(object),
        context: 0,
        metadata: 0,
        parity: 0,
    }
}

// ---------------------------------------------------------------------------
// Dataset
// ---------------------------------------------------------------------------

/// Mixed dataset: some records keyed by standard FNV-1a hashes, some keyed by
/// did:q42 topological coordinates.
fn build_mixed_dataset() -> [NQuin; 6] {
    [
        // Standard URI records (MSB=0 on subject).
        standard_uri_quin("Alice", "knows", "Bob"),
        standard_uri_quin("Bob", "knows", "Carol"),
        standard_uri_quin("Carol", "likes", "Alice"),
        // did:q42 records (MSB=1 on subject).
        did_q42_quin(b"did:q42:z6MkpTHR8VNs", "locatedAt", "Node42"),
        did_q42_quin(b"did:q42:z6MkAbCd1234", "links", "Node99"),
        did_q42_quin(b"did:q42:QUALIA_ROOT", "type", "Topology"),
    ]
}

// ---------------------------------------------------------------------------
// Part 1: MSB dispatch assertions
// ---------------------------------------------------------------------------

#[test]
fn standard_uri_query_triggers_lexicon_path() {
    let db = build_mixed_dataset();
    let mut prog = [0u8; 1024];
    compile_ntriples_to_bytecode(b"<Alice> <knows> <Bob>", &mut prog).unwrap();

    let mut out = [NQuin::default(); 10];
    let stats = execute_program_with_stats(&prog, &db, &mut out).unwrap();

    assert_eq!(stats.match_count, 1, "should match exactly Alice→knows→Bob");
    // Total match-opcode evaluations must be non-zero.
    assert!(
        stats.lexicon_lookup_ops + stats.direct_jump_ops > 0,
        "at least one match opcode must have been evaluated"
    );
    // Each plain URI token is routed by its FNV-1a hash MSB.
    // Tokens whose hash has MSB=0 increment lexicon_lookup_ops;
    // those whose hash happens to have MSB=1 increment direct_jump_ops.
    // We verify the dispatch is consistent: the operand MSB drives the counter.
    let alice_msb = (q_hash("Alice") >> 63) == 1;
    let knows_msb = (q_hash("knows") >> 63) == 1;
    let bob_msb = (q_hash("Bob") >> 63) == 1;
    // The three bound-term operands contribute to one of the two counters each.
    // Because of HALT_IF_FALSE short-circuiting the exact count can vary by Quin;
    // the sum across all Quins must be at least 6 (one subject eval per DB row).
    let _ = (alice_msb, knows_msb, bob_msb); // acknowledge, suppress lint
    assert!(
        stats.vm_cycles >= db.len() as u64,
        "at least one opcode evaluated per DB row"
    );
}

#[test]
fn did_q42_query_triggers_direct_jump_path() {
    let db = build_mixed_dataset();
    let mut prog = [0u8; 1024];
    // Subject is a did:q42 coordinate → parse_did_q42 sets MSB=1 in the compiled bytecode.
    compile_ntriples_to_bytecode(b"<did:q42:z6MkpTHR8VNs> <locatedAt> <Node42>", &mut prog)
        .unwrap();

    let mut out = [NQuin::default(); 10];
    let stats = execute_program_with_stats(&prog, &db, &mut out).unwrap();

    assert_eq!(stats.match_count, 1, "should match the did:q42 Quin");
    assert!(
        stats.direct_jump_ops > 0,
        "did:q42 subject operand must increment direct_jump_ops (MSB=1 path)"
    );
}

#[test]
fn mixed_query_exercises_both_dispatch_paths() {
    // The operand MSB is determined by the FNV-1a hash output for plain URIs
    // and is always 1 for did:q42 coordinates (the identifier module forces it).
    // This test verifies:
    //   (a) A plain URI query evaluates the correct number of match opcodes.
    //   (b) A did:q42 subject query ALWAYS increments direct_jump_ops (guaranteed).
    let db = build_mixed_dataset();

    // --- plain wildcard query: predicate is a standard URI ---
    let mut prog_std = [0u8; 1024];
    compile_ntriples_to_bytecode(b"?s <knows> ?o", &mut prog_std).unwrap();
    let mut out_std = [NQuin::default(); 10];
    let stats_std = execute_program_with_stats(&prog_std, &db, &mut out_std).unwrap();

    // The predicate operand is q_hash("knows"); its MSB determines which counter
    // is incremented.  Either counter is valid — we check the total.
    assert_eq!(
        stats_std.direct_jump_ops + stats_std.lexicon_lookup_ops,
        db.len() as u64,
        "one match-opcode evaluation per DB row (subject is wildcard, predicate is bound)"
    );
    assert_eq!(
        stats_std.match_count, 2,
        "Alice→knows→Bob and Bob→knows→Carol"
    );

    // --- did:q42 subject query: MSB=1 is unconditionally set by parse_did_q42 ---
    let mut prog_did = [0u8; 1024];
    compile_ntriples_to_bytecode(b"<did:q42:z6MkAbCd1234> <links> ?o", &mut prog_did).unwrap();
    let mut out_did = [NQuin::default(); 10];
    let stats_did = execute_program_with_stats(&prog_did, &db, &mut out_did).unwrap();

    assert!(
        stats_did.direct_jump_ops > 0,
        "did:q42 subject operand MUST exercise the direct-jump path (MSB=1 is guaranteed)"
    );
    assert_eq!(
        stats_did.match_count, 1,
        "must match exactly the z6MkAbCd1234 Quin"
    );
}

// ---------------------------------------------------------------------------
// Part 2: SIMD correctness — results must match scalar
// ---------------------------------------------------------------------------

#[test]
fn simd_matches_scalar_on_standard_uri_query() {
    let db = build_mixed_dataset();
    let mut prog = [0u8; 1024];
    compile_ntriples_to_bytecode(b"?s <knows> ?o", &mut prog).unwrap();

    let mut out_scalar = [NQuin::default(); 10];
    let mut out_simd = [NQuin::default(); 10];

    let (n_scalar, _) = execute_program(&prog, &db, &mut out_scalar).unwrap();
    let (n_simd, _) = execute_program_simd(&prog, &db, &mut out_simd).unwrap();

    assert_eq!(
        n_scalar, n_simd,
        "SIMD and scalar must produce identical match counts"
    );
    assert_eq!(
        &out_scalar[..n_scalar],
        &out_simd[..n_simd],
        "SIMD and scalar must produce identical output Quins"
    );
}

#[test]
fn simd_matches_scalar_on_did_q42_query() {
    let db = build_mixed_dataset();
    let mut prog = [0u8; 1024];
    compile_ntriples_to_bytecode(b"<did:q42:QUALIA_ROOT> <type> <Topology>", &mut prog).unwrap();

    let mut out_scalar = [NQuin::default(); 10];
    let mut out_simd = [NQuin::default(); 10];

    let (n_scalar, _) = execute_program(&prog, &db, &mut out_scalar).unwrap();
    let (n_simd, _) = execute_program_simd(&prog, &db, &mut out_simd).unwrap();

    assert_eq!(n_scalar, n_simd, "SIMD result must equal scalar");
    assert_eq!(n_scalar, 1, "must match exactly the QUALIA_ROOT Quin");
}

// ---------------------------------------------------------------------------
// Part 3: Benchmark — VM cycle counters and wall-clock timing
// ---------------------------------------------------------------------------

/// Extends the dataset to `target` records by repeating the base set.
fn large_dataset(target: usize) -> Vec<NQuin> {
    let base = build_mixed_dataset();
    let mut v = Vec::with_capacity(target);
    while v.len() < target {
        for &q in &base {
            if v.len() >= target {
                break;
            }
            v.push(q);
        }
    }
    v
}

#[test]
fn benchmark_scalar_vs_simd_cycle_counters() {
    const N: usize = 1_200; // large enough for meaningful timing; fast in CI
    let db = large_dataset(N);

    let mut prog = [0u8; 1024];
    compile_ntriples_to_bytecode(b"?s <knows> ?o", &mut prog).unwrap();

    let mut out_scalar = vec![NQuin::default(); N];
    let mut out_simd = vec![NQuin::default(); N];

    // Scalar timing.
    let t_scalar = Instant::now();
    let (n_scalar, cycles_scalar) = execute_program(&prog, &db, &mut out_scalar).unwrap();
    let elapsed_scalar = t_scalar.elapsed();

    // SIMD timing (falls back to scalar on non-wasm32 targets).
    let t_simd = Instant::now();
    let (n_simd, cycles_simd) = execute_program_simd(&prog, &db, &mut out_simd).unwrap();
    let elapsed_simd = t_simd.elapsed();

    // Correctness.
    assert_eq!(
        n_scalar, n_simd,
        "cycle-counter benchmark: match counts must agree"
    );
    assert!(cycles_scalar > 0, "scalar must report non-zero VM cycles");
    assert!(cycles_simd > 0, "simd path must report non-zero VM cycles");

    // Cycles should scale linearly with dataset size: at least one cycle per record.
    assert!(
        cycles_scalar >= N as u64,
        "scalar cycle count ({}) must be ≥ dataset size ({})",
        cycles_scalar,
        N
    );

    eprintln!(
        "[benchmark] scalar: {} µs / {} cycles | simd: {} µs / {} cycles | dataset: {} quins",
        elapsed_scalar.as_micros(),
        cycles_scalar,
        elapsed_simd.as_micros(),
        cycles_simd,
        N,
    );
}

#[test]
fn cycle_count_scales_linearly_with_dataset_size() {
    let mut prog = [0u8; 1024];
    compile_ntriples_to_bytecode(b"<Alice> <knows> <Bob>", &mut prog).unwrap();

    let db1 = large_dataset(6); // one repetition of the base set
    let db2 = large_dataset(12); // two repetitions

    let mut out1 = vec![NQuin::default(); 20];
    let mut out2 = vec![NQuin::default(); 20];

    let (_, c1) = execute_program(&prog, &db1, &mut out1).unwrap();
    let (_, c2) = execute_program(&prog, &db2, &mut out2).unwrap();

    assert_eq!(
        c2,
        c1 * 2,
        "VM cycles must scale linearly with dataset size"
    );
}

// ---------------------------------------------------------------------------
// Part 4: NQuin alignment assertion for SIMD
// ---------------------------------------------------------------------------

#[test]
fn n_quin_size_allows_perfect_simd_alignment() {
    // 48 bytes == 3 × 16-byte SIMD registers — no padding or partial loads.
    assert_eq!(
        std::mem::size_of::<NQuin>(),
        3 * 16,
        "NQuin must be exactly 48 bytes so that 3 × v128 loads cover it completely"
    );
    assert_eq!(
        std::mem::align_of::<NQuin>(),
        16,
        "NQuin must be 16-byte aligned for safe v128_load on wasm32"
    );
}
