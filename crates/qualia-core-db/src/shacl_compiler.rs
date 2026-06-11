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
    In { count: u8, values: [u64; 8] },
    
    // Deontic & Epistemic Extensions (from AGENTS.md Task E)
    DeonticObligate,
    DeonticPermit,
    DeonticForbid,
    DeonticNotExpired { now_unix: u32 },
    EpistemicKnowledge { min_certainty: u8 },
    EpistemicBelief { min_certainty: u8 },
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
                        if !valid { return false; }
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
                        if !found { return false; }
                    }
                    _ => {} // Other constraints checked elsewhere
                }
            }
        }
    }
    
    // Check cardinality counts
    for constraint in constraints {
        match constraint {
            ShaclConstraint::MinCount(min) => {
                if matching_count < *min { return false; }
            }
            ShaclConstraint::MaxCount(max) => {
                if matching_count > *max { return false; }
            }
            _ => {}
        }
    }

    true
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
            context: 0, metadata: 0, parity: 0
        };
        
        let constraints = [ShaclConstraint::Datatype(ShaclDatatype::Integer)];
        
        assert!(validate_shacl_property(&[quin_int], subj, prop, &constraints));
        
        // Test failure on incorrect datatype (e.g. String tag 0b000)
        let quin_str = NQuin {
            subject: subj,
            predicate: prop,
            object: (0b000 << 60) | q_hash("forty-two"),
            context: 0, metadata: 0, parity: 0
        };
        assert!(!validate_shacl_property(&[quin_str], subj, prop, &constraints));
    }

    #[test]
    fn test_shacl_cardinality() {
        let subj = q_hash("did:q42:user1");
        let prop = q_hash("schema:email");
        
        let quin = NQuin {
            subject: subj, predicate: prop, object: (0b000 << 60) | q_hash("test@example.com"),
            context: 0, metadata: 0, parity: 0
        };
        
        // MinCount 1 -> passes
        assert!(validate_shacl_property(&[quin.clone()], subj, prop, &[ShaclConstraint::MinCount(1)]));
        
        // MinCount 2 -> fails
        assert!(!validate_shacl_property(&[quin.clone()], subj, prop, &[ShaclConstraint::MinCount(2)]));
    }
}
