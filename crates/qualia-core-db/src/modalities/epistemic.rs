use crate::QualiaQuin;

/// OP_KNOWS = 0x20
pub const OP_KNOWS: u8 = 0x20;

/// OP_BELIEVES = 0x21
pub const OP_BELIEVES: u8 = 0x21;

/// OP_COMMON_KNOWLEDGE = 0x22
pub const OP_COMMON_KNOWLEDGE: u8 = 0x22;

pub const CERTAINTY_BIT_SHIFT: u32 = 8;
pub const NESTING_BIT_SHIFT: u32 = 16;

pub const THRESHOLD_CERTAIN_BELIEF: u8 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpistemicStatus {
    Active,
    Uncertain,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EpistemicVerdict {
    pub claim: QualiaQuin,
    pub status: EpistemicStatus,
    pub certainty: u8,
}

#[derive(Debug, PartialEq)]
pub enum EpistemicError {
    OutputBufferFull,
}

#[inline]
fn has_common_knowledge(ck_buf: &[u64], claim_fingerprint: u64) -> bool {
    let mut i = 0;
    while i < ck_buf.len() {
        if ck_buf[i] == claim_fingerprint {
            return true;
        }
        i += 1;
    }
    false
}

pub fn evaluate_epistemic_frame(
    quins: &[QualiaQuin],
    agent_did_hash: u64,
    world_hash: u64,
    out: &mut [EpistemicVerdict],
) -> Result<usize, EpistemicError> {
    let mut ck_buf = [0u64; 64];
    let mut ck_count = 0;

    for &q in quins {
        let opcode = (q.predicate & 0xFF) as u8;
        if opcode == OP_COMMON_KNOWLEDGE && ck_count < 64 {
            ck_buf[ck_count] = q.object;
            ck_count += 1;
        }
    }

    let active_ck = &ck_buf[..ck_count];
    let mut verdict_count = 0usize;

    for &q in quins {
        let opcode = (q.predicate & 0xFF) as u8;
        if !matches!(opcode, OP_KNOWS | OP_BELIEVES | OP_COMMON_KNOWLEDGE) {
            continue;
        }

        if agent_did_hash != 0 && q.subject != agent_did_hash && opcode != OP_COMMON_KNOWLEDGE {
            continue;
        }
        if world_hash != 0 && q.context != world_hash {
            continue;
        }

        let mut certainty = ((q.predicate >> CERTAINTY_BIT_SHIFT) & 0xFF) as u8;
        if has_common_knowledge(active_ck, q.object) {
            certainty = 255;
        }

        let status = if opcode == OP_BELIEVES && certainty < THRESHOLD_CERTAIN_BELIEF {
            EpistemicStatus::Uncertain
        } else {
            EpistemicStatus::Active
        };

        if verdict_count >= out.len() {
            return Err(EpistemicError::OutputBufferFull);
        }

        out[verdict_count] = EpistemicVerdict {
            claim: q,
            status,
            certainty,
        };
        verdict_count += 1;
    }

    Ok(verdict_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    fn build_epistemic_quin(
        subject: u64,
        opcode: u8,
        certainty: u8,
        claim_fingerprint: u64,
        world: u64,
    ) -> QualiaQuin {
        let predicate = (opcode as u64) | ((certainty as u64) << CERTAINTY_BIT_SHIFT);
        QualiaQuin {
            subject,
            predicate,
            object: claim_fingerprint,
            context: world,
            metadata: 0,
            parity: subject ^ predicate ^ claim_fingerprint ^ world,
        }
    }

    #[test]
    fn test_single_agent_knows_claim_active() {
        let agent = q_hash("agent1");
        let claim = q_hash("claim1");
        let q = build_epistemic_quin(agent, OP_KNOWS, 200, claim, 0);
        let mut out = [EpistemicVerdict {
            claim: q,
            status: EpistemicStatus::Skipped,
            certainty: 0,
        }; 4];
        assert_eq!(
            evaluate_epistemic_frame(&[q], agent, 0, &mut out).unwrap(),
            1
        );
        assert_eq!(out[0].status, EpistemicStatus::Active);
    }

    #[test]
    fn test_believes_below_threshold_uncertain() {
        let agent = q_hash("agent1");
        let claim = q_hash("claim1");
        let q = build_epistemic_quin(agent, OP_BELIEVES, 50, claim, 0);
        let mut out = [EpistemicVerdict {
            claim: q,
            status: EpistemicStatus::Skipped,
            certainty: 0,
        }; 4];
        assert_eq!(
            evaluate_epistemic_frame(&[q], agent, 0, &mut out).unwrap(),
            1
        );
        assert_eq!(out[0].status, EpistemicStatus::Uncertain);
    }

    #[test]
    fn test_common_knowledge_propagation() {
        let agent1 = q_hash("agent1");
        let claim = q_hash("claim1");
        let ck = build_epistemic_quin(0, OP_COMMON_KNOWLEDGE, 255, claim, 0);
        let b = build_epistemic_quin(agent1, OP_BELIEVES, 50, claim, 0);
        let mut out = [EpistemicVerdict {
            claim: ck,
            status: EpistemicStatus::Skipped,
            certainty: 0,
        }; 4];
        assert_eq!(
            evaluate_epistemic_frame(&[ck, b], 0, 0, &mut out).unwrap(),
            2
        );
        assert_eq!(out[1].status, EpistemicStatus::Active);
        assert_eq!(out[1].certainty, 255);
    }

    #[test]
    fn test_agent_filter_world_hash_mismatch() {
        let agent = q_hash("agent1");
        let world = q_hash("world1");
        let claim = q_hash("claim1");
        let q = build_epistemic_quin(agent, OP_KNOWS, 200, claim, world);
        let mut out = [EpistemicVerdict {
            claim: q,
            status: EpistemicStatus::Skipped,
            certainty: 0,
        }; 4];
        assert_eq!(
            evaluate_epistemic_frame(&[q], 0, q_hash("world2"), &mut out).unwrap(),
            0
        );
    }

    #[test]
    fn test_empty_slice_zero_verdicts() {
        let mut out = [EpistemicVerdict {
            claim: build_epistemic_quin(0, 0, 0, 0, 0),
            status: EpistemicStatus::Skipped,
            certainty: 0,
        }; 4];
        assert_eq!(evaluate_epistemic_frame(&[], 0, 0, &mut out).unwrap(), 0);
    }
}
