use crate::QualiaQuin;

/// Returns true if sub_class_hash is subsumed by super_class_hash in the TBox slice
pub fn check_subsumption_quin(
    sub_class_hash: u64,
    super_class_hash: u64,
    tbox: &[QualiaQuin],   // Quins with predicate = q_hash("rdfs:subClassOf")
) -> bool {
    if sub_class_hash == super_class_hash {
        return true;
    }
    
    // Simple transitive closure (DFS without recursion / bounded loop)
    // To stay zero allocation, we just loop up to a max depth.
    let mut current = sub_class_hash;
    for _ in 0..64 {
        let mut found = false;
        for quin in tbox {
            if quin.subject == current {
                current = quin.object;
                found = true;
                if current == super_class_hash {
                    return true;
                }
                break;
            }
        }
        if !found {
            break;
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
            QualiaQuin { subject: 10, predicate: 0, object: 20, context: 0, metadata: 0, parity: 0 },
            QualiaQuin { subject: 20, predicate: 0, object: 30, context: 0, metadata: 0, parity: 0 },
        ];
        
        assert_eq!(check_subsumption_quin(10, 10, &tbox), true);
        assert_eq!(check_subsumption_quin(10, 20, &tbox), true);
        assert_eq!(check_subsumption_quin(10, 30, &tbox), true);
        assert_eq!(check_subsumption_quin(10, 40, &tbox), false);
        assert_eq!(check_subsumption_quin(20, 10, &tbox), false);
    }
}
