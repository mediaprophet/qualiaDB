//! SPARQL Update Operations
//!
//! Implements SPARQL 1.1 Update Language (INSERT DATA, DELETE DATA, DELETE/INSERT WHERE).

use crate::sparql_ast::*;
use crate::NQuin;

/// Update operation types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOperation {
    /// INSERT DATA { triples }
    InsertData {
        quins: [NQuin; 64],
        quin_count: u8,
    },
    /// DELETE DATA { triples }
    DeleteData {
        quins: [NQuin; 64],
        quin_count: u8,
    },
    /// DELETE { pattern } INSERT { pattern } WHERE { pattern }
    DeleteInsert {
        delete_pattern: PatternId,
        insert_pattern: PatternId,
        where_pattern: PatternId,
    },
    /// LOAD <uri> INTO GRAPH <graph>
    Load {
        uri: u64,
        graph: u64,
    },
    /// CLEAR GRAPH <graph>
    Clear {
        graph: u64,
    },
    /// CREATE GRAPH <graph>
    Create {
        graph: u64,
    },
    /// DROP GRAPH <graph>
    Drop {
        graph: u64,
    },
}

/// Update executor
pub struct UpdateExecutor<'a> {
    pub quins: &'a mut Vec<NQuin>,
}

impl<'a> UpdateExecutor<'a> {
    pub fn new(quins: &'a mut Vec<NQuin>) -> Self {
        Self { quins }
    }

    /// Execute an update operation
    pub fn execute(
        &mut self,
        operation: &UpdateOperation,
        ctx: &SparqlQueryContext,
    ) -> Result<u64, String> {
        match operation {
            UpdateOperation::InsertData { quins, quin_count } => {
                self.execute_insert_data(quins, *quin_count)
            }
            UpdateOperation::DeleteData { quins, quin_count } => {
                self.execute_delete_data(quins, *quin_count)
            }
            UpdateOperation::DeleteInsert {
                delete_pattern,
                insert_pattern,
                where_pattern,
            } => self.execute_delete_insert(*delete_pattern, *insert_pattern, *where_pattern, ctx),
            UpdateOperation::Load { uri, graph } => {
                self.execute_load(*uri, *graph)
            }
            UpdateOperation::Clear { graph } => {
                self.execute_clear(*graph)
            }
            UpdateOperation::Create { graph } => {
                self.execute_create(*graph)
            }
            UpdateOperation::Drop { graph } => {
                self.execute_drop(*graph)
            }
        }
    }

    fn execute_insert_data(
        &mut self,
        quins: &[NQuin],
        quin_count: u8,
    ) -> Result<u64, String> {
        let count = quin_count as usize;
        if count > quins.len() {
            return Err("Quin count exceeds array length".to_string());
        }

        for i in 0..count {
            self.quins.push(quins[i]);
        }

        Ok(count as u64)
    }

    fn execute_delete_data(
        &mut self,
        quins: &[NQuin],
        quin_count: u8,
    ) -> Result<u64, String> {
        let count = quin_count as usize;
        if count > quins.len() {
            return Err("Quin count exceeds array length".to_string());
        }

        let mut deleted = 0;
        for i in 0..count {
            let target = quins[i];
            // Remove all matching quins
            self.quins.retain(|quin| {
                quin.subject != target.subject
                    || quin.predicate != target.predicate
                    || quin.object != target.object
            });
            deleted += 1;
        }

        Ok(deleted)
    }

    fn execute_delete_insert(
        &mut self,
        delete_pattern: PatternId,
        insert_pattern: PatternId,
        where_pattern: PatternId,
        ctx: &SparqlQueryContext,
    ) -> Result<u64, String> {
        // Plan WHERE clause
        let mut plan = crate::sparql_library::sparql_planner::ExecutionPlan::new();
        let root_op = crate::sparql_library::sparql_planner::QueryPlanner::plan_pattern(where_pattern, ctx, &mut plan)?;
        plan.root_operator = root_op;

        // Execute WHERE clause
        let bindings = {
            let executor = crate::sparql_library::sparql_executor::QueryExecutor::new(&self.quins);
            executor.execute(&plan, ctx)?
        };

        let mut deleted = 0;
        let mut insert_quins = Vec::new();

        // Helper to evaluate a pattern against a binding row into NQuin
        let evaluate_pattern = |pattern_id: PatternId, ctx: &SparqlQueryContext, row: &BindingRow| -> Option<NQuin> {
            if let Some(pattern) = ctx.patterns.get(pattern_id as usize) {
                match pattern {
                    Pattern::Triple { subject, predicate, object } => {
                        let s = if *subject < 0x8000_0000_0000_0000 { row.get(*subject as u8)? } else { *subject };
                        let p = if *predicate < 0x8000_0000_0000_0000 { row.get(*predicate as u8)? } else { *predicate };
                        let o = if *object < 0x8000_0000_0000_0000 { row.get(*object as u8)? } else { *object };
                        Some(NQuin { subject: s, predicate: p, object: o, context: 0, metadata: 0, parity: 0 })
                    }
                    _ => None
                }
            } else {
                None
            }
        };

        for binding in bindings {
            if let Some(target) = evaluate_pattern(delete_pattern, ctx, &binding) {
                let initial_len = self.quins.len();
                self.quins.retain(|q| q.subject != target.subject || q.predicate != target.predicate || q.object != target.object);
                deleted += initial_len - self.quins.len();
            }
            if let Some(new_quin) = evaluate_pattern(insert_pattern, ctx, &binding) {
                insert_quins.push(new_quin);
            }
        }

        self.quins.extend(insert_quins);
        Ok(deleted as u64)
    }

    fn execute_load(&mut self, uri: u64, graph: u64) -> Result<u64, String> {
        // In production, this would:
        // 1. Resolve URI hash to actual URL
        // 2. Fetch RDF from URL using HTTP client
        // 3. Parse RDF into quins (Turtle, N-Triples, etc.)
        // 4. Set context to graph
        // 5. Insert quins into database
        
        // Simplified: return 0 (requires HTTP client)
        Ok(0)
    }

    fn execute_clear(&mut self, graph: u64) -> Result<u64, String> {
        // Remove all quins with matching graph context
        let original_len = self.quins.len();
        self.quins.retain(|quin| quin.context != graph);
        Ok((original_len - self.quins.len()) as u64)
    }

    fn execute_create(&mut self, graph: u64) -> Result<u64, String> {
        // Create a new named graph (metadata only)
        // In production, this would:
        // 1. Check if graph already exists
        // 2. Create graph metadata entry
        // 3. Set permissions/ACLs
        
        // Simplified: return 1 (success)
        Ok(1)
    }

    fn execute_drop(&mut self, graph: u64) -> Result<u64, String> {
        // Remove all quins with matching graph context and delete graph metadata
        let original_len = self.quins.len();
        self.quins.retain(|quin| quin.context != graph);
        
        // In production, this would also delete graph metadata
        Ok((original_len - self.quins.len()) as u64)
    }
}

/// Update query
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UpdateQuery {
    pub operation: UpdateOperation,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_data() {
        let mut quins = vec![];
        let mut executor = UpdateExecutor::new(&mut quins);
        
        let test_quin = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        
        let mut quins_array = [NQuin::default(); 64];
        quins_array[0] = test_quin;
        
        let result = executor.execute_insert_data(&quins_array, 1).unwrap();
        assert_eq!(result, 1);
        assert_eq!(executor.quins.len(), 1);
    }

    #[test]
    fn test_delete_data() {
        let mut quins = vec![NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: 0,
        }];
        
        let mut executor = UpdateExecutor::new(&mut quins);
        
        let test_quin = NQuin {
            subject: 1,
            predicate: 2,
            object: 3,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        
        let mut quins_array = [NQuin::default(); 64];
        quins_array[0] = test_quin;
        
        let result = executor.execute_delete_data(&quins_array, 1).unwrap();
        assert_eq!(result, 1);
        assert_eq!(executor.quins.len(), 0);
    }
}