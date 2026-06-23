//! #7 — the values/deontic logic runs over the INGESTED instrument corpus.
//!
//! This is the end-to-end proof that the hash-space unification (#14) is operational:
//! the corpus enters the graph through the faithful Turtle ingest path, which hashes
//! every term with `generate_60bit_token`; the engine's deontic/values layer builds
//! its terms with `q_hash`. Before #14 those produced different numbers for the same
//! IRI, so an engine-side norm could never "see" an ingested term. After #14 they are
//! one identity space — so a norm compiled with `q_hash` evaluates against terms that
//! literally came out of the `.n3`.
//!
//! Concept under test: ILO Convention 105, Art. 1 →
//! `concept:DutyToSuppressForcedLabour` (core-ontologies/concepts/...).

use qualia_core_db::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_OBLIGATE,
};
use qualia_core_db::sparql_library::parsers::turtle_doc::parse_turtle_doc_into;
use qualia_core_db::sparql_library::quin_sink::QuinSink;
use qualia_core_db::{q_hash, NQuin};
use std::io;

/// The real pilot concept, ingested verbatim by the faithful Turtle pipeline.
const CONCEPT_N3: &str =
    include_str!("../../../core-ontologies/concepts/duty-to-suppress-forced-labour.n3");

#[derive(Default)]
struct VecSink(Vec<NQuin>);
impl QuinSink for VecSink {
    fn push(&mut self, q: NQuin) -> io::Result<()> {
        self.0.push(q);
        Ok(())
    }
}

/// Ingest a Turtle document the way the CLI does (terms hashed via generate_60bit_token).
fn ingest(doc: &str) -> Vec<NQuin> {
    let mut sink = VecSink::default();
    parse_turtle_doc_into(doc.as_bytes(), 0, &mut sink).expect("concept must ingest cleanly");
    sink.0
}

/// First object of (subject, predicate) in the corpus — a one-hop point lookup.
fn object_of(corpus: &[NQuin], s: u64, p: u64) -> Option<u64> {
    corpus
        .iter()
        .find(|q| q.subject == s && q.predicate == p)
        .map(|q| q.object)
}

#[test]
fn values_logic_runs_over_ingested_instrument_corpus() {
    let corpus = ingest(CONCEPT_N3);
    assert!(!corpus.is_empty(), "ingest produced no quins");

    // Engine-side terms: q_hash over the SAME expanded IRIs the corpus stored.
    let deontic = q_hash("https://ns.webcivics.net/concept/DTSFL-deontic");
    let rdf_type = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    let obligation = q_hash("https://ns.webcivics.net/values/Obligation");
    let borne_by = q_hash("https://ns.webcivics.net/values/borneBy");
    let requires = q_hash("https://ns.webcivics.net/values/requires");

    // ── BRIDGE PROOF (#14): the engine's q_hash terms LOCATE the ingested obligation,
    // which entered the graph via generate_60bit_token. Before #14 this join was empty.
    assert_eq!(
        object_of(&corpus, deontic, rdf_type),
        Some(obligation),
        "the obligation's rdf:type must join across the q_hash / generate_60bit_token bridge",
    );

    // Read the bearer + the required act OUT OF THE CORPUS (terms that came from the .n3).
    let party = object_of(&corpus, deontic, borne_by)
        .expect("the obligation's bearer (values:borneBy) must be found in the ingested corpus");
    let action = object_of(&corpus, deontic, requires)
        .expect("the required act (values:requires) must be found in the ingested corpus");

    // …and they ARE the engine's q_hash of the same IRIs (the bridge, made explicit).
    assert_eq!(party, q_hash("https://ns.webcivics.net/values/State"));
    assert_eq!(
        action,
        q_hash("https://ns.webcivics.net/action/SuppressForcedLabour")
    );

    // ── VALUES LOGIC over corpus-sourced terms: the State is OBLIGATED to suppress
    // forced labour. The norm's party + action are the u64s read out of the ingested
    // instrument, then run through the native deontic VM.
    let contract = q_hash("contract:instrument:ilo-c105");
    let norm = compile_norm_quin(party, OP_OBLIGATE, action, action, contract, 0, false);

    let mut out = [DeonticVerdict::default(); 4];
    let n = evaluate_deontic_contract(&[norm], 1_717_200_000u32, &mut out)
        .expect("deontic evaluation must succeed");
    assert_eq!(n, 1, "exactly one norm evaluated");
    assert_eq!(
        out[0].status,
        DeonticStatus::Active,
        "the State's obligation to suppress forced labour must be Active over the ingested corpus",
    );
}

/// Negative control: a norm whose party/action are NOT in the corpus is a different
/// identity, proving the join above is real selectivity, not a constant match.
#[test]
fn unrelated_term_does_not_join_the_corpus() {
    let corpus = ingest(CONCEPT_N3);
    let deontic = q_hash("https://ns.webcivics.net/concept/DTSFL-deontic");
    let borne_by = q_hash("https://ns.webcivics.net/values/borneBy");

    let party = object_of(&corpus, deontic, borne_by).expect("bearer present");
    // The bearer is values:State, NOT some unrelated corporation IRI.
    assert_ne!(
        party,
        q_hash("https://ns.webcivics.net/values/AcmeCorp"),
        "the ingested bearer must be the actual State term, not an arbitrary one",
    );
}
