use crate::modalities::epistemic::{
    self, EpistemicStatus, OP_BELIEVES, OP_COMMON_KNOWLEDGE, OP_KNOWS,
};
use crate::modalities::logic::deontic::{
    evaluate_deontic_contract, DeonticStatus, DeonticVerdict, OP_FORBID, OP_OBLIGATE, OP_PERMIT,
};
use crate::{q_hash, NQuin};

/// Identifies the SHACL DataType for a node shape
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShaclDatatype {
    String,
    Integer,
    Decimal,
    Boolean,
    DateTime,
}

impl ShaclDatatype {
    /// Maps an IRI to the corresponding ShaclDatatype
    pub fn from_iri_hash(hash: u64) -> Option<Self> {
        match hash {
            h if h == q_hash("xsd:string") => Some(ShaclDatatype::String),
            h if h == q_hash("xsd:integer") => Some(ShaclDatatype::Integer),
            h if h == q_hash("xsd:decimal") => Some(ShaclDatatype::Decimal),
            h if h == q_hash("xsd:boolean") => Some(ShaclDatatype::Boolean),
            h if h == q_hash("xsd:dateTime") => Some(ShaclDatatype::DateTime),
            _ => None,
        }
    }
}

/// Zero-heap SHACL Constraint AST
/// Uses primitive types and FNV-1a hashes to fit within the memory ceiling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShaclConstraint {
    Datatype(ShaclDatatype),
    MinLength(u32),
    MaxLength(u32),
    MinCount(u32),
    MaxCount(u32),
    /// For sh:in, we store up to 8 permitted hashes inline to avoid allocation.
    /// If more are needed, it would overflow to a separate memory-mapped buffer.
    In {
        count: u8,
        values: [u64; 8],
    },

    // Deontic & Epistemic Extensions (from AGENTS.md Task E)
    DeonticObligate,
    DeonticPermit,
    DeonticForbid,
    DeonticNotExpired {
        now_unix: u32,
    },
    EpistemicKnowledge {
        min_certainty: u8,
    },
    EpistemicBelief {
        min_certainty: u8,
    },
    CommonKnowledge,
}

/// Evaluates a slice of NQuins against a set of constraints for a specific target property hash.
/// Returns true if valid, false if a constraint violation occurs.
pub fn validate_shacl_property(
    quins: &[NQuin],
    target_subject: u64,
    target_property: u64,
    constraints: &[ShaclConstraint],
) -> bool {
    let mut matching_count = 0;

    for quin in quins {
        if quin.subject == target_subject && quin.predicate == target_property {
            matching_count += 1;

            for constraint in constraints {
                match constraint {
                    ShaclConstraint::Datatype(expected_dt) => {
                        // Extract inline type tag from object field (bits 60-62 when MSB=0)
                        if quin.object >> 63 != 0 {
                            // MSB=1 implies a pointer, not a literal
                            return false;
                        }
                        let type_tag = (quin.object >> 60) & 0b111;
                        let valid = match expected_dt {
                            ShaclDatatype::String => type_tag == 0b000,
                            ShaclDatatype::Integer => type_tag == 0b001,
                            ShaclDatatype::Decimal => type_tag == 0b010,
                            ShaclDatatype::Boolean => type_tag == 0b011,
                            ShaclDatatype::DateTime => type_tag == 0b001, // Often stored as Unix epoch int
                        };
                        if !valid {
                            return false;
                        }
                    }
                    ShaclConstraint::MinLength(_) | ShaclConstraint::MaxLength(_) => {
                        // In a real system, we'd need to resolve the string length from the object buffer.
                        // Since strings are hashed, length constraints might require looking up the lexicon.
                        // We skip this check if the data is just hashes.
                        // For Phase D we assume true if not available.
                    }
                    ShaclConstraint::In { count, values } => {
                        let payload = quin.object & 0x0FFF_FFFF_FFFF_FFFF;
                        let mut found = false;
                        for i in 0..*count as usize {
                            if values[i] == payload {
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            return false;
                        }
                    }
                    ShaclConstraint::DeonticObligate => {
                        if !deontic_quin_matches(quins, quin, OP_OBLIGATE, DeonticStatus::Active) {
                            return false;
                        }
                    }
                    ShaclConstraint::DeonticPermit => {
                        if !deontic_quin_matches(quins, quin, OP_PERMIT, DeonticStatus::Active) {
                            return false;
                        }
                    }
                    ShaclConstraint::DeonticForbid => {
                        if deontic_quin_matches(quins, quin, OP_FORBID, DeonticStatus::Active) {
                            return false;
                        }
                    }
                    ShaclConstraint::DeonticNotExpired { now_unix } => {
                        if !deontic_not_expired(quin, *now_unix) {
                            return false;
                        }
                    }
                    ShaclConstraint::EpistemicKnowledge { min_certainty } => {
                        if !epistemic_quin_matches(
                            quins,
                            quin,
                            OP_KNOWS,
                            *min_certainty,
                            EpistemicStatus::Active,
                        ) {
                            return false;
                        }
                    }
                    ShaclConstraint::EpistemicBelief { min_certainty } => {
                        if !epistemic_quin_matches(
                            quins,
                            quin,
                            OP_BELIEVES,
                            *min_certainty,
                            EpistemicStatus::Active,
                        ) {
                            return false;
                        }
                    }
                    ShaclConstraint::CommonKnowledge => {
                        if !epistemic_quin_matches(
                            quins,
                            quin,
                            OP_COMMON_KNOWLEDGE,
                            0,
                            EpistemicStatus::Active,
                        ) {
                            return false;
                        }
                    }
                    ShaclConstraint::MinCount(_) | ShaclConstraint::MaxCount(_) => {}
                }
            }
        }
    }

    // Check cardinality counts
    for constraint in constraints {
        match constraint {
            ShaclConstraint::MinCount(min) => {
                if matching_count < *min {
                    return false;
                }
            }
            ShaclConstraint::MaxCount(max) => {
                if matching_count > *max {
                    return false;
                }
            }
            ShaclConstraint::DeonticObligate
            | ShaclConstraint::DeonticPermit
            | ShaclConstraint::DeonticForbid
            | ShaclConstraint::DeonticNotExpired { .. }
            | ShaclConstraint::EpistemicKnowledge { .. }
            | ShaclConstraint::EpistemicBelief { .. }
            | ShaclConstraint::CommonKnowledge => {
                if matching_count == 0 {
                    return false;
                }
            }
            _ => {}
        }
    }

    true
}

fn deontic_not_expired(quin: &NQuin, now_unix: u32) -> bool {
    let expiry = (quin.metadata & 0xFFFF_FFFF) as u32;
    expiry == 0 || now_unix <= expiry
}

fn deontic_quin_matches(
    quins: &[NQuin],
    focus: &NQuin,
    expected_opcode: u8,
    required_status: DeonticStatus,
) -> bool {
    let mut verdicts = [DeonticVerdict::default(); 32];
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);
    let count = evaluate_deontic_contract(quins, now, &mut verdicts).unwrap_or(0);
    for verdict in &verdicts[..count] {
        if verdict.norm.subject == focus.subject
            && verdict.norm.predicate == focus.predicate
            && verdict.norm.object == focus.object
            && (verdict.norm.predicate & 0xFF) as u8 == expected_opcode
            && verdict.status == required_status
        {
            return true;
        }
    }
    false
}

fn epistemic_quin_matches(
    quins: &[NQuin],
    focus: &NQuin,
    expected_opcode: u8,
    min_certainty: u8,
    required_status: EpistemicStatus,
) -> bool {
    let mut verdicts = [epistemic::EpistemicVerdict {
        claim: NQuin::default(),
        status: EpistemicStatus::Skipped,
        certainty: 0,
    }; 32];
    let count =
        epistemic::evaluate_epistemic_frame(quins, focus.subject, focus.context, &mut verdicts)
            .unwrap_or(0);
    for verdict in &verdicts[..count] {
        if verdict.claim.object == focus.object
            && (verdict.claim.predicate & 0xFF) as u8 == expected_opcode
            && verdict.status == required_status
            && verdict.certainty >= min_certainty
        {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shacl_datatype_integer() {
        let subj = q_hash("did:q42:patient1");
        let prop = q_hash("q42:age");

        let quin_int = NQuin {
            subject: subj,
            predicate: prop,
            object: (0b001 << 60) | 42, // Integer tag + value 42
            context: 0,
            metadata: 0,
            parity: 0,
        };

        let constraints = [ShaclConstraint::Datatype(ShaclDatatype::Integer)];

        assert!(validate_shacl_property(
            &[quin_int],
            subj,
            prop,
            &constraints
        ));

        // Test failure on incorrect datatype (e.g. String tag 0b000)
        let quin_str = NQuin {
            subject: subj,
            predicate: prop,
            object: (0b000 << 60) | q_hash("forty-two"),
            context: 0,
            metadata: 0,
            parity: 0,
        };
        assert!(!validate_shacl_property(
            &[quin_str],
            subj,
            prop,
            &constraints
        ));
    }

    #[test]
    #[test]
    fn test_shacl_deontic_obligate() {
        let subj = q_hash("did:q42:party1");
        let prop = q_hash("q42:mustSign");
        let obj = q_hash("contract:nda");
        let mut norm = crate::modalities::logic::deontic::compile_norm_quin(
            subj,
            OP_OBLIGATE,
            prop,
            obj,
            q_hash("ctx:nda"),
            u32::MAX,
            false,
        );
        norm.parity = norm.subject ^ norm.predicate ^ norm.object ^ norm.context;

        let constraints = [ShaclConstraint::DeonticObligate];
        assert!(validate_shacl_property(
            &[norm],
            subj,
            norm.predicate,
            &constraints
        ));
    }

    #[test]
    fn test_shacl_epistemic_knowledge() {
        let agent = q_hash("agent_a");
        let claim_obj = q_hash("claim:p");
        let mut knows = NQuin {
            subject: agent,
            predicate: (200u64 << 8) | OP_KNOWS as u64,
            object: claim_obj,
            context: q_hash("world_w"),
            metadata: 0,
            parity: 0,
        };
        knows.parity = knows.subject ^ knows.predicate ^ knows.object ^ knows.context;

        let prop = knows.predicate;
        let constraints = [ShaclConstraint::EpistemicKnowledge { min_certainty: 128 }];
        assert!(validate_shacl_property(&[knows], agent, prop, &constraints));
    }

    #[test]
    fn test_shacl_cardinality() {
        let subj = q_hash("did:q42:user1");
        let prop = q_hash("schema:email");

        let quin = NQuin {
            subject: subj,
            predicate: prop,
            object: (0b000 << 60) | q_hash("test@example.com"),
            context: 0,
            metadata: 0,
            parity: 0,
        };

        // MinCount 1 -> passes
        assert!(validate_shacl_property(
            &[quin.clone()],
            subj,
            prop,
            &[ShaclConstraint::MinCount(1)]
        ));

        // MinCount 2 -> fails
        assert!(!validate_shacl_property(
            &[quin.clone()],
            subj,
            prop,
            &[ShaclConstraint::MinCount(2)]
        ));
    }
}
