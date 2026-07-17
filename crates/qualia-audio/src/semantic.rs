//! Auditory observations → NQuin-compatible records (epistemic).

use crate::hash::{q_hash, MediaDigest};
use crate::types::AuditoryEvent;

pub const P_AUDITORY_OBSERVATION: &str = "https://ns.webizen.org/q42/AuditoryObservation";
pub const P_PROPOSES_SOUND_CLASS: &str = "https://ns.webizen.org/q42/proposesSoundClass";
pub const P_MODEL_DIGEST: &str = "https://ns.webizen.org/q42/modelDigest";
pub const P_HUMAN_REJECTS: &str = "https://ns.webizen.org/q42/humanRejects";
pub const P_HUMAN_CORRECTS: &str = "https://ns.webizen.org/q42/humanCorrectsClass";
pub const CTX_AUDIO: &str = "https://ns.webizen.org/q42/audio-observation";
pub const CTX_HUMAN: &str = "https://ns.webizen.org/q42/human-attestation";

pub const MAX_OBS_QUINS: usize = 256;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioQuin {
    pub subject: u64,
    pub predicate: u64,
    pub object: u64,
    pub context: u64,
    pub metadata: u64,
    pub parity: u64,
}

impl AudioQuin {
    pub fn with_parity(s: u64, p: u64, o: u64, c: u64, m: u64) -> Self {
        Self {
            subject: s,
            predicate: p,
            object: o,
            context: c,
            metadata: m,
            parity: s ^ p ^ o ^ c ^ m,
        }
    }
}

pub fn compile_auditory_quins(
    media: MediaDigest,
    events: &[AuditoryEvent],
    model_hash: u64,
    out: &mut [AudioQuin],
) -> usize {
    if out.is_empty() {
        return 0;
    }
    let mut w = 0usize;
    out[w] = AudioQuin::with_parity(
        media.hash,
        q_hash(P_MODEL_DIGEST),
        model_hash,
        q_hash(CTX_AUDIO),
        media.byte_len,
    );
    w += 1;
    let ctx = q_hash(CTX_AUDIO) ^ model_hash;
    for e in events {
        if e.class_hash == 0 && e.confidence_u16 == 0 {
            continue;
        }
        if w + 2 > out.len() {
            break;
        }
        let inst = e.source_hash
            ^ e.start_frame
            ^ (e.end_frame.wrapping_mul(0x9e37_79b9));
        let meta = (e.confidence_u16 as u64)
            | ((e.start_frame & 0xFFFF) << 16)
            | ((e.end_frame & 0xFFFF) << 32);
        out[w] = AudioQuin::with_parity(
            media.hash,
            q_hash(P_AUDITORY_OBSERVATION),
            inst,
            ctx,
            meta,
        );
        w += 1;
        out[w] = AudioQuin::with_parity(
            inst,
            q_hash(P_PROPOSES_SOUND_CLASS),
            e.class_hash,
            ctx,
            e.confidence_u16 as u64,
        );
        w += 1;
    }
    w
}

pub fn human_reject_quin(human: u64, instance: u64, reason: u64) -> AudioQuin {
    AudioQuin::with_parity(
        human,
        q_hash(P_HUMAN_REJECTS),
        instance,
        q_hash(CTX_HUMAN),
        reason,
    )
}

pub fn human_correct_quin(human: u64, instance: u64, new_class: u64) -> AudioQuin {
    AudioQuin::with_parity(
        human,
        q_hash(P_HUMAN_CORRECTS),
        new_class,
        q_hash(CTX_HUMAN) ^ instance,
        instance,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AuditoryEvent;

    #[test]
    fn compile_digest_and_events() {
        let mut e = AuditoryEvent::empty();
        e.class_hash = 9;
        e.confidence_u16 = 1000;
        e.start_frame = 0;
        e.end_frame = 100;
        e.source_hash = 1;
        let mut out = [AudioQuin::with_parity(0, 0, 0, 0, 0); 8];
        let n = compile_auditory_quins(
            MediaDigest {
                hash: 5,
                byte_len: 10,
            },
            &[e],
            7,
            &mut out,
        );
        assert_eq!(n, 3);
        assert_eq!(out[0].predicate, q_hash(P_MODEL_DIGEST));
    }
}
