use crate::QualiaQuin;

pub const OP_LTL_GLOBALLY: u8 = 0x40;
pub const OP_LTL_FINALLY: u8 = 0x41;
pub const OP_LTL_NEXT: u8 = 0x42;
pub const OP_LTL_UNTIL: u8 = 0x43;
pub const OP_LTL_RELEASE: u8 = 0x44;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LtlFormula {
    Globally(u64),
    Finally(u64),
    Next(u64),
    Until { ante: u64, consequent: u64 },
    Release { trigger: u64, invariant: u64 },
}

pub fn evaluate_ltl_trace(trace: &[QualiaQuin], formula: &LtlFormula) -> bool {
    match formula {
        LtlFormula::Globally(p) => {
            if trace.is_empty() {
                return false;
            }
            for quin in trace {
                if quin.predicate != *p {
                    return false;
                }
            }
            true
        }
        LtlFormula::Finally(p) => {
            if trace.is_empty() {
                return false;
            }
            for quin in trace {
                if quin.predicate == *p {
                    return true;
                }
            }
            false
        }
        LtlFormula::Next(p) => {
            if trace.len() < 2 {
                return false;
            }
            trace[1].predicate == *p
        }
        LtlFormula::Until { ante, consequent } => {
            if trace.is_empty() {
                return false;
            }
            for (i, quin) in trace.iter().enumerate() {
                if quin.predicate == *consequent {
                    let mut ante_held = true;
                    for j in 0..i {
                        if trace[j].predicate != *ante {
                            ante_held = false;
                            break;
                        }
                    }
                    if ante_held {
                        return true;
                    }
                }
            }
            false
        }
        LtlFormula::Release { trigger, invariant } => {
            if trace.is_empty() {
                return true;
            }
            for (i, quin) in trace.iter().enumerate() {
                if quin.predicate != *invariant {
                    let mut triggered = false;
                    for j in 0..=i {
                        if trace[j].predicate == *trigger {
                            triggered = true;
                            break;
                        }
                    }
                    if !triggered {
                        return false;
                    }
                }
            }
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::QualiaQuin;

    fn make_quin(predicate: u64) -> QualiaQuin {
        QualiaQuin {
            subject: 0,
            predicate,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        }
    }

    #[test]
    fn test_ltl_globally() {
        let p = 100;
        let q_p = make_quin(p);
        let q_not_p = make_quin(99);

        assert!(evaluate_ltl_trace(
            &[q_p, q_p, q_p],
            &LtlFormula::Globally(p)
        ));
        assert!(!evaluate_ltl_trace(
            &[q_p, q_not_p, q_p],
            &LtlFormula::Globally(p)
        ));
        assert!(!evaluate_ltl_trace(&[], &LtlFormula::Globally(p)));
    }

    #[test]
    fn test_ltl_finally() {
        let p = 100;
        let q_p = make_quin(p);
        let q_not_p = make_quin(99);

        assert!(evaluate_ltl_trace(
            &[q_not_p, q_not_p, q_p],
            &LtlFormula::Finally(p)
        ));
        assert!(!evaluate_ltl_trace(
            &[q_not_p, q_not_p],
            &LtlFormula::Finally(p)
        ));
        assert!(!evaluate_ltl_trace(&[], &LtlFormula::Finally(p)));
    }

    #[test]
    fn test_ltl_next() {
        let p = 100;
        let q_p = make_quin(p);
        let q_not_p = make_quin(99);

        assert!(evaluate_ltl_trace(&[q_not_p, q_p], &LtlFormula::Next(p)));
        assert!(!evaluate_ltl_trace(&[q_p, q_not_p], &LtlFormula::Next(p)));
        assert!(!evaluate_ltl_trace(&[q_p], &LtlFormula::Next(p)));
        assert!(!evaluate_ltl_trace(&[], &LtlFormula::Next(p)));
    }

    #[test]
    fn test_ltl_until() {
        let p = 100;
        let q = 200;
        let q_p = make_quin(p);
        let q_q = make_quin(q);
        let q_other = make_quin(99);

        assert!(evaluate_ltl_trace(
            &[q_p, q_p, q_q],
            &LtlFormula::Until {
                ante: p,
                consequent: q
            }
        ));
        assert!(evaluate_ltl_trace(
            &[q_q],
            &LtlFormula::Until {
                ante: p,
                consequent: q
            }
        ));
        assert!(!evaluate_ltl_trace(
            &[q_p, q_p, q_p],
            &LtlFormula::Until {
                ante: p,
                consequent: q
            }
        ));
        assert!(!evaluate_ltl_trace(
            &[q_p, q_other, q_q],
            &LtlFormula::Until {
                ante: p,
                consequent: q
            }
        ));
        assert!(!evaluate_ltl_trace(
            &[],
            &LtlFormula::Until {
                ante: p,
                consequent: q
            }
        ));
    }

    #[test]
    fn test_ltl_release() {
        let trigger = 100;
        let invariant = 200;
        let q_t = make_quin(trigger);
        let q_i = make_quin(invariant);
        let q_other = make_quin(99);

        assert!(evaluate_ltl_trace(
            &[q_i, q_i, q_i],
            &LtlFormula::Release { trigger, invariant }
        ));
        assert!(evaluate_ltl_trace(
            &[q_i, q_t, q_other],
            &LtlFormula::Release { trigger, invariant }
        ));
        assert!(!evaluate_ltl_trace(
            &[q_i, q_other, q_t],
            &LtlFormula::Release { trigger, invariant }
        ));
        assert!(evaluate_ltl_trace(
            &[],
            &LtlFormula::Release { trigger, invariant }
        ));
    }
}
