//! Capstone — the whole hybrid-identity stack resolving one identifier end-to-end.
//!
//! Over the REAL ILO C105 forced-labour concept, in a single flow, this exercises every
//! piece built this session:
//!   turtle_doc ingest  →  LexiconInterner (collision-aware)  →  q_hash == generate_60bit_token
//!   (#14)  →  modal_kind tag + resolve (full-width identifier kinds)  →  zero-alloc QuinIndex
//!   resolution (resolve.rs)  →  deontic VM governance  →  lexicon value recovery.
//!
//! If this passes, an identifier ingested from a human-rights instrument can be resolved
//! to its kind, its relations, the human-readable value behind its handle, and the norm
//! that governs it — through one consistent path.

use qualia_core_db::indexing::QuinIndex;
use qualia_core_db::lexicon::LexiconInterner;
use qualia_core_db::modal_kind::{kind_name, tag_kind, KIND_DICTIONARY};
use qualia_core_db::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_OBLIGATE,
};
use qualia_core_db::resolve::resolve_in_index;
use qualia_core_db::sparql_library::parsers::turtle_doc::parse_turtle_doc_into;
use qualia_core_db::sparql_library::quin_sink::QuinSink;
use qualia_core_db::{q_hash, NQuin};
use std::io;

const CONCEPT_N3: &str =
    include_str!("../../../core-ontologies/concepts/duty-to-suppress-forced-labour.n3");

/// A sink that does what real ingest does: collect quins AND intern terms into the
/// collision-aware lexicon (turtle_doc calls `push_lex` for every term).
#[derive(Default)]
struct CapstoneSink {
    quins: Vec<NQuin>,
    lex: LexiconInterner,
}
impl QuinSink for CapstoneSink {
    fn push(&mut self, q: NQuin) -> io::Result<()> {
        self.quins.push(q);
        Ok(())
    }
    fn push_lex(&mut self, hash: u64, term: &str) {
        self.lex.intern(hash, term);
    }
}

#[test]
fn whole_stack_resolves_one_identifier_end_to_end() {
    // ── 1. Ingest the real instrument concept (generate_60bit_token + lexicon intern). ──
    let mut sink = CapstoneSink::default();
    parse_turtle_doc_into(CONCEPT_N3.as_bytes(), 0, &mut sink).expect("concept ingests");
    let CapstoneSink { mut quins, lex } = sink;
    assert!(!quins.is_empty(), "ingest produced no quins");

    // ── 2. Tag the obligation node with a modal identifier-KIND (open kind fabric). ──
    let deontic = q_hash("https://ns.webcivics.net/concept/DTSFL-deontic");
    quins.push(tag_kind(deontic, KIND_DICTIONARY));

    // ── 3. Build the zero-alloc index and resolve the identifier through resolve.rs. ──
    let idx = QuinIndex::from_slice(&quins);
    let resolved = resolve_in_index(&idx, deontic);
    assert_eq!(resolved.kind, Some(KIND_DICTIONARY), "modal kind resolves");
    assert_eq!(kind_name(resolved.kind.unwrap()), Some("DictionaryHash"));
    assert!(
        resolved.out_degree >= 4,
        "obligation node has its relations + the kind edge (got {})",
        resolved.out_degree
    );

    // ── 4. #14 bridge: read the bearer + required act OUT OF the ingested corpus. ──
    let borne_by = q_hash("https://ns.webcivics.net/values/borneBy");
    let requires = q_hash("https://ns.webcivics.net/values/requires");
    let party = idx
        .object_of(deontic, borne_by)
        .expect("bearer found in corpus");
    let action = idx
        .object_of(deontic, requires)
        .expect("required act found in corpus");
    assert_eq!(party, q_hash("https://ns.webcivics.net/values/State"));
    assert_eq!(
        action,
        q_hash("https://ns.webcivics.net/action/SuppressForcedLabour")
    );

    // ── 5. Deontic governance: the State's obligation is Active over corpus terms. ──
    let contract = q_hash("contract:capstone:ilo-c105");
    let norm = compile_norm_quin(party, OP_OBLIGATE, action, action, contract, 0, false);
    let mut out = [DeonticVerdict::default(); 2];
    let n = evaluate_deontic_contract(&[norm], 1_717_200_000u32, &mut out).expect("deontic eval");
    assert_eq!(n, 1);
    assert_eq!(
        out[0].status,
        DeonticStatus::Active,
        "the State's obligation to suppress forced labour is in force"
    );

    // ── 6. Lexicon value recovery: the handle resolves to its human-readable IRI. ──
    assert_eq!(
        lex.resolve(party),
        Some("https://ns.webcivics.net/values/State"),
        "the bearer handle recovers its lexical value through the lexicon backstop"
    );
    assert_eq!(
        lex.resolve(action),
        Some("https://ns.webcivics.net/action/SuppressForcedLabour")
    );
}
