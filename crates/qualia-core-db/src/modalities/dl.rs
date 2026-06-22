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
        let e = |s: u64, o: u64| NQuin { subject: s, predicate: 1, object: o, context: 0, metadata: 0, parity: 0 };
        let (np, agent, human, moral) = (1u64, 2u64, 3u64, 4u64);
        let tbox = [
            e(np, agent),    // NaturalPerson ⊑ Agent       (first parent)
            e(np, human),    // NaturalPerson ⊑ HumanBeing  (SECOND parent — old impl missed this)
            e(human, moral), // HumanBeing ⊑ MoralFrame
            e(agent, moral), // Agent ⊑ MoralFrame          (diamond)
        ];
        assert!(check_subsumption_quin(np, agent, &tbox), "via first parent");
        assert!(check_subsumption_quin(np, human, &tbox), "via SECOND parent (multiple inheritance)");
        assert!(check_subsumption_quin(np, moral, &tbox), "transitively via either diamond path");
        assert!(!check_subsumption_quin(agent, human, &tbox), "Agent is not a HumanBeing");
    }
}
