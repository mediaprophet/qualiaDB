use crate::{q_hash, NQuin};

pub const OP_KNOWS: u8 = 0x20;
pub const OP_BELIEVES: u8 = 0x21;
pub const OP_COMMON_KNOWLEDGE: u8 = 0x22;
pub const OP_INTENT_LOCK: u8 = 0x23;
pub const OP_IS_LOCKED: u8 = 0x24;
pub const OP_NAMESPACE_LOCK: u8 = 0x25;
pub const OP_NAMESPACE_IS_LOCKED: u8 = 0x26;

pub const CERTAINTY_BIT_SHIFT: u32 = 8;
pub const NESTING_BIT_SHIFT: u32 = 16;

// Named epistemic-strength bands for the `certainty` byte — the assertive/doxastic axis
// (see core-ontologies/modal-junctures.n3). A juncture verb maps to a band; the eval is
// Active at >= 128, so Knows/Affirms/Believes/Recognizes/Considers are Active and
// Supposes/Suspects/Speculates/Doubts are Uncertain ("speculates" = a low-certainty
// belief). The ILLOCUTIONARY speech acts (proclaims/declares/recommends/undertakes) are a
// SEPARATE axis, not certainty — they route to deontic / soft-deontic / performative.
pub const CERTAINTY_KNOWS: u8 = 255;
pub const CERTAINTY_AFFIRMS: u8 = 230;
pub const CERTAINTY_BELIEVES: u8 = 200;
pub const CERTAINTY_RECOGNIZES: u8 = 200;
pub const CERTAINTY_CONSIDERS: u8 = 128;
pub const CERTAINTY_SUPPOSES: u8 = 100;
pub const CERTAINTY_SUSPECTS: u8 = 80;
pub const CERTAINTY_SPECULATES: u8 = 50;
pub const CERTAINTY_DOUBTS: u8 = 20;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EpistemicStatus {
    Active,
    Uncertain,
    Skipped,
}

#[derive(Debug)]
pub enum EpistemicError {
    BufferOverflow,
    NodeLocked(u64),      // Hash of the locked node
    NamespaceLocked(u64), // Hash of the locked namespace context
}

#[derive(Debug, Clone, Copy)]
pub struct EpistemicVerdict {
    pub claim: NQuin,
    pub status: EpistemicStatus,
    pub certainty: u8,
}

/// Evaluates a slice of Quins for epistemic/doxastic claims.
pub fn evaluate_epistemic_frame(
    quins: &[NQuin],
    agent_did_hash: u64, // 0 = accept all agents
    world_hash: u64,     // 0 = accept all worlds
    out: &mut [EpistemicVerdict],
) -> Result<usize, EpistemicError> {
    let mut count = 0;

    for q in quins {
        if world_hash != 0 && q.context != world_hash {
            continue;
        }
        if agent_did_hash != 0 && q.subject != agent_did_hash {
            continue;
        }

        let opcode = (q.predicate & 0xFF) as u8;
        if opcode != OP_KNOWS && opcode != OP_BELIEVES && opcode != OP_COMMON_KNOWLEDGE {
            continue;
        }

        let certainty = ((q.predicate >> CERTAINTY_BIT_SHIFT) & 0xFF) as u8;

        let status = if certainty >= 128 || opcode == OP_KNOWS || opcode == OP_COMMON_KNOWLEDGE {
            EpistemicStatus::Active
        } else {
            EpistemicStatus::Uncertain
        };

        if count >= out.len() {
            return Err(EpistemicError::BufferOverflow);
        }

        out[count] = EpistemicVerdict {
            claim: *q,
            status,
            certainty,
        };
        count += 1;
    }

    Ok(count)
}

/// Checks if any nodes requested by an intent quin are locked by another agent.
pub fn check_node_locks(
    intent_quins: &[NQuin],
    current_graph: &[NQuin],
    agent_did_hash: u64,
) -> Result<(), EpistemicError> {
    for i_quin in intent_quins {
        let opcode = (i_quin.predicate & 0xFF) as u8;

        // 1. Check Namespace Locks First
        if opcode == OP_NAMESPACE_LOCK {
            let target_namespace = i_quin.object;
            // Ensure no other agent holds a namespace lock on this target
            for c_quin in current_graph {
                let c_opcode = (c_quin.predicate & 0xFF) as u8;
                if c_opcode == OP_NAMESPACE_IS_LOCKED && c_quin.object == target_namespace {
                    if c_quin.subject != agent_did_hash {
                        return Err(EpistemicError::NamespaceLocked(target_namespace));
                    }
                }
            }
        }

        // 2. Check Standard Node Locks
        if opcode == OP_INTENT_LOCK {
            let target_node = i_quin.object;
            let target_namespace = i_quin.context; // The namespace context of the intent

            for c_quin in current_graph {
                let c_opcode = (c_quin.predicate & 0xFF) as u8;

                // If the entire namespace is locked by another agent, reject the node lock
                if c_opcode == OP_NAMESPACE_IS_LOCKED && c_quin.object == target_namespace {
                    if c_quin.subject != agent_did_hash {
                        return Err(EpistemicError::NamespaceLocked(target_namespace));
                    }
                }

                // If the specific node is locked by another agent, reject the node lock
                if c_opcode == OP_IS_LOCKED && c_quin.object == target_node {
                    if c_quin.subject != agent_did_hash {
                        return Err(EpistemicError::NodeLocked(target_node));
                    }
                }
            }
        }
    }
    Ok(())
}

// ─── Multi-agent epistemic operators (E, C, D), introspection, AGM revision ───────

// AGM belief revision (expand / contract / revise over a signed-literal belief base) lives in
// `modal.rs` and is re-exported here — belief revision IS an epistemic operation.
pub use crate::modalities::modal::{
    contract as agm_contract, expand as agm_expand, is_consistent as belief_set_consistent,
    revise as agm_revise, Belief,
};

/// **Everyone-knows** `E φ`: every agent in the group knows φ. `agent_knows[i]` = does agent i know φ?
pub fn everyone_knows(agent_knows: &[bool]) -> bool {
    !agent_knows.is_empty() && agent_knows.iter().all(|&k| k)
}

/// **Distributed knowledge** `D φ`: the group COLLECTIVELY knows φ by pooling — φ is entailed by
/// the union of what agents individually know. Modelled over fact-fragments: φ is distributed-
/// known iff every fragment in `required` appears in `known` (the union of all agents' fragments).
pub fn distributed_knowledge(required: &[u64], known: &[u64]) -> bool {
    !required.is_empty() && required.iter().all(|r| known.contains(r))
}

/// **Common knowledge** `C φ`: practically established by a PUBLIC ANNOUNCEMENT to the whole group
/// (everyone knows φ, everyone knows that everyone knows, ad infinitum). Holds iff the announcement
/// was perceived by everyone.
#[inline]
pub fn common_knowledge_via_announcement(everyone_perceived: bool) -> bool {
    everyone_perceived
}

/// **Positive introspection** (axiom 4): `Kφ → KKφ` — knowing implies knowing that one knows.
#[inline]
pub fn positive_introspection(knows_p: bool) -> bool {
    knows_p
}

/// **Negative introspection** (axiom 5, S5): `¬Kφ → K¬Kφ` — not-knowing implies knowing one doesn't.
#[inline]
pub fn negative_introspection(knows_p: bool) -> bool {
    !knows_p
}

/// The **Muddy Children** deduction: after the public announcement "at least one is muddy", each
/// silent round eliminates a hypothesis; a muddy child deduces it is muddy exactly at
/// `round == num_muddy` (1-indexed). Shows how common knowledge + iterated "I don't know" produces
/// knowledge. Returns whether a muddy child KNOWS its own state at `round`.
pub fn muddy_child_knows(num_muddy: u32, round: u32) -> bool {
    num_muddy > 0 && round >= num_muddy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_epistemic_evaluation() {
        let mut out = Vec::with_capacity(10);
        for _ in 0..10 {
            out.push(EpistemicVerdict {
                claim: NQuin::default(),
                status: EpistemicStatus::Skipped,
                certainty: 0,
            });
        }

        let agent_a = q_hash("agent_a");
        let agent_b = q_hash("agent_b");
        let world_w = q_hash("world_w");

        let mut q_knows = NQuin::default();
        q_knows.subject = agent_a;
        q_knows.predicate = (200u64 << CERTAINTY_BIT_SHIFT) | (OP_KNOWS as u64);
        q_knows.context = world_w;

        let mut q_believes_low = NQuin::default();
        q_believes_low.subject = agent_a;
        q_believes_low.predicate = (50u64 << CERTAINTY_BIT_SHIFT) | (OP_BELIEVES as u64);
        q_believes_low.context = world_w;

        let mut q_wrong_world = NQuin::default();
        q_wrong_world.subject = agent_a;
        q_wrong_world.predicate = (200u64 << CERTAINTY_BIT_SHIFT) | (OP_KNOWS as u64);
        q_wrong_world.context = q_hash("other_world");

        let quins = [q_knows, q_believes_low, q_wrong_world];

        // 1. Single-agent K_a(p)
        let count = evaluate_epistemic_frame(&quins, agent_a, world_w, &mut out).unwrap();
        assert_eq!(count, 2);
        assert_eq!(out[0].status, EpistemicStatus::Active); // Knows
        assert_eq!(out[1].status, EpistemicStatus::Uncertain); // Believes low certainty

        // 2. World filter
        let mut out2 = Vec::with_capacity(10);
        for _ in 0..10 {
            out2.push(EpistemicVerdict {
                claim: NQuin::default(),
                status: EpistemicStatus::Skipped,
                certainty: 0,
            });
        }
        let count2 = evaluate_epistemic_frame(&quins, 0, q_hash("other_world"), &mut out2).unwrap();
        assert_eq!(count2, 1);

        // 3. Agent filter
        let count3 = evaluate_epistemic_frame(&quins, agent_b, 0, &mut out2).unwrap();
        assert_eq!(count3, 0);

        // 4. Empty slice
        let count4 = evaluate_epistemic_frame(&[], 0, 0, &mut out2).unwrap();
        assert_eq!(count4, 0);
    }

    #[test]
    fn epistemic_strength_bands_map_to_active_or_uncertain() {
        // The named certainty bands (modal-junctures.n3) route through the eval: confident
        // attitudes (knows/believes/considers) are Active; tentative ones (supposes/
        // speculates/doubts) are Uncertain — "speculates" as a low-certainty belief.
        let agent = q_hash("agent");
        let world = q_hash("world");
        let mk = |band: u8| {
            let mut q = NQuin::default();
            q.subject = agent;
            q.predicate = ((band as u64) << CERTAINTY_BIT_SHIFT) | (OP_BELIEVES as u64);
            q.context = world;
            q
        };
        let mut out = vec![
            EpistemicVerdict {
                claim: NQuin::default(),
                status: EpistemicStatus::Skipped,
                certainty: 0,
            };
            4
        ];

        for b in [CERTAINTY_KNOWS, CERTAINTY_BELIEVES, CERTAINTY_CONSIDERS] {
            evaluate_epistemic_frame(&[mk(b)], agent, world, &mut out).unwrap();
            assert_eq!(
                out[0].status,
                EpistemicStatus::Active,
                "band {b} should be Active"
            );
        }
        for b in [CERTAINTY_SUPPOSES, CERTAINTY_SPECULATES, CERTAINTY_DOUBTS] {
            evaluate_epistemic_frame(&[mk(b)], agent, world, &mut out).unwrap();
            assert_eq!(
                out[0].status,
                EpistemicStatus::Uncertain,
                "band {b} should be Uncertain"
            );
        }
    }

    #[test]
    fn test_namespace_lock_blocks_node_lock() {
        let agent_a = q_hash("agent_a");
        let agent_b = q_hash("agent_b");
        let target_namespace = q_hash("specialized_libs/");
        let target_node = q_hash("specialized_libs/file.rs");

        // Graph: Agent A holds a namespace lock on specialized_libs/
        let mut namespace_locked = NQuin::default();
        namespace_locked.subject = agent_a;
        namespace_locked.predicate = OP_NAMESPACE_IS_LOCKED as u64;
        namespace_locked.object = target_namespace;

        let current_graph = vec![namespace_locked];

        // Intent: Agent B tries to lock a specific node within that namespace
        let mut intent_node_lock = NQuin::default();
        intent_node_lock.subject = agent_b;
        intent_node_lock.predicate = OP_INTENT_LOCK as u64;
        intent_node_lock.object = target_node;
        intent_node_lock.context = target_namespace;

        let result = check_node_locks(&[intent_node_lock], &current_graph, agent_b);
        assert!(
            matches!(result, Err(EpistemicError::NamespaceLocked(ns)) if ns == target_namespace)
        );

        // Intent: Agent A tries to lock a node within their own namespace lock -> Should Succeed
        let mut intent_node_lock_a = NQuin::default();
        intent_node_lock_a.subject = agent_a;
        intent_node_lock_a.predicate = OP_INTENT_LOCK as u64;
        intent_node_lock_a.object = target_node;
        intent_node_lock_a.context = target_namespace;

        let result_a = check_node_locks(&[intent_node_lock_a], &current_graph, agent_a);
        assert!(result_a.is_ok());
    }

    #[test]
    fn multi_agent_operators_and_introspection() {
        // Everyone-knows: all agents must know it.
        assert!(everyone_knows(&[true, true, true]));
        assert!(!everyone_knows(&[true, false, true]));
        assert!(!everyone_knows(&[]));
        // Distributed knowledge: pooled fragments cover the requirement.
        let (a, b, c) = (q_hash("f:a"), q_hash("f:b"), q_hash("f:c"));
        assert!(distributed_knowledge(&[a, b], &[a, b, c]));
        assert!(
            !distributed_knowledge(&[a, b], &[a, c]),
            "missing fragment b"
        );
        // Common knowledge via public announcement.
        assert!(common_knowledge_via_announcement(true));
        assert!(!common_knowledge_via_announcement(false));
        // Introspection axioms (S5).
        assert!(positive_introspection(true) && !positive_introspection(false));
        assert!(negative_introspection(false) && !negative_introspection(true));
    }

    #[test]
    fn muddy_children_deduction() {
        // 2 muddy children: nobody knows in round 1; each deduces at round 2.
        assert!(!muddy_child_knows(2, 1));
        assert!(muddy_child_knows(2, 2));
        // 1 muddy child knows immediately (round 1, from the announcement).
        assert!(muddy_child_knows(1, 1));
    }

    #[test]
    fn common_knowledge_propagation_across_two_agents() {
        // Two agents both know φ (OP_KNOWS, certainty 255) in the same world.
        // When everyone knows φ and a public announcement is made, φ becomes
        // common knowledge. This tests the propagation path:
        //   individual knowledge → everyone-knows → common knowledge
        let agent_a = q_hash("agent_a");
        let agent_b = q_hash("agent_b");
        let world_w = q_hash("world_w");

        let mk_knows = |agent: u64| {
            let mut q = NQuin::default();
            q.subject = agent;
            q.predicate = (255u64 << CERTAINTY_BIT_SHIFT) | (OP_KNOWS as u64);
            q.context = world_w;
            q
        };

        let quins = [mk_knows(agent_a), mk_knows(agent_b)];

        // Evaluate: both agents' knowledge claims should be Active
        let mut out = [
            EpistemicVerdict {
                claim: NQuin::default(),
                status: EpistemicStatus::Skipped,
                certainty: 0,
            };
            4
        ];
        let count = evaluate_epistemic_frame(&quins, 0, world_w, &mut out).unwrap();
        assert_eq!(count, 2, "both agents' claims must be evaluated");
        assert_eq!(out[0].status, EpistemicStatus::Active);
        assert_eq!(out[1].status, EpistemicStatus::Active);

        // Both agents know → everyone_knows is true
        let agent_knows = [out[0].status == EpistemicStatus::Active, out[1].status == EpistemicStatus::Active];
        assert!(everyone_knows(&agent_knows), "everyone knows φ");

        // Public announcement → common knowledge
        assert!(
            common_knowledge_via_announcement(everyone_knows(&agent_knows)),
            "φ becomes common knowledge when everyone knows and it is publicly announced"
        );
    }

    #[test]
    fn agm_belief_revision_is_available_in_the_epistemic_namespace() {
        // The AGM operators (from modal.rs) are re-exported here; revise is consistent.
        let p = Belief {
            atom: 1,
            positive: true,
        };
        let not_p = Belief {
            atom: 1,
            positive: false,
        };
        let mut out = [Belief {
            atom: 0,
            positive: true,
        }; 4];
        let n = agm_revise(&[not_p], p, &mut out);
        assert!(out[..n].contains(&p) && !out[..n].contains(&not_p));
        assert!(belief_set_consistent(&out[..n]));
        let _ = agm_expand;
        let _ = agm_contract;
    }
}
