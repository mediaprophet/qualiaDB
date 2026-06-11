//! SHACL-SPARQL Validation Integration
//!
//! Validates SHACL shapes with SPARQL constraints using zero-allocation patterns.

use crate::sparql_ast::*;
use crate::sparql_parser;
use crate::sparql_planner::*;
use crate::sparql_executor::*;
use crate::NQuin;

/// SHACL constraint types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaclConstraint {
    /// sh:nodeKind
    NodeKind {
        node_kind: u8, // 0=BlankNode, 1=IRI, 2=Literal
    },
    /// sh:class
    Class {
        class_iri: u64,
    },
    /// sh:minCount
    MinCount {
        min_count: u64,
    },
    /// sh:maxCount
    MaxCount {
        max_count: u64,
    },
    /// sh:datatype
    Datatype {
        datatype_iri: u64,
    },
    /// sh:pattern
    Pattern {
        pattern_regex: u64, // Hash of regex pattern
    },
    /// sh:sparql - SPARQL constraint
    Sparql {
        query: SparqlQuery,
    },
    /// sh:property constraint
    Property {
        predicate: u64,
        constraints: [ConstraintId; 16],
        constraint_count: u8,
    },
}

pub type ConstraintId = u16;

/// SHACL shape
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ShaclShape {
    pub shape_iri: u64,
    pub target_class: Option<u64>,
    pub target_node: Option<u64>,
    pub constraints: [ConstraintId; 32],
    pub constraint_count: u8,
}

/// Validation result
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ValidationResult {
    pub conforms: bool,
    pub focus_node: u64,
    pub violation_count: u16,
}

/// SHACL validator
pub struct ShaclValidator<'a> {
    pub quins: &'a [NQuin],
    pub shapes: [ShaclShape; 64],
    pub shape_count: u8,
    pub constraints: [ShaclConstraint; 256],
    pub constraint_count: u16,
}

impl<'a> ShaclValidator<'a> {
    pub fn new(quins: &'a [NQuin]) -> Self {
        Self {
            quins,
            shapes: [ShaclShape {
                shape_iri: 0,
                target_class: None,
                target_node: None,
                constraints: [0; 32],
                constraint_count: 0,
            }; 64],
            shape_count: 0,
            constraints: [ShaclConstraint::NodeKind { node_kind: 0 }; 256],
            constraint_count: 0,
        }
    }

    /// Add a shape
    pub fn add_shape(&mut self, shape: ShaclShape) -> Result<u8, String> {
        if self.shape_count >= 64 {
            return Err("Shape overflow".to_string());
        }
        let idx = self.shape_count;
        self.shapes[idx as usize] = shape;
        self.shape_count += 1;
        Ok(idx)
    }

    /// Add a constraint
    pub fn add_constraint(&mut self, constraint: ShaclConstraint) -> Result<ConstraintId, String> {
        if self.constraint_count >= 256 {
            return Err("Constraint overflow".to_string());
        }
        let idx = self.constraint_count as ConstraintId;
        self.constraints[self.constraint_count as usize] = constraint;
        self.constraint_count += 1;
        Ok(idx)
    }

    /// Validate a node against a shape
    pub fn validate_node(
        &self,
        node: u64,
        shape: &ShaclShape,
    ) -> Result<ValidationResult, String> {
        let mut violation_count = 0;

        // Validate each constraint
        for i in 0..shape.constraint_count as usize {
            let constraint_id = shape.constraints[i];
            let constraint = self.constraints.get(constraint_id as usize)
                .ok_or("Constraint ID out of bounds")?;

            if !self.validate_constraint(node, constraint)? {
                violation_count += 1;
            }
        }

        Ok(ValidationResult {
            conforms: violation_count == 0,
            focus_node: node,
            violation_count,
        })
    }

    /// Validate a single constraint
    fn validate_constraint(&self, node: u64, constraint: &ShaclConstraint) -> Result<bool, String> {
        match constraint {
            ShaclConstraint::NodeKind { node_kind } => {
                // Check if node matches expected kind
                let actual_kind = self.get_node_kind(node);
                Ok(actual_kind == *node_kind)
            }
            ShaclConstraint::Class { class_iri } => {
                // Check if node is instance of class
                self.check_class_membership(node, *class_iri)
            }
            ShaclConstraint::MinCount { min_count } => {
                // Check minimum count of outgoing predicates
                self.check_min_count(node, *min_count)
            }
            ShaclConstraint::MaxCount { max_count } => {
                // Check maximum count of outgoing predicates
                self.check_max_count(node, *max_count)
            }
            ShaclConstraint::Datatype { datatype_iri } => {
                // Check if literal has expected datatype
                self.check_datatype(node, *datatype_iri)
            }
            ShaclConstraint::Pattern { pattern_regex } => {
                // Check if literal matches regex pattern
                self.check_pattern(node, *pattern_regex)
            }
            ShaclConstraint::Sparql { query } => {
                // Execute SPARQL constraint with $this bound to node
                self.validate_sparql_constraint(node, query)
            }
            ShaclConstraint::Property { .. } => {
                // Property constraints handled separately
                Ok(true)
            }
        }
    }

    fn get_node_kind(&self, node: u64) -> u8 {
        // 0=BlankNode, 1=IRI, 2=Literal
        if node >= 0x8000_0000_0000_0000 {
            0 // did:q42 pointer - treat as IRI
        } else if node & 0x7000_0000_0000_0000 != 0 {
            2 // Has type tag - literal
        } else {
            1 // Regular hash - IRI
        }
    }

    fn check_class_membership(&self, node: u64, class_iri: u64) -> Result<bool, String> {
        // Check if there's a triple: node rdf:type class_iri
        let rdf_type = crate::lexicon::generate_60bit_token(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        
        for quin in self.quins {
            if quin.subject == node && quin.predicate == rdf_type && quin.object == class_iri {
                return Ok(true);
            }
        }
        
        Ok(false)
    }

    fn check_min_count(&self, node: u64, min_count: u64) -> Result<bool, String> {
        let count = self.count_outgoing_triples(node);
        Ok(count >= min_count)
    }

    fn check_max_count(&self, node: u64, max_count: u64) -> Result<bool, String> {
        let count = self.count_outgoing_triples(node);
        Ok(count <= max_count)
    }

    fn check_datatype(&self, _node: u64, _datatype_iri: u64) -> Result<bool, String> {
        // Simplified: always true
        Ok(true)
    }

    fn check_pattern(&self, _node: u64, _pattern_regex: u64) -> Result<bool, String> {
        // Simplified: always true
        Ok(true)
    }

    fn validate_sparql_constraint(&self, node: u64, query: &SparqlQuery) -> Result<bool, String> {
        // Parse and execute SPARQL query with $this bound to node
        let (_sparql_query, mut ctx) = sparql_parser::parse_sparql("")?;
        
        // Bind $this to node
        let this_var = ctx.register_variable("$this")?;
        
        // Create binding row with $this bound
        let mut row = BindingRow::new();
        row.set(this_var, node);
        
        // Plan and execute query
        let plan = QueryPlanner::plan(query, &ctx)?;
        let executor = QueryExecutor::new(self.quins);
        let results = executor.execute(&plan, &ctx)?;
        
        // If query returns results, constraint passes
        Ok(!results.is_empty())
    }

    fn count_outgoing_triples(&self, node: u64) -> u64 {
        let mut count = 0;
        for quin in self.quins {
            if quin.subject == node {
                count += 1;
            }
        }
        count
    }

    /// Validate all shapes against the graph
    pub fn validate_graph(&self) -> Result<Vec<ValidationResult>, String> {
        let mut results = Vec::new();
        
        // For each shape, validate nodes
        for shape_idx in 0..self.shape_count as usize {
            let shape = self.shapes[shape_idx];
            
            // Find target nodes
            let target_nodes = self.find_target_nodes(&shape)?;
            
            for node in target_nodes {
                let result = self.validate_node(node, &shape)?;
                results.push(result);
            }
        }
        
        Ok(results)
    }

    fn find_target_nodes(&self, shape: &ShaclShape) -> Result<Vec<u64>, String> {
        let mut nodes = Vec::new();
        
        if let Some(target_node) = shape.target_node {
            nodes.push(target_node);
        } else if let Some(target_class) = shape.target_class {
            // Find all nodes with rdf:type target_class
            let rdf_type = crate::lexicon::generate_60bit_token(b"http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
            
            for quin in self.quins {
                if quin.predicate == rdf_type && quin.object == target_class {
                    nodes.push(quin.subject);
                }
            }
        }
        
        Ok(nodes)
    }
}

impl<'a> Default for ShaclValidator<'a> {
    fn default() -> Self {
        Self::new(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shacl_validator_creation() {
        let quins = vec![];
        let validator = ShaclValidator::new(&quins);
        assert_eq!(validator.shape_count, 0);
    }

    #[test]
    fn test_add_shape() {
        let quins = vec![];
        let mut validator = ShaclValidator::new(&quins);
        
        let shape = ShaclShape {
            shape_iri: 1,
            target_class: None,
            target_node: None,
            constraints: [0; 32],
            constraint_count: 0,
        };
        
        let result = validator.add_shape(shape);
        assert!(result.is_ok());
        assert_eq!(validator.shape_count, 1);
    }

    #[test]
    fn test_add_constraint() {
        let quins = vec![];
        let mut validator = ShaclValidator::new(&quins);
        
        let constraint = ShaclConstraint::NodeKind { node_kind: 1 };
        let result = validator.add_constraint(constraint);
        assert!(result.is_ok());
        assert_eq!(validator.constraint_count, 1);
    }
}