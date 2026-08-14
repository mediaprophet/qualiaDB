//! Define the BodyParts3D anatomy as an addressable **ontology** and serialise it to a `.q42` graph — the
//! semantic backbone the `.10d` mesh library is addressed by (Timothy 2026-07-12: "define an ontology …
//! store the info in a q42 file, with a library of d10 files produced via converting the files").
//!
//! Each FMA concept is keyed by its **canonical OBO IRI** (`http://purl.obolibrary.org/obo/FMA_<id>`), so
//! it joins *directly* to Monarch / UBERON / MONDO disease linked-data — which is what makes reasoning
//! over the **implications of comorbidities** a graph walk rather than a lookup table. Alongside the
//! canonical OBO triples it emits **house `q42:`/`geo:` aliases** for native reading (Timothy's "both"
//! choice). Per concept: `rdfs:label`, `rdfs:subClassOf` (**is-a**), `obo:BFO_0000050` (**part-of**),
//! `geo:bodySystem` (membership), `geo:compiledDigest` (the `.10d` it *hasMesh*), and a link to a dataset
//! node carrying the CC-BY-SA provenance + citation once.
//!
//! Pure (graph construction + `.q42` serialisation); unit-tested. The producer supplies the concepts +
//! their compiled digests + the parsed is-a / part-of tables.

use std::collections::HashMap;

use qualia_core_db::hypermedia::fnv60;
use qualia_core_db::q42_volume::UnifiedVolumeBuilder;
use qualia_core_db::{NQuin, QUINS_PER_BLOCK};

use super::bodyparts3d_resolver::{
    Bp3dHierarchy, BP3D_ATTRIBUTION, BP3D_CITATION, BP3D_DATA_DOI, BP3D_LICENCE, BP3D_SOURCE_URL,
};

/// A meshed anatomical concept to place in the ontology: its BodyParts3D id, the whole-file digest of its
/// compiled `.10d` (the geometry it `hasMesh`), and its resolved body-system membership(s).
#[derive(Debug, Clone)]
pub struct OntologyConcept {
    pub id: String,
    pub compiled_digest: u32,
    pub systems: Vec<String>,
}

// ── Vocabulary: canonical OBO / standards, plus house aliases ────────────────────────────────────
const OBO_FMA_PREFIX: &str = "http://purl.obolibrary.org/obo/FMA_";
const DATASET_IRI: &str = "urn:qualia:bodyparts3d:ontology";
// canonical predicates / classes
const P_RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const P_RDFS_LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
const P_RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const P_BFO_PART_OF: &str = "http://purl.obolibrary.org/obo/BFO_0000050"; // "part of"
const P_DCT_LICENSE: &str = "http://purl.org/dc/terms/license";
const P_DCT_CREATOR: &str = "http://purl.org/dc/terms/creator";
const P_DCT_SOURCE: &str = "http://purl.org/dc/terms/source";
const P_DCT_CITATION: &str = "http://purl.org/dc/terms/bibliographicCitation";
const P_DCT_IS_PART_OF: &str = "http://purl.org/dc/terms/isPartOf";
const C_ANATOMICAL_STRUCTURE: &str = "http://purl.obolibrary.org/obo/FMA_62955"; // "anatomical structure"
const C_DATASET: &str = "http://www.w3.org/ns/dcat#Dataset";
// house aliases (readable / native — Timothy's "both" choice)
const P_GEO_SYSTEM: &str = "geo:bodySystem";
const P_GEO_DIGEST: &str = "geo:compiledDigest";
const P_Q42_PART_OF: &str = "q42:partOf";
const P_Q42_IS_A: &str = "q42:isA";
const C_Q42_CONCEPT: &str = "q42:AnatomicalConcept";

/// The canonical IRI for a BodyParts3D id — an OBO FMA IRI for `FMA<n>`, else a namespaced URN.
fn concept_iri(id: &str) -> String {
    match id.strip_prefix("FMA") {
        Some(n) if !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()) => {
            format!("{OBO_FMA_PREFIX}{n}")
        }
        _ => format!("urn:bodyparts3d:{id}"),
    }
}

/// Small quin-graph builder: interns strings into a lexicon (60-bit FNV, the codebase's subject-identity
/// space) and pushes parity-valid quins in one named-graph context.
struct GraphBuilder {
    quins: Vec<NQuin>,
    lex: HashMap<u64, String>,
    context: u64,
}

impl GraphBuilder {
    fn intern(&mut self, s: &str) -> u64 {
        let h = fnv60(s.as_bytes());
        self.lex.entry(h).or_insert_with(|| s.to_string());
        h
    }
    /// Push `(subject, predicate, object)` with valid ECC parity.
    fn edge(&mut self, subject: u64, predicate: &str, object: u64) {
        let p = self.intern(predicate);
        let (ctx, md) = (self.context, 0u64);
        self.quins.push(NQuin {
            subject,
            predicate: p,
            object,
            context: ctx,
            metadata: md,
            parity: NQuin::calculate_parity(subject, p, object, ctx, md),
        });
    }
    /// An edge whose object is an IRI (interned).
    fn edge_iri(&mut self, subject: u64, predicate: &str, object_iri: &str) {
        let o = self.intern(object_iri);
        self.edge(subject, predicate, o);
    }
    /// An edge whose object is a string literal (interned).
    fn edge_lit(&mut self, subject: u64, predicate: &str, literal: &str) {
        let o = self.intern(literal);
        self.edge(subject, predicate, o);
    }
}

/// Emit the ontology graph (quins + object-lexicon) for a set of meshed concepts. The concepts' is-a
/// (`isa`) and part-of (`hier`) come from the BodyParts3D tables; each concept's OBO IRI is the canonical
/// identity, with `q42:`/`geo:` aliases emitted alongside.
pub fn emit_ontology(
    concepts: &[OntologyConcept],
    hier: &Bp3dHierarchy,
    isa: &HashMap<String, String>,
) -> (Vec<NQuin>, HashMap<u64, String>) {
    let mut g = GraphBuilder {
        quins: Vec::new(),
        lex: HashMap::new(),
        context: fnv60(DATASET_IRI.as_bytes()),
    };

    // The dataset node carries the CC-BY-SA provenance + citation ONCE (concepts link to it).
    let ds = g.intern(DATASET_IRI);
    g.edge_iri(ds, P_RDF_TYPE, C_DATASET);
    g.edge_lit(ds, P_RDFS_LABEL, "BodyParts3D anatomy ontology");
    g.edge_lit(ds, P_DCT_LICENSE, BP3D_LICENCE);
    g.edge_lit(ds, P_DCT_CREATOR, BP3D_ATTRIBUTION);
    g.edge_lit(ds, P_DCT_SOURCE, BP3D_SOURCE_URL);
    g.edge_lit(ds, P_DCT_CITATION, BP3D_CITATION);
    g.edge_lit(ds, P_DCT_CITATION, BP3D_DATA_DOI);

    for c in concepts {
        let s = {
            let iri = concept_iri(&c.id);
            g.intern(&iri)
        };
        // type — canonical anatomical-structure class + house alias.
        g.edge_iri(s, P_RDF_TYPE, C_ANATOMICAL_STRUCTURE);
        g.edge_iri(s, P_RDF_TYPE, C_Q42_CONCEPT);
        // label
        if let Some(name) = hier.name(&c.id) {
            g.edge_lit(s, P_RDFS_LABEL, name);
        }
        // is-a (canonical rdfs:subClassOf + house q42:isA), skipping self-loops.
        if let Some(parent) = isa.get(&c.id) {
            if parent != &c.id {
                let piri = concept_iri(parent);
                g.edge_iri(s, P_RDFS_SUBCLASSOF, &piri);
                g.edge_iri(s, P_Q42_IS_A, &piri);
            }
        }
        // part-of (canonical BFO_0000050 + house q42:partOf) — direct wholes.
        for whole in hier.wholes_of(&c.id) {
            let wiri = concept_iri(whole);
            g.edge_iri(s, P_BFO_PART_OF, &wiri);
            g.edge_iri(s, P_Q42_PART_OF, &wiri);
        }
        // body-system membership (house geo:bodySystem, literal system id).
        for sys in &c.systems {
            g.edge_lit(s, P_GEO_SYSTEM, sys);
        }
        // geometry — the concept hasMesh the .10d whose whole-file digest is this (numeric object).
        g.edge(s, P_GEO_DIGEST, c.compiled_digest as u64);
        // provenance link to the dataset node.
        g.edge(s, P_DCT_IS_PART_OF, ds);
    }

    (g.quins, g.lex)
}

/// Serialise the ontology of `concepts` into unified v3 `.q42` bytes (object-sorted blocks). Returns the
/// bytes and the number of quins in the graph.
pub fn ontology_q42_bytes(
    concepts: &[OntologyConcept],
    hier: &Bp3dHierarchy,
    isa: &HashMap<String, String>,
) -> (Vec<u8>, usize) {
    let (quins, lex) = emit_ontology(concepts, hier, isa);
    let count = quins.len();
    let mut sorted = quins;
    sorted.sort_by_key(|q| q.object);
    let mut builder = UnifiedVolumeBuilder::with_lex_map(&lex)
        .expect("ontology Q42 lexicon entries fit the current Q42LEX format");
    for (seq, chunk) in sorted.chunks(QUINS_PER_BLOCK).enumerate() {
        builder
            .push_block(seq as u64, chunk)
            .expect("ontology Q42 graph is object-sorted");
    }
    (builder.finish_to_bytes(), count)
}

#[cfg(test)]
mod tests {
    use super::super::bodyparts3d_resolver::Bp3dHierarchy;
    use super::*;

    const PARTS: &str = "\"id\"\ten\n\
        FMA72954\tmuscular system\n\
        FMA7158\trespiratory system\n\
        FMA13295\tdiaphragm\n";
    const PART_OF: &str = "\"id\"\tname\tpart id\tpart name\n\
        FMA72954\tmuscular system\tFMA13295\tdiaphragm\n\
        FMA7158\trespiratory system\tFMA13295\tdiaphragm\n";

    #[test]
    fn emits_an_addressable_ontology_q42_with_obo_iris_and_aliases() {
        let hier = Bp3dHierarchy::from_mapping(PARTS, PART_OF);
        let mut isa = HashMap::new();
        isa.insert("FMA13295".to_string(), "FMA9909".to_string()); // diaphragm is-a some muscle class
        let concepts = vec![OntologyConcept {
            id: "FMA13295".to_string(),
            compiled_digest: 0xDEAD_BEEF,
            systems: vec!["muscular".to_string(), "respiratory".to_string()],
        }];

        let (bytes, n) = ontology_q42_bytes(&concepts, &hier, &isa);
        assert!(n > 0);
        assert!(bytes.starts_with(&qualia_core_db::q42_volume::Q42_MAGIC));

        // Round-trip through a real unified volume.
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), &bytes).unwrap();
        let vol = qualia_core_db::q42_volume::Q42Volume::open(tmp.path()).unwrap();
        let quins = vol.read_all_quins().unwrap();
        assert_eq!(quins.len(), n, "every fact recoverable");
        let lex = vol.lex_view().unwrap();
        let objs: Vec<String> = quins
            .iter()
            .filter_map(|q| lex.lookup_hash(q.object).map(str::to_string))
            .collect();

        // Canonical OBO IRI IDENTITY (the concept is addressable + joins to disease data).
        let iri = "http://purl.obolibrary.org/obo/FMA_13295";
        assert_eq!(
            lex.lookup_hash(fnv60(iri.as_bytes())),
            Some(iri),
            "concept keyed by OBO IRI"
        );
        // is-a parent + part-of parents as OBO IRIs (objects).
        assert!(
            objs.iter()
                .any(|v| v == "http://purl.obolibrary.org/obo/FMA_9909"),
            "is-a parent IRI"
        );
        assert!(
            objs.iter()
                .any(|v| v == "http://purl.obolibrary.org/obo/FMA_72954"),
            "part-of muscular sys"
        );
        assert!(
            objs.iter()
                .any(|v| v == "http://purl.obolibrary.org/obo/FMA_7158"),
            "part-of respiratory sys"
        );
        // Label + BOTH system memberships + the CC-BY-SA licence (dataset node).
        assert!(objs.iter().any(|v| v == "diaphragm"), "label");
        assert!(
            objs.iter().any(|v| v == "muscular") && objs.iter().any(|v| v == "respiratory"),
            "systems"
        );
        assert!(
            objs.iter().any(|v| v == BP3D_LICENCE),
            "CC-BY-SA licence on the dataset node"
        );
        // The compiled `.10d` digest is addressable as a numeric object (geo:compiledDigest).
        assert!(
            quins.iter().any(|q| q.object == 0xDEAD_BEEF),
            "hasMesh digest present"
        );
    }
}
