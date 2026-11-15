//! OWL support for QualiaDB.
//!
//! Two distinct concerns, one per submodule, plus the disjoint domain boundary
//! guard (`D = D_living ⊔ D_artifact`, Playbook v4 §6.2.2):
//!
//! - [`shacl_convert`] — *vocabulary lowering*: parse OWL (RadLex/DICOM healthcare
//!   ontologies in RDF/XML, Turtle, or N3) and emit `sh:NodeShape` graphs for the
//!   Webizen Sentinel, preserving the agency invariant (a `q42:Principal` may *have*
//!   `q42:Thing` possessions but is never itself a Thing).
//! - [`materialize`] — *reasoning*: OWL 2 RL forward-chaining entailment closure
//!   over NQuin-style triples (zero-heap, datalog-style fixpoint), with disjointness
//!   contradiction isolation and property-chain unrolling.
//! - **Disjoint Domain Partition (`D = D_living ⊔ D_artifact`):** Because W3C OWL 2
//!   Direct Semantics defines `owl:Thing` as the universal domain of all individuals
//!   admitted into an OWL interpretation, nodes in `D_living` (`self:HumanBeing`,
//!   `values:NaturalPerson`, `idf:NaturalAgent`, `idf:LivingFloraFauna`) are evaluated
//!   strictly under `N3Logic` / `living-SHACL` and rejected at the evaluator routing
//!   boundary if routed into the OWL 2 artifact reasoner.
//!
//! The public surface of both submodules is re-exported here so the historical
//! module path `crate::modalities::logic::owl::*` is preserved.

pub mod materialize;
pub mod shacl_convert;

pub use materialize::*;
pub use shacl_convert::*;

// ── Disjoint Domain Partition (D = D_living ⊔ D_artifact, §6.2.2) ────────────

pub const OWL_THING: u64 = crate::q_hash("http://www.w3.org/2002/07/owl#Thing");
pub const OWL_THING_CURIE: u64 = crate::q_hash("owl:Thing");

pub const SELF_HUMAN_BEING: u64 = crate::q_hash("https://ns.webcivics.net/selfhood#HumanBeing");
pub const SELF_HUMAN_BEING_CURIE: u64 = crate::q_hash("self:HumanBeing");
pub const VALUES_NATURAL_PERSON: u64 =
    crate::q_hash("https://ns.webcivics.net/values#NaturalPerson");
pub const VALUES_NATURAL_PERSON_CURIE: u64 = crate::q_hash("values:NaturalPerson");
pub const IDF_NATURAL_AGENT: u64 = crate::q_hash("urn:qualia:idf:NaturalAgent");
pub const IDF_NATURAL_AGENT_CURIE: u64 = crate::q_hash("idf:NaturalAgent");
pub const IDF_LIVING_FLORA_FAUNA: u64 = crate::q_hash("urn:qualia:idf:LivingFloraFauna");
pub const IDF_LIVING_FLORA_FAUNA_CURIE: u64 = crate::q_hash("idf:LivingFloraFauna");

/// Partition of the semantic domain `D = D_living ⊔ D_artifact`.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainPartition {
    /// Living beings (`self:HumanBeing`, `values:NaturalPerson`, `idf:NaturalAgent`,
    /// `idf:LivingFloraFauna`) — evaluated strictly under `N3Logic` + `living-SHACL`.
    Living = 1,
    /// Artifacts, instruments, and structural vocabularies — eligible for OWL 2 RL.
    Artifact = 2,
}

/// Routing boundary rejection when `D_living` nodes or `owl:Thing` subsumptions
/// are routed into the OWL 2 artifact reasoner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwlDomainBoundaryError {
    LivingEntityRoutedToOwlArtifactReasoner { subject: u64, living_class: u64 },
    LivingClassSubsumedUnderOwlThing { living_class: u64 },
    LivingSameAsMergeForbidden { left: u64, right: u64 },
}

/// True iff `class_hash` names a `D_living` class (`self:HumanBeing`,
/// `values:NaturalPerson`, `idf:NaturalAgent`, or `idf:LivingFloraFauna`).
#[inline]
pub const fn is_living_domain_class(class_hash: u64) -> bool {
    class_hash == SELF_HUMAN_BEING
        || class_hash == SELF_HUMAN_BEING_CURIE
        || class_hash == VALUES_NATURAL_PERSON
        || class_hash == VALUES_NATURAL_PERSON_CURIE
        || class_hash == IDF_NATURAL_AGENT
        || class_hash == IDF_NATURAL_AGENT_CURIE
        || class_hash == IDF_LIVING_FLORA_FAUNA
        || class_hash == IDF_LIVING_FLORA_FAUNA_CURIE
}

#[inline]
const fn is_owl_thing(class_hash: u64) -> bool {
    class_hash == OWL_THING || class_hash == OWL_THING_CURIE
}

/// Enforce `D = D_living ⊔ D_artifact` before running the OWL 2 RL artifact reasoner.
///
/// Rejects any triple graph that:
/// 1. Types an individual as a member of `D_living` inside an OWL 2 reasoning batch;
/// 2. Declares a `D_living` class as `rdfs:subClassOf` or `owl:equivalentClass` of `owl:Thing`;
/// 3. Applies `owl:sameAs` to a `D_living` class or individual.
pub fn enforce_disjoint_artifact_domain(
    triples: &[RdfTriple],
) -> Result<DomainPartition, OwlDomainBoundaryError> {
    let mut i = 0usize;
    while i < triples.len() {
        let t = triples[i];
        if (t.p == RDF_TYPE || t.p == crate::q_hash("rdf:type") || t.p == crate::q_hash("a"))
            && is_living_domain_class(t.o)
        {
            return Err(
                OwlDomainBoundaryError::LivingEntityRoutedToOwlArtifactReasoner {
                    subject: t.s,
                    living_class: t.o,
                },
            );
        }
        if (t.p == RDFS_SUBCLASS_OF
            || t.p == OWL_EQUIVALENT_CLASS
            || t.p == crate::q_hash("rdfs:subClassOf")
            || t.p == crate::q_hash("owl:equivalentClass"))
            && ((is_living_domain_class(t.s) && is_owl_thing(t.o))
                || (is_owl_thing(t.s) && is_living_domain_class(t.o)))
        {
            let living_class = if is_living_domain_class(t.s) {
                t.s
            } else {
                t.o
            };
            return Err(OwlDomainBoundaryError::LivingClassSubsumedUnderOwlThing { living_class });
        }
        if (t.p == OWL_SAME_AS || t.p == crate::q_hash("owl:sameAs"))
            && (is_living_domain_class(t.s) || is_living_domain_class(t.o))
        {
            return Err(OwlDomainBoundaryError::LivingSameAsMergeForbidden {
                left: t.s,
                right: t.o,
            });
        }
        i += 1;
    }
    Ok(DomainPartition::Artifact)
}

#[cfg(test)]
mod domain_partition_tests {
    use super::*;

    #[test]
    fn owl_reasoner_boundary_rejects_d_living_classes() {
        for living in [
            SELF_HUMAN_BEING,
            SELF_HUMAN_BEING_CURIE,
            VALUES_NATURAL_PERSON,
            VALUES_NATURAL_PERSON_CURIE,
            IDF_NATURAL_AGENT,
            IDF_NATURAL_AGENT_CURIE,
            IDF_LIVING_FLORA_FAUNA,
            IDF_LIVING_FLORA_FAUNA_CURIE,
        ] {
            let triples = [RdfTriple::new(crate::q_hash("urn:agent:alice"), RDF_TYPE, living)];
            assert_eq!(
                enforce_disjoint_artifact_domain(&triples),
                Err(
                    OwlDomainBoundaryError::LivingEntityRoutedToOwlArtifactReasoner {
                        subject: crate::q_hash("urn:agent:alice"),
                        living_class: living,
                    }
                )
            );
        }
    }

    #[test]
    fn owl_reasoner_boundary_rejects_living_subclass_of_owl_thing() {
        let triples = [RdfTriple::new(IDF_NATURAL_AGENT, RDFS_SUBCLASS_OF, OWL_THING)];
        assert_eq!(
            enforce_disjoint_artifact_domain(&triples),
            Err(OwlDomainBoundaryError::LivingClassSubsumedUnderOwlThing {
                living_class: IDF_NATURAL_AGENT,
            })
        );
    }

    #[test]
    fn owl_reasoner_boundary_admits_pure_artifact_triples() {
        let triples = [RdfTriple::new(
            crate::q_hash("urn:device:sensor1"),
            RDF_TYPE,
            crate::q_hash("urn:qualia:idf:MachineDevice"),
        )];
        assert_eq!(
            enforce_disjoint_artifact_domain(&triples),
            Ok(DomainPartition::Artifact)
        );
    }
}
