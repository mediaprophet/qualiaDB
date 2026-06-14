use crate::{NQuin, q_hash};

pub const OP_KNOWS: u8 = 0x20;
pub const OP_BELIEVES: u8 = 0x21;
pub const OP_COMMON_KNOWLEDGE: u8 = 0x22;
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
}
