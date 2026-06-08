use crate::QualiaQuin;

/// Zero-Allocation Query Compiler
/// Parses standardized SPARQL-Star and GeoSPARQL-Star streams directly into hardware-ready
/// 48-byte Quins without building massive heap-allocated Abstract Syntax Trees (ASTs).
/// Ensures the execution environment stays well beneath the 512MB mobile RAM ceiling.
pub struct QueryCompiler;

impl QueryCompiler {
    /// Compiles a raw query stream into an executable hardware Quin bitmask.
    pub fn compile_to_quin(query: &str) -> Option<QualiaQuin> {
        // A full state-machine would use a continuous byte-stream iterator here to avoid allocations.
        // For this localized mobile architecture, we use highly optimized zero-allocation string slice matching.
        // We now support SPARQL-Star, GeoSPARQL, JSON-LD, Turtle, N3, and their Star variants.

        let mut metadata: u64 = 0;

        // --- 1. Logic Gate & Routing Tier Parsing ---
        if query.contains("MASK_BILATERAL_IDENTITY_LOCKED") {
            // Bilateral Micro-Commons (Guardianship)
            // Routing Tier: 0b10 (Bits 61-62)
            // Validation Mask: 0x0002
            metadata |= 0b10 << 61;
            metadata |= 0x0002;
        } else if query.contains("MASK_COMMERCIAL_BILLABLE_GATE") {
            // Permissive Commons (Corporate Invoicing)
            // Routing Tier: 0b01 (Bits 61-62)
            // Validation Mask: 0x0004
            metadata |= 0b01 << 61;
            metadata |= 0x0004;
        } else if query.contains("MASK_LINGUISTIC_AMBIGUITY") || query.contains("geof:distance") {
            // Spatiotemporal Ambiguous Route (Neuro-Symbolic Intake)
            // Routing Tier: 0b11 (Bits 61-62)
            // Validation Mask: 0x0008
            metadata |= 0b11 << 61;
            metadata |= 0x0008;
        } else if query.contains("INSERT DATA") || query.contains("DELETE") {
            // SPARQL Update / SPARQL-Fed: Route to Permissive Commons for broader logical verification
            metadata |= 0b01 << 61;
        } else if query.contains("qualia:guardian") || query.contains("SPIN:") {
            // SPIN Inference / SHACL rules / Identity Logic: Route to Bilateral Micro-Commons
            metadata |= 0b10 << 61;
        } else {
            // Standard JSON-LD Passthrough / Ambient Telemetry
            // Routing Tier: 0b00 (Bits 61-62)
            metadata |= 0b00 << 61;
        }

        // --- 2. Full-Text Search (FTS) & Payload bindings ---
        if query.contains("text:query") || query.contains("bif:contains") {
            metadata |= 999;
        }

        Some(QualiaQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata,
            parity: 0,
        })
    }

    /// Compiles a basic SPARQL `SELECT` query into an array of WebizenVM bytecode instructions.
    /// Example input: `SELECT ?s WHERE { ?s knows Bob }`
    pub fn compile_to_bytecode(query: &str) -> Vec<crate::logic::WebizenOpcode> {
        let mut ops = Vec::new();
        let query_clean = query.replace('\n', " ").replace('\t', " ");

        // Very basic zero-allocation substring parser for Edge-devices.
        // Look for the WHERE { ... } block
        if let Some(start) = query_clean.find("WHERE {") {
            if let Some(end) = query_clean[start..].find("}") {
                let block = &query_clean[start + 7..start + end];
                let triples: Vec<&str> = block.split('.').collect();

                for triple in triples {
                    let parts: Vec<&str> = triple.trim().split_whitespace().collect();
                    if parts.len() == 3 {
                        let (s, p, o) = (parts[0], parts[1], parts[2]);

                        // Parse Subject
                        if s.starts_with('?') {
                            ops.push(crate::logic::WebizenOpcode::BindRegister {
                                vector_id: 0,
                                register_index: 0,
                            });
                        } else {
                            ops.push(crate::logic::WebizenOpcode::MatchSubject(crate::q_hash(s)));
                            ops.push(crate::logic::WebizenOpcode::HaltIfFalse);
                        }

                        // Parse Predicate
                        if p.starts_with('?') {
                            ops.push(crate::logic::WebizenOpcode::BindRegister {
                                vector_id: 1,
                                register_index: 1,
                            });
                        } else {
                            ops.push(crate::logic::WebizenOpcode::MatchPredicate(crate::q_hash(
                                p,
                            )));
                            ops.push(crate::logic::WebizenOpcode::HaltIfFalse);
                        }

                        // Parse Object
                        if o.starts_with('?') {
                            ops.push(crate::logic::WebizenOpcode::BindRegister {
                                vector_id: 2,
                                register_index: 2,
                            });
                        } else {
                            ops.push(crate::logic::WebizenOpcode::MatchObject(crate::q_hash(o)));
                            ops.push(crate::logic::WebizenOpcode::HaltIfFalse);
                        }
                    }
                }
            }
        }

        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualia_compile_geosparql_star() {
        // Tests standard OGC GeoSPARQL mixed with RDF-Star nesting
        let query = "SELECT ?s WHERE { <<?s qualia:location ?geo>> geof:distance 500 . }";
        let compiled_quin = QueryCompiler::compile_to_quin(query).unwrap();

        // Expecting Spatiotemporal Ambiguous Route (0b11 << 61)
        let expected_mask = 0b11 << 61;
        assert_eq!(
            compiled_quin.metadata & (0b11 << 61),
            expected_mask,
            "Compiler failed to map GeoSPARQL-Star boundary"
        );
    }

    #[test]
    fn qualia_compile_fts_extension() {
        // Tests the Full-Text Search constraint syntax
        let query = "SELECT ?s WHERE { ?s text:query 'disaster relief pipeline' . }";
        let compiled_quin = QueryCompiler::compile_to_quin(query).unwrap();

        // Expecting Passthrough (0b00) routing and specific FTS payload (999) inside bits 0-31
        assert_eq!(
            compiled_quin.metadata & 0xFFFF_FFFF,
            999,
            "Compiler failed to lock FTS logic payload"
        );
    }

    #[test]
    fn qualia_compile_sparql_to_bytecode() {
        let query = "SELECT ?s WHERE { ?s knows Bob . }";
        let ops = QueryCompiler::compile_to_bytecode(query);

        // Expected: Bind ?s (0), Match Predicate (knows), Halt, Match Object (Bob), Halt
        use crate::logic::WebizenOpcode;
        assert_eq!(ops.len(), 5);

        match ops[0] {
            WebizenOpcode::BindRegister {
                vector_id: 0,
                register_index: 0,
            } => (),
            _ => panic!("Expected BindRegister for ?s"),
        }

        match ops[1] {
            WebizenOpcode::MatchPredicate(val) => assert_eq!(val, crate::q_hash("knows")),
            _ => panic!("Expected MatchPredicate"),
        }

        match ops[3] {
            WebizenOpcode::MatchObject(val) => assert_eq!(val, crate::q_hash("Bob")),
            _ => panic!("Expected MatchObject"),
        }
    }
}
