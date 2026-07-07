//! Do the OTHER logic modalities evaluate over the generated CML corpus? (#7 follow-up)
//!
//! The corpus is now a deontic concept-graph in ONE hash space (#14). This proves the
//! other modalities COMPOSE over the same corpus terms — an agent's EPISTEMIC stance
//! about a corpus norm, and the norm's TEMPORAL in-force window — because everything
//! shares one identity space. (Honest scope: the generated layer carries norm TYPE +
//! bearer + metadata; the specific ACTION each norm obliges, and ECONOMIC cost data,
//! are content layers added at attestation/refinement, not in the heuristic scaffold.)

use qualia_core_db::modalities::epistemic::{
    evaluate_epistemic_frame, EpistemicStatus, EpistemicVerdict, CERTAINTY_BIT_SHIFT, OP_BELIEVES,
    OP_KNOWS,
};
use qualia_core_db::modalities::interval_reasoning::TemporalInterval;
use qualia_core_db::sparql_library::parsers::turtle_doc::parse_turtle_doc_into;
use qualia_core_db::sparql_library::quin_sink::QuinSink;
use qualia_core_db::{q_hash, NQuin};
use std::io;

const ICCPR_CONCEPTS: &str = include_str!(
    "../../../core-ontologies/concepts/international-covenant-civil-and-political-rights.n3"
);

#[derive(Default)]
struct VecSink(Vec<NQuin>);
impl QuinSink for VecSink {
    fn push(&mut self, q: NQuin) -> io::Result<()> {
        self.0.push(q);
        Ok(())
    }
}
fn ingest(doc: &str) -> Vec<NQuin> {
    let mut s = VecSink::default();
    parse_turtle_doc_into(doc.as_bytes(), 0, &mut s).expect("concepts ingest");
    s.0
}

#[test]
fn epistemic_and_temporal_modalities_compose_over_the_corpus() {
    let corpus = ingest(ICCPR_CONCEPTS);

    // A real generated norm from the corpus (ICCPR Art. 3 — equal rights of men & women).
    let norm = q_hash("https://ns.webcivics.net/concept/international-covenant-civil-and-political-rights-article-3-norm");
    let rdf_type = q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
    let obligation = q_hash("https://ns.webcivics.net/values/Obligation");
    // Deontic content is present: this corpus term IS typed an Obligation.
    assert!(
        corpus
            .iter()
            .any(|q| q.subject == norm && q.predicate == rdf_type && q.object == obligation),
        "the corpus norm must be a deontic Obligation"
    );

    // ── EPISTEMIC: agents take stances ABOUT the corpus norm (object = the norm hash). ──
    let state = q_hash("did:example:state-party");
    let world = q_hash("world:consensus");
    // KNOWS is factive -> Active; a low-certainty BELIEF is Uncertain (Axiom 2:
    // true-for-the-observer vs established-in-fact).
    let knows = NQuin {
        subject: state,
        predicate: OP_KNOWS as u64,
        object: norm,
        context: world,
        metadata: 0,
        parity: 0,
    };
    let weak_belief = NQuin {
        subject: state,
        predicate: (OP_BELIEVES as u64) | (40u64 << CERTAINTY_BIT_SHIFT), // certainty 40 (<128)
        object: norm,
        context: world,
        metadata: 0,
        parity: 0,
    };

    let blank = EpistemicVerdict {
        claim: NQuin::default(),
        status: EpistemicStatus::Skipped,
        certainty: 0,
    };
    let mut out = [blank; 4];
    let n = evaluate_epistemic_frame(&[knows], state, world, &mut out).expect("epistemic eval");
    assert_eq!(n, 1);
    assert_eq!(
        out[0].status,
        EpistemicStatus::Active,
        "KNOWS a corpus norm -> Active (factive)"
    );
    assert_eq!(
        out[0].claim.object, norm,
        "the epistemic claim is ABOUT the corpus norm"
    );

    let mut out2 = [blank; 4];
    evaluate_epistemic_frame(&[weak_belief], state, world, &mut out2).expect("epistemic eval");
    assert_eq!(
        out2[0].status,
        EpistemicStatus::Uncertain,
        "a low-certainty BELIEF about the same corpus norm is Uncertain, not fact"
    );

    // ── TEMPORAL: the norm's in-force window (ICCPR entered into force 1976-03-23). ──
    // Unix seconds: 1976-03-23 ~= 196387200; "now" 2024 ~= 1_700_000_000.
    let in_force = TemporalInterval::new(norm, 196_387_200, i64::MAX);
    assert!(
        in_force.contains(1_700_000_000),
        "the norm is in force in 2024"
    );
    assert!(
        !in_force.contains(0),
        "the norm was not in force at the unix epoch (1970)"
    );
}
