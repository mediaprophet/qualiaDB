use std::collections::HashMap;
use spargebra::term::{TermPattern, NamedNodePattern, TriplePattern};
use spargebra::algebra::GraphPattern;

const OP_MATCH_SUBJ: u8 = 0x01;
const OP_MATCH_PRED: u8 = 0x02;
const OP_MATCH_OBJ:  u8 = 0x03;

// OP_BIND_VAR is followed by 2 bytes: [Target Slot (Subj=0, Pred=1, Obj=2), Var Register ID]
const OP_BIND_VAR: u8   = 0x10; 
const SLOT_SUBJ: u8 = 0x00;
const SLOT_PRED: u8 = 0x01;
const SLOT_OBJ: u8  = 0x02;

const OP_HALT: u8       = 0xFF;

struct VariableRegistry {
    name_to_id: std::collections::HashMap<String, u8>,
    next_id: u8,
}

impl VariableRegistry {
    fn new() -> Self { Self { name_to_id: std::collections::HashMap::new(), next_id: 0 } }
    fn get_or_assign(&mut self, name: &str) -> u8 {
        *self.name_to_id.entry(name.to_string()).or_insert_with(|| {
            let id = self.next_id;
            self.next_id += 1;
            id
        })
    }
}

pub struct SparqlCompiler {
    // In a real run, this would be loaded from the .lex file
    lexicon: HashMap<String, u64>, 
}

impl SparqlCompiler {
    pub fn new(lexicon: HashMap<String, u64>) -> Self {
        Self { lexicon }
    }

    pub fn compile(&self, sparql_string: &str) -> Result<Vec<u8>, String> {
        let query = spargebra::Query::parse(sparql_string, None).map_err(|e| e.to_string())?;
        let mut bytecode = Vec::new();
        let mut var_registry = VariableRegistry::new();

        // Extract the base GraphPattern (ignoring SELECT projections for the MVP)
        let spargebra::Query::Select { pattern, .. } = query else {
            return Err("Only SELECT queries are supported in the MVP".to_string());
        };

        if let GraphPattern::Bgp { patterns } = pattern {
            for triple in patterns {
                self.compile_triple_pattern(&triple, &mut bytecode, &mut var_registry)?;
            }
        } else {
            return Err("Only Basic Graph Patterns (BGPs) are currently supported".to_string());
        }

        bytecode.push(OP_HALT);
        Ok(bytecode)
    }

    fn compile_triple_pattern(
        &self, 
        triple: &TriplePattern, 
        bytecode: &mut Vec<u8>, 
        vars: &mut VariableRegistry
    ) -> Result<(), String> {
        
        // 1. Compile Subject
        match &triple.subject {
            TermPattern::NamedNode(node) => {
                let hash = self.lexicon.get(node.as_str()).unwrap_or(&0); // Mock fallback
                bytecode.push(OP_MATCH_SUBJ);
                bytecode.extend_from_slice(&hash.to_le_bytes());
            },
            TermPattern::Variable(var) => {
                let var_id = vars.get_or_assign(var.as_str());
                bytecode.extend_from_slice(&[OP_BIND_VAR, SLOT_SUBJ, var_id]);
            },
            _ => return Err("Unsupported Subject Type".into()),
        }

        // 2. Compile Predicate
        match &triple.predicate {
            NamedNodePattern::NamedNode(node) => {
                let hash = self.lexicon.get(node.as_str()).unwrap_or(&0);
                bytecode.push(OP_MATCH_PRED);
                bytecode.extend_from_slice(&hash.to_le_bytes());
            },
            NamedNodePattern::Variable(var) => {
                let var_id = vars.get_or_assign(var.as_str());
                bytecode.extend_from_slice(&[OP_BIND_VAR, SLOT_PRED, var_id]);
            }
        }

        // 3. Compile Object (Includes Inline Datatype packing)
        match &triple.object {
            TermPattern::NamedNode(node) => {
                let hash = self.lexicon.get(node.as_str()).unwrap_or(&0);
                bytecode.push(OP_MATCH_OBJ);
                bytecode.extend_from_slice(&hash.to_le_bytes());
            },
            TermPattern::Literal(literal) => {
                // For MVP, parse as string hash. 
                // TODO: If literal.datatype() is xsd:integer, pack into Inline u64 here.
                let hash = self.lexicon.get(literal.value()).unwrap_or(&0);
                bytecode.push(OP_MATCH_OBJ);
                bytecode.extend_from_slice(&hash.to_le_bytes());
            }
            TermPattern::Variable(var) => {
                let var_id = vars.get_or_assign(var.as_str());
                bytecode.extend_from_slice(&[OP_BIND_VAR, SLOT_OBJ, var_id]);
            },
            _ => return Err("Unsupported Object Type".into()),
        }

        Ok(())
    }
}
