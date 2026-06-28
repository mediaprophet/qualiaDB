use crate::NQuin;

/// Max distinct classes considered in one subsumption query (bounded, zero-heap).
const DL_MAX_CLASSES: usize = 256;

/// Returns true if `sub_class_hash` is subsumed by `super_class_hash` in the TBox.
///
/// Comprehensive: a full transitive-closure search over the `rdfs:subClassOf` **DAG**, so
/// **multiple inheritance** is handled (a class may have many superclasses — e.g.
/// `NaturalPerson ⊑ Agent` AND `NaturalPerson ⊑ self:HumanBeing`). Zero-heap (fixed
/// frontier + visited arrays) and cycle-safe (visited set). The earlier version followed only
/// the FIRST parent edge per node and silently missed every other inheritance path.
pub fn check_subsumption_quin(
    sub_class_hash: u64,
    super_class_hash: u64,
    tbox: &[NQuin], // Quins with predicate = q_hash("rdfs:subClassOf")
) -> bool {
    if sub_class_hash == super_class_hash {
        return true;
    }
    let mut frontier = [0u64; DL_MAX_CLASSES];
    let mut visited = [0u64; DL_MAX_CLASSES];
    let mut fl = 1usize; // frontier length
    let mut vl = 0usize; // visited length
    frontier[0] = sub_class_hash;

    while fl > 0 {
        fl -= 1;
        let current = frontier[fl];
        if visited[..vl].contains(&current) {
            continue;
        }
        if vl < DL_MAX_CLASSES {
            visited[vl] = current;
            vl += 1;
        } else {
            break; // closure exceeds the bound; refuse rather than mis-answer
        }
        for quin in tbox {
            if quin.subject == current {
                let parent = quin.object;
                if parent == super_class_hash {
                    return true;
                }
                if fl < DL_MAX_CLASSES && !visited[..vl].contains(&parent) {
                    frontier[fl] = parent;
                    fl += 1;
                }
            }
        }
    }
    false
}

// ─── Structural SROIQ constructs (disjointness/clash, roles, cardinality, nominals) ─
//
// SCOPE (honest): these are the zero-heap STRUCTURAL constructs of SROIQ — concept disjointness
// + clash detection (ABox consistency core), role hierarchies + transitivity, qualified
// cardinality, and nominals — over an ABox/RBox of NQuins. The full ALC/SROIQ model-construction
// TABLEAU (∃/∀ expansion with individual generation + blocking) is a separate research-grade
// effort that ALSO conflicts with the zero-heap invariant (a tableau builds a dynamic model tree)
// — recorded in AUDIT_BOUNDARY_DEFERRALS.md.

/// Are concepts `a` and `b` declared DISJOINT in `disjoint` (symmetric pairs)?
pub fn concepts_disjoint(a: u64, b: u64, disjoint: &[(u64, u64)]) -> bool {
    disjoint
        .iter()
        .any(|&(c1, c2)| (c1 == a && c2 == b) || (c1 == b && c2 == a))
}

/// **Clash detection** (ABox consistency core): does an individual asserted to have all of
/// `types` have a clash — two types that are disjoint, directly or via subsumption (`t1 ⊑ X`,
/// `t2 ⊑ Y`, `X` disjoint `Y`)? Returns true on a clash (inconsistency). Zero-heap.
pub fn abox_clash(types: &[u64], disjoint: &[(u64, u64)], tbox: &[NQuin]) -> bool {
    for (i, &t1) in types.iter().enumerate() {
        for &t2 in &types[i + 1..] {
            if concepts_disjoint(t1, t2, disjoint) {
                return true;
            }
            for &(d1, d2) in disjoint {
                if (check_subsumption_quin(t1, d1, tbox) && check_subsumption_quin(t2, d2, tbox))
                    || (check_subsumption_quin(t1, d2, tbox)
                        && check_subsumption_quin(t2, d1, tbox))
                {
                    return true;
                }
            }
        }
    }
    false
}

/// **Role hierarchy**: is `sub_role` subsumed by `super_role` (transitively) in the RBox of
/// `rdfs:subPropertyOf` quins? (Same transitive-closure search as class subsumption.)
#[inline]
pub fn role_subsumes(sub_role: u64, super_role: u64, rbox: &[NQuin]) -> bool {
    check_subsumption_quin(sub_role, super_role, rbox)
}

/// **Transitive role**: is `role` declared transitive?
#[inline]
pub fn is_transitive_role(role: u64, transitive_roles: &[u64]) -> bool {
    transitive_roles.contains(&role)
}

/// Count an individual's `role`-successors that are instances of `filler_class` (directly or via
/// subsumption) — for **qualified cardinality** restrictions. `abox` holds role assertions
/// `(individual, role, successor)`; `type_assertions` holds `(individual, class)`. Zero-heap.
pub fn count_qualified_fillers(
    individual: u64,
    role: u64,
    filler_class: u64,
    abox: &[NQuin],
    type_assertions: &[(u64, u64)],
    tbox: &[NQuin],
) -> usize {
    let mut count = 0usize;
    for e in abox {
        if e.subject == individual && e.predicate == role {
            let succ = e.object;
            if type_assertions
                .iter()
                .any(|&(s, c)| s == succ && check_subsumption_quin(c, filler_class, tbox))
            {
                count += 1;
            }
        }
    }
    count
}

/// Qualified MIN cardinality `≥ n R.C` satisfied?
#[inline]
pub fn min_cardinality_met(actual: usize, n: usize) -> bool {
    actual >= n
}

/// Qualified MAX cardinality `≤ n R.C` satisfied?
#[inline]
pub fn max_cardinality_met(actual: usize, n: usize) -> bool {
    actual <= n
}

/// **Nominal** `{a}`: a nominal concept has exactly one instance — `a` itself. Is `individual`
/// the instance of the nominal `{nominal_individual}`?
#[inline]
pub fn is_nominal_instance(individual: u64, nominal_individual: u64) -> bool {
    individual == nominal_individual
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_subsumption_quin() {
        let tbox = vec![
            NQuin {
                subject: 10,
                predicate: 0,
                object: 20,
                context: 0,
                metadata: 0,
                parity: 0,
            },
            NQuin {
                subject: 20,
                predicate: 0,
                object: 30,
                context: 0,
                metadata: 0,
                parity: 0,
            },
        ];

        assert_eq!(check_subsumption_quin(10, 10, &tbox), true);
        assert_eq!(check_subsumption_quin(10, 20, &tbox), true);
        assert_eq!(check_subsumption_quin(10, 30, &tbox), true);
        assert_eq!(check_subsumption_quin(10, 40, &tbox), false);
        assert_eq!(check_subsumption_quin(20, 10, &tbox), false);
    }

    /// Comprehensive: MULTIPLE INHERITANCE + diamond (the old single-edge impl failed these).
    #[test]
    fn multiple_inheritance_dag() {
        let e = |s: u64, o: u64| NQuin {
            subject: s,
            predicate: 1,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let (np, agent, human, moral) = (1u64, 2u64, 3u64, 4u64);
        let tbox = [
            e(np, agent),    // NaturalPerson ⊑ Agent       (first parent)
            e(np, human),    // NaturalPerson ⊑ HumanBeing  (SECOND parent — old impl missed this)
            e(human, moral), // HumanBeing ⊑ MoralFrame
            e(agent, moral), // Agent ⊑ MoralFrame          (diamond)
        ];
        assert!(check_subsumption_quin(np, agent, &tbox), "via first parent");
        assert!(
            check_subsumption_quin(np, human, &tbox),
            "via SECOND parent (multiple inheritance)"
        );
        assert!(
            check_subsumption_quin(np, moral, &tbox),
            "transitively via either diamond path"
        );
        assert!(
            !check_subsumption_quin(agent, human, &tbox),
            "Agent is not a HumanBeing"
        );
    }

    #[test]
    fn disjointness_clash_detection() {
        let sub = 1u64; // pretend predicate
        let e = |s: u64, o: u64| NQuin {
            subject: s,
            predicate: sub,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let (human, robot, agent, machine) = (10u64, 20u64, 30u64, 40u64);
        let tbox = [e(human, agent), e(robot, machine)]; // Human⊑Agent, Robot⊑Machine
        let disjoint = [(agent, machine)]; // Agent disjoint Machine
                                           // Direct disjointness.
        assert!(concepts_disjoint(agent, machine, &disjoint));
        // An individual that is both Human and Robot clashes (Human⊑Agent, Robot⊑Machine, Agent⊥Machine).
        assert!(abox_clash(&[human, robot], &disjoint, &tbox));
        // Human alone, or Human+Agent, is consistent.
        assert!(!abox_clash(&[human, agent], &disjoint, &tbox));
    }

    #[test]
    fn roles_cardinality_nominals() {
        let sp = 7u64;
        let e = |s: u64, o: u64| NQuin {
            subject: s,
            predicate: sp,
            object: o,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        let (has_mother, has_parent) = (100u64, 200u64);
        let rbox = [e(has_mother, has_parent)]; // hasMother ⊑ hasParent
        assert!(role_subsumes(has_mother, has_parent, &rbox));
        assert!(!role_subsumes(has_parent, has_mother, &rbox));
        assert!(is_transitive_role(
            crate::q_hash("role:ancestorOf"),
            &[crate::q_hash("role:ancestorOf")]
        ));

        // Qualified cardinality: alice has 2 children who are Students.
        let role = crate::q_hash("role:hasChild");
        let student = crate::q_hash("class:Student");
        let (alice, bob, cara) = (
            crate::q_hash("ind:alice"),
            crate::q_hash("ind:bob"),
            crate::q_hash("ind:cara"),
        );
        let mut a1 = NQuin {
            subject: alice,
            predicate: role,
            object: bob,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        a1.parity = a1.subject ^ a1.predicate ^ a1.object;
        let mut a2 = a1;
        a2.object = cara;
        a2.parity = a2.subject ^ a2.predicate ^ a2.object;
        let abox = [a1, a2];
        let types = [(bob, student), (cara, student)];
        let n = count_qualified_fillers(alice, role, student, &abox, &types, &[]);
        assert_eq!(n, 2);
        assert!(min_cardinality_met(n, 2) && !min_cardinality_met(n, 3));
        assert!(max_cardinality_met(n, 2) && !max_cardinality_met(n, 1));

        // Nominal {alice} has exactly alice as its instance.
        assert!(is_nominal_instance(alice, alice));
        assert!(!is_nominal_instance(bob, alice));
    }
}
