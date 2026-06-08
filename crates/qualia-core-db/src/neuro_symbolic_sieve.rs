//! Grammar-constrained FSM sieve for neuro-symbolic LLM output (zero-heap hot path).

use crate::q_hash;
use crate::QualiaQuin;

/// Max allowed token IDs per FSM state (stack-only mask).
pub const MAX_SIEVE_ALLOW: usize = 16;

/// One lexicon-bound token slot in a state mask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SieveSlot {
    pub token_id: u32,
    pub lexicon_hash: u64,
}

/// Stack mask: linear scan during chunked argmax (no `HashMap` / `Vec`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SieveStateMask {
    pub slots: [SieveSlot; MAX_SIEVE_ALLOW],
    pub len: u8,
}

impl SieveStateMask {
    pub const EMPTY: Self = Self {
        slots: [SieveSlot { token_id: 0, lexicon_hash: 0 }; MAX_SIEVE_ALLOW],
        len: 0,
    };

    #[inline]
    pub fn allows(&self, token_id: u32) -> bool {
        if self.len == 0 {
            return true;
        }
        for i in 0..self.len as usize {
            if self.slots[i].token_id == token_id {
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn lexicon_hash_for(&self, token_id: u32) -> Option<u64> {
        for i in 0..self.len as usize {
            if self.slots[i].token_id == token_id {
                return Some(self.slots[i].lexicon_hash);
            }
        }
        None
    }

    pub(crate) fn push(&mut self, token_id: u32, lexicon_hash: u64) {
        let n = self.len as usize;
        if n >= MAX_SIEVE_ALLOW {
            return;
        }
        for i in 0..n {
            if self.slots[i].token_id == token_id {
                return;
            }
        }
        self.slots[n] = SieveSlot { token_id, lexicon_hash };
        self.len += 1;
    }
}

/// FSM states for Subject → Predicate → Object graph emission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SieveState {
    ExpectSubject = 0,
    ExpectPredicate = 1,
    ExpectObject = 2,
    Complete = 3,
}

/// Sieve exhausted: no allowed token had finite logit mass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SieveError {
    Misaligned,
    AlreadyComplete,
}

/// Stack-only IRI hash lists for dynamic `.q42.lex` mask population (cold path).
#[derive(Debug, Clone, Copy)]
pub struct SieveLexSpec {
    pub subjects: [u64; MAX_SIEVE_ALLOW],
    pub subjects_len: u8,
    pub predicates: [u64; MAX_SIEVE_ALLOW],
    pub predicates_len: u8,
    pub objects: [u64; MAX_SIEVE_ALLOW],
    pub objects_len: u8,
}

impl SieveLexSpec {
    pub const EMPTY: Self = Self {
        subjects: [0; MAX_SIEVE_ALLOW],
        subjects_len: 0,
        predicates: [0; MAX_SIEVE_ALLOW],
        predicates_len: 0,
        objects: [0; MAX_SIEVE_ALLOW],
        objects_len: 0,
    };

    pub fn push_subject(&mut self, hash: u64) {
        let n = self.subjects_len as usize;
        if n < MAX_SIEVE_ALLOW {
            self.subjects[n] = hash;
            self.subjects_len += 1;
        }
    }

    pub fn push_predicate(&mut self, hash: u64) {
        let n = self.predicates_len as usize;
        if n < MAX_SIEVE_ALLOW {
            self.predicates[n] = hash;
            self.predicates_len += 1;
        }
    }

    pub fn push_object(&mut self, hash: u64) {
        let n = self.objects_len as usize;
        if n < MAX_SIEVE_ALLOW {
            self.objects[n] = hash;
            self.objects_len += 1;
        }
    }

    /// Default graph-mutation triple for clinical / conduct intents.
    pub fn graph_mutation_default() -> Self {
        let mut s = Self::EMPTY;
        s.push_subject(q_hash("schema:Patient"));
        s.push_subject(q_hash("q42:subject"));
        s.push_predicate(q_hash("snomed:hasFever"));
        s.push_predicate(q_hash("q42:conductViolation"));
        s.push_predicate(q_hash("q42:hasGuardian"));
        s.push_object(q_hash("xsd:true"));
        s.push_object(q_hash("q42:entity"));
        s
    }

    /// Fever observation triple used by the e2e WAL pipeline test.
    pub fn fever_observation() -> Self {
        let mut s = Self::EMPTY;
        s.push_subject(q_hash("Patient"));
        s.push_predicate(q_hash("fever"));
        s.push_object(q_hash("True"));
        s
    }
}

/// Neuro-symbolic grammar sieve (masks built once from tokenizer + lex — cold path may alloc).
#[derive(Debug, Clone)]
pub struct NeuroSymbolicSieve {
    masks: [SieveStateMask; 3],
    state: SieveState,
    subject_hash: u64,
    predicate_hash: u64,
    object_hash: u64,
    emitted_tokens: [u32; 3],
    emitted_len: u8,
}

impl NeuroSymbolicSieve {
    /// Build masks from a memory-mapped `.q42.lex` view and GGUF tokenizer (load-time only).
    pub fn from_lex_and_tokenizer(
        lex: &crate::q42_lex::Q42LexMmap<'_>,
        tok: &crate::gguf_sharder::GgufTokenizer,
        spec: &SieveLexSpec,
    ) -> Self {
        let mut sieve = Self::empty_fsm();
        fill_mask_from_lex(&mut sieve.masks[0], lex, tok, &spec.subjects[..spec.subjects_len as usize]);
        fill_mask_from_lex(
            &mut sieve.masks[1],
            lex,
            tok,
            &spec.predicates[..spec.predicates_len as usize],
        );
        fill_mask_from_lex(&mut sieve.masks[2], lex, tok, &spec.objects[..spec.objects_len as usize]);
        sieve
    }

    /// Tokenizer-only fallback when no `.q42.lex` sidecar is loaded (dev / tests).
    pub fn from_gguf_tokenizer(tok: &crate::gguf_sharder::GgufTokenizer) -> Self {
        let mut sieve = Self::empty_fsm();
        const SUBJECTS: &[(&str, u64)] = &[
            ("Webizen", q_hash("q42:webizenAgent")),
            ("Agent", q_hash("q42:agent")),
            ("Subject", q_hash("q42:subject")),
        ];
        const PREDICATES: &[(&str, u64)] = &[
            ("conductViolation", q_hash("q42:conductViolation")),
            ("hasGuardian", q_hash("q42:hasGuardian")),
            ("violation", q_hash("q42:conductViolation")),
            ("Guardian", q_hash("q42:hasGuardian")),
        ];
        const OBJECTS: &[(&str, u64)] = &[
            ("guardian", q_hash("q42:guardianEntity")),
            ("Entity", q_hash("q42:entity")),
            ("Object", q_hash("q42:object")),
        ];
        fill_mask_literal(&mut sieve.masks[0], tok, SUBJECTS);
        fill_mask_literal(&mut sieve.masks[1], tok, PREDICATES);
        fill_mask_literal(&mut sieve.masks[2], tok, OBJECTS);
        sieve
    }

    pub(crate) fn empty_fsm() -> Self {
        Self {
            masks: [SieveStateMask::EMPTY; 3],
            state: SieveState::ExpectSubject,
            subject_hash: 0,
            predicate_hash: 0,
            object_hash: 0,
            emitted_tokens: [0; 3],
            emitted_len: 0,
        }
    }

    #[inline]
    pub fn state(&self) -> SieveState {
        self.state
    }

    #[inline]
    pub fn is_complete(&self) -> bool {
        self.state == SieveState::Complete
    }

    #[inline]
    pub fn emitted_len(&self) -> u8 {
        self.emitted_len
    }

    #[inline]
    pub fn current_mask(&self) -> &SieveStateMask {
        match self.state {
            SieveState::ExpectSubject => &self.masks[0],
            SieveState::ExpectPredicate => &self.masks[1],
            SieveState::ExpectObject => &self.masks[2],
            SieveState::Complete => &SieveStateMask::EMPTY,
        }
    }

    /// Apply a sieve-selected token and advance the FSM.
    pub fn apply_token(&mut self, token_id: u32) -> Result<(), SieveError> {
        if self.state == SieveState::Complete {
            return Err(SieveError::AlreadyComplete);
        }
        let mask = self.current_mask();
        if mask.len == 0 {
            return Err(SieveError::Misaligned);
        }
        let hash = mask.lexicon_hash_for(token_id).ok_or(SieveError::Misaligned)?;
        match self.state {
            SieveState::ExpectSubject => {
                self.subject_hash = hash;
                self.state = SieveState::ExpectPredicate;
            }
            SieveState::ExpectPredicate => {
                self.predicate_hash = hash;
                self.state = SieveState::ExpectObject;
            }
            SieveState::ExpectObject => {
                self.object_hash = hash;
                self.state = SieveState::Complete;
            }
            SieveState::Complete => return Err(SieveError::AlreadyComplete),
        }
        let n = self.emitted_len as usize;
        if n < 3 {
            self.emitted_tokens[n] = token_id;
            self.emitted_len += 1;
        }
        Ok(())
    }

    /// Assemble the 48-byte `QualiaQuin` from constrained emissions (stack only).
    pub fn assemble_quin(&self, context_hash: u64) -> QualiaQuin {
        let mut quin = QualiaQuin {
            subject: self.subject_hash,
            predicate: self.predicate_hash,
            object: self.object_hash,
            context: context_hash,
            metadata: 0,
            parity: 0,
        };
        quin.parity = quin.subject ^ quin.predicate ^ quin.object ^ quin.context;
        quin
    }

    pub fn masks_ready(&self) -> bool {
        self.masks[0].len > 0 && self.masks[1].len > 0 && self.masks[2].len > 0
    }

    /// First resolved token ID per FSM slot (cold-path / test helper).
    pub fn resolved_token_triple(&self) -> Option<(u32, u32, u32)> {
        if !self.masks_ready() {
            return None;
        }
        Some((
            self.masks[0].slots[0].token_id,
            self.masks[1].slots[0].token_id,
            self.masks[2].slots[0].token_id,
        ))
    }
}

fn fill_mask_literal(mask: &mut SieveStateMask, tok: &crate::gguf_sharder::GgufTokenizer, entries: &[(&str, u64)]) {
    for &(text, hash) in entries {
        let ids = tok.encode(text);
        if let Some(&id) = ids.first() {
            mask.push(id, hash);
        }
    }
}

fn fill_mask_from_lex(
    mask: &mut SieveStateMask,
    lex: &crate::q42_lex::Q42LexMmap<'_>,
    tok: &crate::gguf_sharder::GgufTokenizer,
    hashes: &[u64],
) {
    for &hash in hashes {
        if let Some(text) = lex.lookup_hash(hash) {
            let ids = tok.encode(text);
            if let Some(&id) = ids.first() {
                mask.push(id, hash);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_lex_bytes(entries: &[(u64, &str)]) -> Vec<u8> {
        let mut sorted: Vec<(u64, &str)> = entries.to_vec();
        sorted.sort_unstable_by_key(|(h, _)| *h);
        let entry_count = sorted.len() as u64;
        let strings_offset = 32 + entry_count * 16;
        let mut blob = Vec::new();
        let mut index = Vec::new();
        for (hash, text) in &sorted {
            let str_off = blob.len() as u64;
            let b = text.as_bytes();
            let len = b.len().min(65535) as u16;
            blob.extend_from_slice(&len.to_le_bytes());
            blob.extend_from_slice(&b[..len as usize]);
            index.extend_from_slice(&hash.to_le_bytes());
            index.extend_from_slice(&str_off.to_le_bytes());
        }
        let mut out = Vec::new();
        out.extend_from_slice(b"Q42LEX\0\0");
        out.extend_from_slice(&entry_count.to_le_bytes());
        out.extend_from_slice(&strings_offset.to_le_bytes());
        out.extend_from_slice(&1u64.to_le_bytes());
        out.extend_from_slice(&index);
        out.extend_from_slice(&blob);
        out
    }

    #[test]
    fn sieve_mask_allows_linear_scan() {
        let mut m = SieveStateMask::EMPTY;
        m.push(42, q_hash("a"));
        m.push(99, q_hash("b"));
        assert!(m.allows(42));
        assert!(!m.allows(1));
        assert_eq!(m.lexicon_hash_for(99), Some(q_hash("b")));
    }

    #[test]
    fn sieve_fsm_transitions_to_complete() {
        let mut s = NeuroSymbolicSieve::empty_fsm();
        s.masks[0].push(10, q_hash("sub"));
        s.masks[1].push(20, q_hash("pred"));
        s.masks[2].push(30, q_hash("obj"));
        assert!(s.apply_token(10).is_ok());
        assert!(s.apply_token(20).is_ok());
        assert!(s.apply_token(30).is_ok());
        assert!(s.is_complete());
        let q = s.assemble_quin(q_hash("ctx"));
        assert_eq!(q.subject, q_hash("sub"));
        assert_eq!(q.predicate, q_hash("pred"));
        assert_eq!(q.object, q_hash("obj"));
    }

    #[test]
    fn sieve_builds_masks_from_mmap_lex() {
        let h_sub = q_hash("Patient");
        let h_pred = q_hash("fever");
        let h_obj = q_hash("True");
        let bytes = write_lex_bytes(&[(h_sub, "Patient"), (h_pred, "fever"), (h_obj, "True")]);
        let lex = crate::q42_lex::Q42LexMmap::from_bytes(&bytes).unwrap();
        let tok = crate::gguf_sharder::GgufTokenizer::default();
        let spec = SieveLexSpec::fever_observation();
        let sieve = NeuroSymbolicSieve::from_lex_and_tokenizer(&lex, &tok, &spec);
        assert!(sieve.masks_ready());
    }
}
