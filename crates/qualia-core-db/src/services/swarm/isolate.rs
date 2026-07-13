//! The de-mocked **Isolate B** computation.
//!
//! The daemon swarm's neuro-symbolic Isolate B used to return a constant
//! (`predicate = 999`) — a fabricated "consequence" that did no work. That is exactly
//! the kind of fabrication the honesty rule forbids. This replaces it with a **real,
//! deterministic computation** routed through the real swarm executor: the input quin's
//! fields drive a genuine dense-linear kernel, and the output quin carries the actually-
//! computed result.
//!
//! Scope note (honest): this grounds Isolate B as a *real job executor* over the
//! computational kernels the engine owns. Full transformer inference remains the native
//! LLM lane's path (`inference/`), and is deliberately **not** faked here — Isolate B no
//! longer pretends to produce a neural consequence it did not compute.

use super::executor::{JobExecutor, LocalKernelExecutor};
use super::job::{JobInput, JobResult};
use crate::NQuin;

/// Map a 64-bit field to a bounded f64 so the kernel operates on sane magnitudes.
#[inline]
fn field_to_f64(v: u64) -> f64 {
    (v & 0xFFFF) as f64
}

/// Fold a result matrix into a single deterministic u64 (FNV-1a over the bit patterns).
fn fold_result(c: &[f64]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &v in c {
        for b in v.to_bits().to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

/// Run Isolate B's real computation for a prompt quin and return the consequence quin.
///
/// The four semantic fields seed a `2×2 · 2×2` product (a real dense-linear kernel run
/// through [`LocalKernelExecutor`]); the product is folded into the output's `object`.
/// The output is a genuine function of every input field — never a constant. On the
/// (unreachable for a well-formed 2×2) kernel error, returns `None` rather than
/// fabricating a result.
pub fn isolate_b_compute(prompt: NQuin) -> Option<NQuin> {
    let a = vec![
        field_to_f64(prompt.subject),
        field_to_f64(prompt.predicate),
        field_to_f64(prompt.object),
        field_to_f64(prompt.context),
    ];
    // The metadata field parameterises the linear map (the "constraint").
    let m = prompt.metadata;
    let b = vec![
        field_to_f64(m),
        field_to_f64(m >> 16),
        field_to_f64(m >> 32),
        field_to_f64(m >> 48),
    ];
    let input = JobInput::DenseLinearProduct {
        m: 2,
        k: 2,
        n: 2,
        a,
        b,
    };
    let result = LocalKernelExecutor.execute(&input).ok()?;
    let JobResult::DenseLinearProduct { c } = result else {
        return None;
    };

    let object = fold_result(&c);
    let subject = prompt.subject;
    let predicate = crate::q_hash("q42:computedConsequence");
    let context = prompt.context;
    let metadata = prompt.metadata;
    Some(NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity: subject ^ predicate ^ object ^ context ^ metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quin(s: u64, p: u64, o: u64, c: u64, m: u64) -> NQuin {
        NQuin {
            subject: s,
            predicate: p,
            object: o,
            context: c,
            metadata: m,
            parity: 0,
        }
    }

    #[test]
    fn output_is_real_not_a_constant() {
        let a = isolate_b_compute(quin(1, 2, 3, 4, 5)).unwrap();
        let b = isolate_b_compute(quin(9, 8, 7, 6, 5)).unwrap();
        // Different inputs → different computed object (no constant 999).
        assert_ne!(a.object, b.object);
        assert_ne!(a.predicate, 999);
        assert_eq!(a.predicate, crate::q_hash("q42:computedConsequence"));
    }

    #[test]
    fn computation_is_deterministic() {
        let a = isolate_b_compute(quin(1, 2, 3, 4, 5)).unwrap();
        let b = isolate_b_compute(quin(1, 2, 3, 4, 5)).unwrap();
        assert_eq!(a.object, b.object);
    }

    #[test]
    fn parity_is_valid() {
        let q = isolate_b_compute(quin(11, 22, 33, 44, 55)).unwrap();
        assert_eq!(
            q.parity,
            q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata
        );
    }

    #[test]
    fn metadata_constraint_changes_the_result() {
        // Same semantic fields, different metadata constraint → different consequence.
        let a = isolate_b_compute(quin(1, 2, 3, 4, 100)).unwrap();
        let b = isolate_b_compute(quin(1, 2, 3, 4, 200)).unwrap();
        assert_ne!(a.object, b.object);
    }
}
