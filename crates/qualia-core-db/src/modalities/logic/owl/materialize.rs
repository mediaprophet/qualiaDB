//! OWL 2 RL forward-chaining materialization over NQuin-style triples.
//!
//! This is the *reasoner* companion to [`super::shacl_convert`] (which only
//! *lowers* OWL vocabularies into SHACL shapes). Here we compute the OWL 2 RL
//! entailment closure of a triple set by datalog-style fixpoint iteration — the
//! standard PTIME approach to OWL 2 RL — using only fixed caller-supplied buffers
//! (zero heap; no `Vec`/`Box`). Three audit capabilities live here:
//!
//! 1. **OWL 2 RL partial materialization** — the property/class-axiom rule subset
//!    that needs no RDF-list decoding: `cax-sco`, `prp-spo1`, `prp-dom`, `prp-rng`,
//!    `prp-symp`, `prp-trp`, `prp-inv`, `prp-fp`, `prp-ifp`, `eq-sym`, `eq-trans`,
//!    `scm-sco`, `scm-spo`, plus equivalence expansion (`scm-eqc`/`scm-eqp`),
//!    iterated to a fixpoint. This is the polynomial-time core of OWL 2 RL.
//! 2. **Disjointness contradiction isolation** (`cax-dw`) — a violating individual
//!    is recorded into a quarantine buffer and the closure *keeps going*. An
//!    inconsistency does NOT explode the graph into "everything entailed"; the rest
//!    of the Information-Banking ecosystem stays usable off-grid.
//! 3. **Property-chain axiom unrolling** — `p ⊑ p1 ∘ p2` is composed by the sparse
//!    boolean-relation product (the join form of a boolean matrix multiply),
//!    supplied as explicit [`ChainAxiom`]s (the internal form an N3/OWL parser
//!    produces from `owl:propertyChainAxiom (p1 p2)`).
//!
//! ## Complexity
//! Each fixpoint pass scans a stable prefix of the bounded working set and inserts
//! deduplicated derivations into the tail; iteration stops at a fixpoint or a caller
//! `max_iters` cap. With a working set bounded by the caller's buffer this is
//! polynomial and terminates — suitable for constrained hardware.

use crate::NQuin;

// ── OWL / RDFS vocabulary (FNV-1a hashed at compile time via `q_hash`) ──────────

pub const RDF_TYPE: u64 = crate::q_hash("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
pub const RDFS_SUBCLASS_OF: u64 = crate::q_hash("http://www.w3.org/2000/01/rdf-schema#subClassOf");
pub const RDFS_SUBPROPERTY_OF: u64 =
    crate::q_hash("http://www.w3.org/2000/01/rdf-schema#subPropertyOf");
pub const RDFS_DOMAIN: u64 = crate::q_hash("http://www.w3.org/2000/01/rdf-schema#domain");
pub const RDFS_RANGE: u64 = crate::q_hash("http://www.w3.org/2000/01/rdf-schema#range");
pub const OWL_SAME_AS: u64 = crate::q_hash("http://www.w3.org/2002/07/owl#sameAs");
pub const OWL_INVERSE_OF: u64 = crate::q_hash("http://www.w3.org/2002/07/owl#inverseOf");
pub const OWL_EQUIVALENT_CLASS: u64 =
    crate::q_hash("http://www.w3.org/2002/07/owl#equivalentClass");
pub const OWL_EQUIVALENT_PROPERTY: u64 =
    crate::q_hash("http://www.w3.org/2002/07/owl#equivalentProperty");
pub const OWL_DISJOINT_WITH: u64 = crate::q_hash("http://www.w3.org/2002/07/owl#disjointWith");
pub const OWL_SYMMETRIC_PROPERTY: u64 =
    crate::q_hash("http://www.w3.org/2002/07/owl#SymmetricProperty");
pub const OWL_TRANSITIVE_PROPERTY: u64 =
    crate::q_hash("http://www.w3.org/2002/07/owl#TransitiveProperty");
pub const OWL_FUNCTIONAL_PROPERTY: u64 =
    crate::q_hash("http://www.w3.org/2002/07/owl#FunctionalProperty");
pub const OWL_INVERSE_FUNCTIONAL_PROPERTY: u64 =
    crate::q_hash("http://www.w3.org/2002/07/owl#InverseFunctionalProperty");

// ── Types ───────────────────────────────────────────────────────────────────

/// A reasoning triple — the `(subject, predicate, object)` projection of an NQuin.
/// `Copy` and 24 bytes, so working sets live entirely in caller stack/arena buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RdfTriple {
    pub s: u64,
    pub p: u64,
    pub o: u64,
}

impl RdfTriple {
    pub const fn new(s: u64, p: u64, o: u64) -> Self {
        Self { s, p, o }
    }

    /// Project an `NQuin` to its `(subject, predicate, object)` reasoning triple.
    pub const fn from_nquin(q: &NQuin) -> Self {
        Self {
            s: q.subject,
            p: q.predicate,
            o: q.object,
        }
    }
}

/// A 2-step property chain axiom `composed ⊑ first ∘ second`. An OWL/N3 parser
/// lowers `composed owl:propertyChainAxiom (first second)` into this form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChainAxiom {
    pub composed: u64,
    pub first: u64,
    pub second: u64,
}

/// A recorded `cax-dw` disjointness contradiction: `individual` was inferred to be
/// a member of two `owl:disjointWith` classes. Reported, not fatal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisjointnessViolation {
    pub individual: u64,
    pub class_a: u64,
    pub class_b: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterializeError {
    /// The working-set buffer filled before the closure saturated. Grow `triples`.
    WorkingSetFull,
    /// The contradiction buffer filled. Grow `contradictions_out`.
    ContradictionBufferFull,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaterializeSummary {
    /// Total triples after materialization (inputs + inferred).
    pub triple_count: usize,
    /// Newly inferred triples (`triple_count - initial_len`).
    pub inferred_count: usize,
    /// Disjointness contradictions quarantined.
    pub contradiction_count: usize,
    /// Fixpoint passes performed.
    pub iterations: u32,
    /// `true` if a fixpoint was reached (no new triples), `false` if `max_iters` hit.
    pub saturated: bool,
}

// ── Working-set helpers ───────────────────────────────────────────────────────

#[inline]
fn contains(triples: &[RdfTriple], len: usize, t: RdfTriple) -> bool {
    triples[..len].iter().any(|&x| x == t)
}

/// Insert `t` if absent. Returns `Ok(true)` if newly inserted, `Ok(false)` if a
/// duplicate, `Err(WorkingSetFull)` if the buffer is full.
#[inline]
fn try_push(
    triples: &mut [RdfTriple],
    len: &mut usize,
    t: RdfTriple,
) -> Result<bool, MaterializeError> {
    if contains(triples, *len, t) {
        return Ok(false);
    }
    if *len >= triples.len() {
        return Err(MaterializeError::WorkingSetFull);
    }
    triples[*len] = t;
    *len += 1;
    Ok(true)
}

// ── Materialization ───────────────────────────────────────────────────────────

/// Compute the OWL 2 RL entailment closure of `triples[..initial_len]` in place.
///
/// `triples` must be sized to hold inputs *plus* derivations; the function fills
/// the tail and returns the new total length in the summary. `chains` supplies
/// property-chain axioms. Disjointness contradictions are written to
/// `contradictions_out`. The closure isolates contradictions (records and
/// continues) rather than halting.
pub fn materialize_owl_rl(
    triples: &mut [RdfTriple],
    initial_len: usize,
    chains: &[ChainAxiom],
    max_iters: u32,
    contradictions_out: &mut [DisjointnessViolation],
) -> Result<MaterializeSummary, MaterializeError> {
    let mut len = initial_len.min(triples.len());
    let mut iterations = 0u32;
    let mut saturated = false;

    while iterations < max_iters {
        iterations += 1;
        let mut changed = false;
        let n = len; // stable antecedent prefix for this pass

        for i in 0..n {
            let t = triples[i];

            // scm-eqc / scm-eqp: equivalence expands to both-way sub-axioms.
            if t.p == OWL_EQUIVALENT_CLASS {
                changed |= try_push(
                    triples,
                    &mut len,
                    RdfTriple::new(t.s, RDFS_SUBCLASS_OF, t.o),
                )?;
                changed |= try_push(
                    triples,
                    &mut len,
                    RdfTriple::new(t.o, RDFS_SUBCLASS_OF, t.s),
                )?;
            } else if t.p == OWL_EQUIVALENT_PROPERTY {
                changed |= try_push(
                    triples,
                    &mut len,
                    RdfTriple::new(t.s, RDFS_SUBPROPERTY_OF, t.o),
                )?;
                changed |= try_push(
                    triples,
                    &mut len,
                    RdfTriple::new(t.o, RDFS_SUBPROPERTY_OF, t.s),
                )?;
            }
            // eq-sym: sameAs is symmetric.
            else if t.p == OWL_SAME_AS && t.s != t.o {
                changed |= try_push(triples, &mut len, RdfTriple::new(t.o, OWL_SAME_AS, t.s))?;
            }
            // prp-inv: x p1 y ⟹ y p2 x for each (p1 inverseOf p2). Walk inverse decls.
            else if t.p == OWL_INVERSE_OF {
                let (p1, p2) = (t.s, t.o);
                for j in 0..n {
                    let u = triples[j];
                    if u.p == p1 {
                        changed |= try_push(triples, &mut len, RdfTriple::new(u.o, p2, u.s))?;
                    }
                    if u.p == p2 {
                        changed |= try_push(triples, &mut len, RdfTriple::new(u.o, p1, u.s))?;
                    }
                }
            }
        }

        // scm-sco / scm-spo: subClassOf / subPropertyOf transitivity.
        for i in 0..n {
            let a = triples[i];
            if a.p != RDFS_SUBCLASS_OF && a.p != RDFS_SUBPROPERTY_OF {
                continue;
            }
            for j in 0..n {
                let b = triples[j];
                if b.p == a.p && b.s == a.o && a.s != b.o {
                    changed |= try_push(triples, &mut len, RdfTriple::new(a.s, a.p, b.o))?;
                }
            }
        }

        // cax-sco: x type c1 ∧ c1 subClassOf c2 ⟹ x type c2.
        // prp-spo1: x p1 y ∧ p1 subPropertyOf p2 ⟹ x p2 y.
        for i in 0..n {
            let inst = triples[i];
            for j in 0..n {
                let ax = triples[j];
                if ax.p == RDFS_SUBCLASS_OF
                    && inst.p == RDF_TYPE
                    && inst.o == ax.s
                    && inst.o != ax.o
                {
                    changed |= try_push(triples, &mut len, RdfTriple::new(inst.s, RDF_TYPE, ax.o))?;
                } else if ax.p == RDFS_SUBPROPERTY_OF && inst.p == ax.s && ax.s != ax.o {
                    changed |= try_push(triples, &mut len, RdfTriple::new(inst.s, ax.o, inst.o))?;
                }
            }
        }

        // prp-dom / prp-rng: property domain/range typing.
        for i in 0..n {
            let ax = triples[i];
            if ax.p != RDFS_DOMAIN && ax.p != RDFS_RANGE {
                continue;
            }
            for j in 0..n {
                let inst = triples[j];
                if inst.p == ax.s {
                    let subj = if ax.p == RDFS_DOMAIN { inst.s } else { inst.o };
                    changed |= try_push(triples, &mut len, RdfTriple::new(subj, RDF_TYPE, ax.o))?;
                }
            }
        }

        // prp-symp: p type SymmetricProperty ∧ x p y ⟹ y p x.
        // prp-trp:  p type TransitiveProperty ∧ x p y ∧ y p z ⟹ x p z.
        for i in 0..n {
            let inst = triples[i];
            if inst.p == RDF_TYPE {
                continue;
            }
            let is_symmetric = contains(
                triples,
                len,
                RdfTriple::new(inst.p, RDF_TYPE, OWL_SYMMETRIC_PROPERTY),
            );
            if is_symmetric && inst.s != inst.o {
                changed |= try_push(triples, &mut len, RdfTriple::new(inst.o, inst.p, inst.s))?;
            }
            let is_transitive = contains(
                triples,
                len,
                RdfTriple::new(inst.p, RDF_TYPE, OWL_TRANSITIVE_PROPERTY),
            );
            if is_transitive {
                for j in 0..n {
                    let next = triples[j];
                    if next.p == inst.p && next.s == inst.o && inst.s != next.o {
                        changed |=
                            try_push(triples, &mut len, RdfTriple::new(inst.s, inst.p, next.o))?;
                    }
                }
            }
        }

        // prp-fp:  p type FunctionalProperty ∧ x p y1 ∧ x p y2 ⟹ y1 sameAs y2.
        // prp-ifp: p type InverseFunctionalProperty ∧ x1 p y ∧ x2 p y ⟹ x1 sameAs x2.
        for i in 0..n {
            let a = triples[i];
            if a.p == RDF_TYPE {
                continue;
            }
            let functional = contains(
                triples,
                len,
                RdfTriple::new(a.p, RDF_TYPE, OWL_FUNCTIONAL_PROPERTY),
            );
            let inv_functional = contains(
                triples,
                len,
                RdfTriple::new(a.p, RDF_TYPE, OWL_INVERSE_FUNCTIONAL_PROPERTY),
            );
            if !functional && !inv_functional {
                continue;
            }
            for j in 0..n {
                let b = triples[j];
                if b.p != a.p {
                    continue;
                }
                if functional && b.s == a.s && a.o != b.o {
                    changed |= try_push(triples, &mut len, RdfTriple::new(a.o, OWL_SAME_AS, b.o))?;
                }
                if inv_functional && b.o == a.o && a.s != b.s {
                    changed |= try_push(triples, &mut len, RdfTriple::new(a.s, OWL_SAME_AS, b.s))?;
                }
            }
        }

        // eq-trans: sameAs transitivity.
        for i in 0..n {
            let a = triples[i];
            if a.p != OWL_SAME_AS {
                continue;
            }
            for j in 0..n {
                let b = triples[j];
                if b.p == OWL_SAME_AS && b.s == a.o && a.s != b.o {
                    changed |= try_push(triples, &mut len, RdfTriple::new(a.s, OWL_SAME_AS, b.o))?;
                }
            }
        }

        // Property-chain unrolling: composed ⊑ first ∘ second (sparse boolean product).
        for chain in chains {
            for i in 0..n {
                let lhs = triples[i];
                if lhs.p != chain.first {
                    continue;
                }
                for j in 0..n {
                    let rhs = triples[j];
                    if rhs.p == chain.second && rhs.s == lhs.o {
                        changed |= try_push(
                            triples,
                            &mut len,
                            RdfTriple::new(lhs.s, chain.composed, rhs.o),
                        )?;
                    }
                }
            }
        }

        if !changed {
            saturated = true;
            break;
        }
    }

    // ── cax-dw: disjointness contradiction isolation (post-closure) ──────────
    let mut contradiction_count = 0usize;
    for i in 0..len {
        let dj = triples[i];
        if dj.p != OWL_DISJOINT_WITH {
            continue;
        }
        let (c1, c2) = (dj.s, dj.o);
        for j in 0..len {
            let tj = triples[j];
            if tj.p != RDF_TYPE || tj.o != c1 {
                continue;
            }
            let x = tj.s;
            if !contains(triples, len, RdfTriple::new(x, RDF_TYPE, c2)) {
                continue;
            }
            let dup = contradictions_out[..contradiction_count].iter().any(|e| {
                e.individual == x
                    && ((e.class_a == c1 && e.class_b == c2)
                        || (e.class_a == c2 && e.class_b == c1))
            });
            if dup {
                continue;
            }
            if contradiction_count >= contradictions_out.len() {
                return Err(MaterializeError::ContradictionBufferFull);
            }
            contradictions_out[contradiction_count] = DisjointnessViolation {
                individual: x,
                class_a: c1,
                class_b: c2,
            };
            contradiction_count += 1;
        }
    }

    Ok(MaterializeSummary {
        triple_count: len,
        inferred_count: len - initial_len.min(triples.len()),
        contradiction_count,
        iterations,
        saturated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // Stable test IRIs.
    fn h(s: &str) -> u64 {
        crate::q_hash(s)
    }

    #[test]
    fn subclass_transitivity_and_type_propagation() {
        let (alice, student, person, agent) = (h("alice"), h("Student"), h("Person"), h("Agent"));
        let mut triples = [RdfTriple::new(0, 0, 0); 64];
        triples[0] = RdfTriple::new(alice, RDF_TYPE, student);
        triples[1] = RdfTriple::new(student, RDFS_SUBCLASS_OF, person);
        triples[2] = RdfTriple::new(person, RDFS_SUBCLASS_OF, agent);
        let mut contra = [DisjointnessViolation {
            individual: 0,
            class_a: 0,
            class_b: 0,
        }; 4];

        let s = materialize_owl_rl(&mut triples, 3, &[], 16, &mut contra).unwrap();
        assert!(s.saturated);
        let out = &triples[..s.triple_count];
        // scm-sco: Student ⊑ Agent
        assert!(out.contains(&RdfTriple::new(student, RDFS_SUBCLASS_OF, agent)));
        // cax-sco: alice is a Person AND an Agent
        assert!(out.contains(&RdfTriple::new(alice, RDF_TYPE, person)));
        assert!(out.contains(&RdfTriple::new(alice, RDF_TYPE, agent)));
        assert_eq!(s.contradiction_count, 0);
    }

    #[test]
    fn domain_and_range_typing() {
        let (knows, person, x, y) = (h("knows"), h("Person"), h("x"), h("y"));
        let mut triples = [RdfTriple::new(0, 0, 0); 32];
        triples[0] = RdfTriple::new(knows, RDFS_DOMAIN, person);
        triples[1] = RdfTriple::new(knows, RDFS_RANGE, person);
        triples[2] = RdfTriple::new(x, knows, y);
        let mut contra = [DisjointnessViolation {
            individual: 0,
            class_a: 0,
            class_b: 0,
        }; 2];

        let s = materialize_owl_rl(&mut triples, 3, &[], 16, &mut contra).unwrap();
        let out = &triples[..s.triple_count];
        assert!(out.contains(&RdfTriple::new(x, RDF_TYPE, person))); // prp-dom
        assert!(out.contains(&RdfTriple::new(y, RDF_TYPE, person))); // prp-rng
    }

    #[test]
    fn transitive_and_inverse_properties() {
        let (anc, has_child, a, b, c) = (h("ancestorOf"), h("hasChild"), h("a"), h("b"), h("c"));
        let mut triples = [RdfTriple::new(0, 0, 0); 32];
        triples[0] = RdfTriple::new(anc, RDF_TYPE, OWL_TRANSITIVE_PROPERTY);
        triples[1] = RdfTriple::new(a, anc, b);
        triples[2] = RdfTriple::new(b, anc, c);
        triples[3] = RdfTriple::new(anc, OWL_INVERSE_OF, has_child);
        let mut contra = [DisjointnessViolation {
            individual: 0,
            class_a: 0,
            class_b: 0,
        }; 2];

        let s = materialize_owl_rl(&mut triples, 4, &[], 16, &mut contra).unwrap();
        let out = &triples[..s.triple_count];
        assert!(out.contains(&RdfTriple::new(a, anc, c))); // prp-trp
        assert!(out.contains(&RdfTriple::new(b, has_child, a))); // prp-inv
        assert!(out.contains(&RdfTriple::new(c, has_child, b)));
    }

    #[test]
    fn property_chain_unrolling() {
        // uncleOf ⊑ parentOf ∘ brotherOf
        let (parent, brother, uncle, x, p, u) = (
            h("parentOf"),
            h("brotherOf"),
            h("uncleOf"),
            h("x"),
            h("p"),
            h("u"),
        );
        let mut triples = [RdfTriple::new(0, 0, 0); 16];
        triples[0] = RdfTriple::new(x, parent, p);
        triples[1] = RdfTriple::new(p, brother, u);
        let chains = [ChainAxiom {
            composed: uncle,
            first: parent,
            second: brother,
        }];
        let mut contra = [DisjointnessViolation {
            individual: 0,
            class_a: 0,
            class_b: 0,
        }; 2];

        let s = materialize_owl_rl(&mut triples, 2, &chains, 16, &mut contra).unwrap();
        assert!(triples[..s.triple_count].contains(&RdfTriple::new(x, uncle, u)));
    }

    #[test]
    fn disjointness_isolation_does_not_halt() {
        // Cat disjointWith Dog; fido inferred to be both (via subclass) — quarantine,
        // but unrelated inference (rex is a Pet) must still complete.
        let (cat, dog, animal, fido, rex, pet, breed) = (
            h("Cat"),
            h("Dog"),
            h("Animal"),
            h("fido"),
            h("rex"),
            h("Pet"),
            h("Breed"),
        );
        let mut triples = [RdfTriple::new(0, 0, 0); 64];
        triples[0] = RdfTriple::new(cat, OWL_DISJOINT_WITH, dog);
        triples[1] = RdfTriple::new(fido, RDF_TYPE, cat);
        triples[2] = RdfTriple::new(fido, RDF_TYPE, dog);
        triples[3] = RdfTriple::new(breed, RDFS_SUBCLASS_OF, pet);
        triples[4] = RdfTriple::new(rex, RDF_TYPE, breed);
        let _ = animal;
        let mut contra = [DisjointnessViolation {
            individual: 0,
            class_a: 0,
            class_b: 0,
        }; 4];

        let s = materialize_owl_rl(&mut triples, 5, &[], 16, &mut contra).unwrap();
        // The contradiction was isolated, not fatal.
        assert_eq!(s.contradiction_count, 1);
        assert_eq!(contra[0].individual, fido);
        // Closure still completed: rex is a Pet (cax-sco) despite the inconsistency.
        assert!(triples[..s.triple_count].contains(&RdfTriple::new(rex, RDF_TYPE, pet)));
    }

    #[test]
    fn functional_property_implies_sameas() {
        // hasSSN functional: x hasSSN a, x hasSSN b ⟹ a sameAs b (+ eq-sym).
        let (has_ssn, x, a, b) = (h("hasSSN"), h("x"), h("idA"), h("idB"));
        let mut triples = [RdfTriple::new(0, 0, 0); 16];
        triples[0] = RdfTriple::new(has_ssn, RDF_TYPE, OWL_FUNCTIONAL_PROPERTY);
        triples[1] = RdfTriple::new(x, has_ssn, a);
        triples[2] = RdfTriple::new(x, has_ssn, b);
        let mut contra = [DisjointnessViolation {
            individual: 0,
            class_a: 0,
            class_b: 0,
        }; 2];

        let s = materialize_owl_rl(&mut triples, 3, &[], 16, &mut contra).unwrap();
        let out = &triples[..s.triple_count];
        assert!(
            out.contains(&RdfTriple::new(a, OWL_SAME_AS, b))
                || out.contains(&RdfTriple::new(b, OWL_SAME_AS, a))
        );
        // eq-sym closed it both ways.
        assert!(out.contains(&RdfTriple::new(a, OWL_SAME_AS, b)));
        assert!(out.contains(&RdfTriple::new(b, OWL_SAME_AS, a)));
    }

    #[test]
    fn working_set_full_is_reported() {
        let (a, sub) = (h("a"), RDFS_SUBCLASS_OF);
        // A long subclass chain in a deliberately tiny buffer overflows on derivation.
        let mut triples = [RdfTriple::new(0, 0, 0); 6];
        triples[0] = RdfTriple::new(a, RDF_TYPE, h("C0"));
        triples[1] = RdfTriple::new(h("C0"), sub, h("C1"));
        triples[2] = RdfTriple::new(h("C1"), sub, h("C2"));
        triples[3] = RdfTriple::new(h("C2"), sub, h("C3"));
        let mut contra = [DisjointnessViolation {
            individual: 0,
            class_a: 0,
            class_b: 0,
        }; 2];
        let r = materialize_owl_rl(&mut triples, 4, &[], 16, &mut contra);
        assert_eq!(r, Err(MaterializeError::WorkingSetFull));
    }
}
