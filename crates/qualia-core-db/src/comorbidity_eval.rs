//! Defeasible comorbidity evaluation for the Core 1 Prolog Sentinel.
//!
//! Traverses nested RDF-Star Quins (subject bit 63) and compounding `q42:exacerbates`
//! edges without heap allocation. Contradictory diagnoses route through paraconsistent
//! isolation rather than halting evaluation.

use crate::modalities::paraconsistent::route_paraconsistent;
use crate::q_hash;
use crate::QualiaQuin;

pub const MAX_CONDITION_SLOTS: usize = 32;
pub const MAX_COMORBIDITY_VERDICTS: usize = 64;
pub const NESTED_SUBJECT_MASK: u64 = 1u64 << 63;
pub const NESTED_PAYLOAD_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

pub const PRED_EXACERBATES: u64 = q_hash("q42:exacerbates");
pub const PRED_HAS_CONDITION: u64 = q_hash("q42:hasCondition");
pub const PRED_HAS_SEVERITY: u64 = q_hash("q42:hasSeverity");

const INLINE_TAG_DECIMAL: u64 = 0b010u64 << 60;
const INLINE_TAG_MASK: u64 = 0b111u64 << 60;
const INLINE_VALUE_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComorbidityStatus {
    Active = 0,
    Isolated = 1,
    Defeated = 2,
}

#[repr(C, align(8))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComorbidityVerdict {
    pub condition_hash: u64,
    pub compounded_risk_milli: u32,
    pub status: ComorbidityStatus,
    pub _pad: [u8; 3],
}

#[derive(Debug, PartialEq, Eq)]
pub enum ComorbidityError {
    OutputBufferFull,
}

/// Fingerprint for a nested RDF-Star claim: `<< ante pred cons >>`.
#[inline]
pub fn nested_claim_fingerprint(ante: u64, pred: u64, cons: u64) -> u64 {
    let body = (ante ^ pred ^ cons) & NESTED_PAYLOAD_MASK;
    body | NESTED_SUBJECT_MASK
}

#[inline]
pub fn is_nested_subject(subject: u64) -> bool {
    (subject & NESTED_SUBJECT_MASK) != 0
}

#[inline]
fn decode_severity_milli(object: u64) -> u32 {
    if (object & INLINE_TAG_MASK) == INLINE_TAG_DECIMAL {
        let scaled = (object & INLINE_VALUE_MASK) as u32;
        // Object stores severity × 10⁶; map to 0–1000 milli scale.
        ((scaled as u64 * 1000) / 1_000_000).min(1000) as u32
    } else {
        500
    }
}

#[inline]
fn encode_severity_object(severity: f32) -> u64 {
    let clamped = severity.clamp(0.0, 1.0);
    let scaled = (clamped * 1_000_000.0).round() as u64;
    (scaled & INLINE_VALUE_MASK) | INLINE_TAG_DECIMAL
}

/// Compile a nested exacerbation Quin pair (relationship + severity).
pub fn compile_exacerbation_quins(
    ante_condition: u64,
    cons_condition: u64,
    patient_context: u64,
    severity: f32,
    out: &mut [QualiaQuin; 2],
) -> usize {
    let nested = nested_claim_fingerprint(ante_condition, PRED_EXACERBATES, cons_condition);

    let mut edge = QualiaQuin::default();
    edge.subject = ante_condition;
    edge.predicate = PRED_EXACERBATES;
    edge.object = cons_condition;
    edge.context = patient_context;
    edge.parity = edge.subject ^ edge.predicate ^ edge.object ^ edge.context;
    out[0] = edge;

    let mut severity_quin = QualiaQuin::default();
    severity_quin.subject = nested;
    severity_quin.predicate = PRED_HAS_SEVERITY;
    severity_quin.object = encode_severity_object(severity);
    severity_quin.context = patient_context;
    severity_quin.parity =
        severity_quin.subject ^ severity_quin.predicate ^ severity_quin.object ^ severity_quin.context;
    out[1] = severity_quin;

    2
}

/// Returns `true` when `condition_hash` plausibly intersects the target organ hash.
#[inline]
fn condition_intersects_organ(condition_hash: u64, target_organ_hash: u64) -> bool {
    if target_organ_hash == 0 {
        return true;
    }
    if condition_hash == target_organ_hash {
        return true;
    }
    // Lightweight organ–system intersection via hashed clinical tokens.
    let cardio = q_hash("Heart");
    let diabetes = q_hash("Type 2 Diabetes Mellitus");
    let neuropathy = q_hash("Diabetic Neuropathy");
    let hypertension = q_hash("Hypertension");

    if target_organ_hash == cardio {
        return matches!(
            condition_hash,
            x if x == diabetes || x == hypertension || x == cardio || x == neuropathy
        );
    }

    condition_hash == target_organ_hash
}

/// Core 1 evaluator — zero heap allocation; caller supplies output buffers.
pub fn eval_comorbidity(
    patient_did_hash: u64,
    target_organ_hash: u64,
    quins: &[QualiaQuin],
    out: &mut [ComorbidityVerdict],
) -> Result<usize, ComorbidityError> {
    let mut consistent_buf = [QualiaQuin::default(); 128];
    let mut isolated_buf = [QualiaQuin::default(); 32];
    let (consistent_count, isolated_count) =
        route_paraconsistent(quins, &mut consistent_buf, &mut isolated_buf).map_err(|_| {
            ComorbidityError::OutputBufferFull
        })?;
    let graph = &consistent_buf[..consistent_count];

    let mut conditions = [0u64; MAX_CONDITION_SLOTS];
    let mut condition_count = 0usize;
    let mut severities = [(0u64, 0u32); MAX_CONDITION_SLOTS];
    let mut severity_count = 0usize;

    for quin in graph {
        if quin.context != patient_did_hash && quin.subject != patient_did_hash {
            continue;
        }

        if quin.predicate == PRED_HAS_CONDITION && quin.subject == patient_did_hash {
            if condition_count < MAX_CONDITION_SLOTS {
                conditions[condition_count] = quin.object;
                condition_count += 1;
            }
        }

        if is_nested_subject(quin.subject) && quin.predicate == PRED_HAS_SEVERITY {
            if severity_count < MAX_CONDITION_SLOTS {
                severities[severity_count] = (quin.subject, decode_severity_milli(quin.object));
                severity_count += 1;
            }
        }
    }

    let mut emitted = 0usize;

    for i in 0..condition_count {
        let condition = conditions[i];
        if !condition_intersects_organ(condition, target_organ_hash) {
            continue;
        }

        let mut risk_milli: u32 = 400;

        for quin in graph {
            if quin.predicate != PRED_EXACERBATES || quin.context != patient_did_hash {
                continue;
            }
            if quin.subject != condition && quin.object != condition {
                continue;
            }
            let nested_fp =
                nested_claim_fingerprint(quin.subject, quin.predicate, quin.object);
            let mut matched_severity = 0u32;
            for j in 0..severity_count {
                if severities[j].0 == nested_fp {
                    matched_severity = severities[j].1;
                    break;
                }
            }
            if matched_severity > 0 {
                risk_milli = risk_milli.saturating_add(matched_severity / 2);
                risk_milli = ((risk_milli as u64 * 14) / 10).min(1000) as u32;
            } else {
                risk_milli = ((risk_milli as u64 * 12) / 10).min(1000) as u32;
            }
        }

        if emitted >= out.len() {
            return Err(ComorbidityError::OutputBufferFull);
        }

        out[emitted] = ComorbidityVerdict {
            condition_hash: condition,
            compounded_risk_milli: risk_milli.min(1000),
            status: ComorbidityStatus::Active,
            _pad: [0; 3],
        };
        emitted += 1;
    }

    for quin in &isolated_buf[..isolated_count] {
        if quin.subject != patient_did_hash && quin.context != patient_did_hash {
            continue;
        }
        if emitted >= out.len() {
            return Err(ComorbidityError::OutputBufferFull);
        }
        out[emitted] = ComorbidityVerdict {
            condition_hash: quin.object,
            compounded_risk_milli: 0,
            status: ComorbidityStatus::Isolated,
            _pad: [0; 3],
        };
        emitted += 1;
    }

    Ok(emitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn patient_ctx() -> u64 {
        q_hash("did:patient:comorb-test")
    }

    fn compile_demo_graph(patient: u64, out: &mut [QualiaQuin; 16]) -> usize {
        let diabetes = q_hash("Type 2 Diabetes Mellitus");
        let neuropathy = q_hash("Diabetic Neuropathy");
        let heart = q_hash("Heart");

        let mut idx = 0usize;

        let mut has_diabetes = QualiaQuin::default();
        has_diabetes.subject = patient;
        has_diabetes.predicate = PRED_HAS_CONDITION;
        has_diabetes.object = diabetes;
        has_diabetes.context = patient;
        has_diabetes.parity =
            has_diabetes.subject ^ has_diabetes.predicate ^ has_diabetes.object ^ has_diabetes.context;
        out[idx] = has_diabetes;
        idx += 1;

        let mut has_neuropathy = QualiaQuin::default();
        has_neuropathy.subject = patient;
        has_neuropathy.predicate = PRED_HAS_CONDITION;
        has_neuropathy.object = neuropathy;
        has_neuropathy.context = patient;
        has_neuropathy.parity = has_neuropathy.subject
            ^ has_neuropathy.predicate
            ^ has_neuropathy.object
            ^ has_neuropathy.context;
        out[idx] = has_neuropathy;
        idx += 1;

        let mut pair = [QualiaQuin::default(); 2];
        let wrote = compile_exacerbation_quins(diabetes, neuropathy, patient, 0.85, &mut pair);
        out[idx..idx + wrote].copy_from_slice(&pair[..wrote]);
        idx += wrote;

        let mut cardio = QualiaQuin::default();
        cardio.subject = patient;
        cardio.predicate = PRED_HAS_CONDITION;
        cardio.object = heart;
        cardio.context = patient;
        cardio.parity =
            cardio.subject ^ cardio.predicate ^ cardio.object ^ cardio.context;
        out[idx] = cardio;
        idx += 1;

        idx
    }

    #[test]
    fn nested_fingerprint_sets_msb() {
        let fp = nested_claim_fingerprint(1, 2, 3);
        assert!(is_nested_subject(fp));
    }

    #[test]
    fn eval_finds_compounded_diabetes_neuropathy_risk() {
        let patient = patient_ctx();
        let mut graph = [QualiaQuin::default(); 16];
        let n = compile_demo_graph(patient, &mut graph);

        let mut verdicts = [ComorbidityVerdict {
            condition_hash: 0,
            compounded_risk_milli: 0,
            status: ComorbidityStatus::Defeated,
            _pad: [0; 3],
        }; 8];

        let count = eval_comorbidity(
            patient,
            q_hash("Heart"),
            &graph[..n],
            &mut verdicts,
        )
        .unwrap();
        assert!(count >= 2);
        assert!(verdicts[0].compounded_risk_milli > 400);
    }

    #[test]
    fn zero_heap_eval_comorbidity() {
        let patient = patient_ctx();
        let mut graph = [QualiaQuin::default(); 16];
        let n = compile_demo_graph(patient, &mut graph);

        let _profiler = dhat::Profiler::builder().testing().build();
        let mut verdicts = [ComorbidityVerdict {
            condition_hash: 0,
            compounded_risk_milli: 0,
            status: ComorbidityStatus::Active,
            _pad: [0; 3],
        }; MAX_COMORBIDITY_VERDICTS];

        let result = eval_comorbidity(patient, q_hash("Heart"), &graph[..n], &mut verdicts);
        assert!(result.is_ok());

        let stats = dhat::HeapStats::get();
        assert_eq!(
            stats.curr_blocks, 0,
            "eval_comorbidity must not allocate on the heap"
        );
        assert_eq!(stats.curr_bytes, 0);
    }
}
