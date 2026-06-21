use crate::{NQuin, q_hash};

pub const OP_KNOWS: u8 = 0x20;
pub const OP_BELIEVES: u8 = 0x21;
pub const OP_COMMON_KNOWLEDGE: u8 = 0x22;
pub const OP_INTENT_LOCK: u8 = 0x23;
pub const OP_IS_LOCKED: u8 = 0x24;
pub const OP_NAMESPACE_LOCK: u8 = 0x25;
pub const OP_NAMESPACE_IS_LOCKED: u8 = 0x26;

pub const CERTAINTY_BIT_SHIFT: u32 = 8;
pub const NESTING_BIT_SHIFT: u32 = 16;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum EpistemicStatus {
    Active,
    Uncertain,
    Skipped,
}

#[derive(Debug)]
pub enum EpistemicError {
    BufferOverflow,
    NodeLocked(u64), // Hash of the locked node
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
    agent_did_hash: u64,    // 0 = accept all agents
    world_hash: u64,        // 0 = accept all worlds
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
        for _ in 0..10 { out2.push(EpistemicVerdict { claim: NQuin::default(), status: EpistemicStatus::Skipped, certainty: 0 }); }
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
        assert!(matches!(result, Err(EpistemicError::NamespaceLocked(ns)) if ns == target_namespace));

        // Intent: Agent A tries to lock a node within their own namespace lock -> Should Succeed
        let mut intent_node_lock_a = NQuin::default();
        intent_node_lock_a.subject = agent_a;
        intent_node_lock_a.predicate = OP_INTENT_LOCK as u64;
        intent_node_lock_a.object = target_node;
        intent_node_lock_a.context = target_namespace;

        let result_a = check_node_locks(&[intent_node_lock_a], &current_graph, agent_a);
        assert!(result_a.is_ok());
    }
}
