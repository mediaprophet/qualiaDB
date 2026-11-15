//! Zero-allocation OpenQASM 3 Abstract Syntax Tree (AST).
//!
//! Uses compile-time hashing for identifiers to avoid heap allocations.

use qualia_core_db::q_hash;

pub mod parser;
pub use parser::*;

/// OpenQASM 3 Program AST.
#[derive(Debug, Clone)]
pub struct QasmProgram {
    pub statements: [QasmStatement; 256],
    pub statement_count: usize,
}

/// An operation within a composite gate declaration body.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GateBodyOp {
    pub name_hash: u64,
    pub target_indices: [u8; 4],
    pub num_targets: u8,
}

/// A zero-allocation statement.
#[derive(Debug, Clone, Copy)]
pub enum QasmStatement {
    Empty,
    GateDecl {
        name_hash: u64,
        num_qubits: u8,
        body_count: u8,
        body_ops: [GateBodyOp; 16],
    },
    GateCall {
        name_hash: u64,
        target_qubits: [u16; 4],
        num_targets: u8,
        params: [f64; 4],
        num_params: u8,
    },
    QubitDecl {
        name_hash: u64,
        size: u16,
    },
}

impl Default for QasmStatement {
    fn default() -> Self {
        QasmStatement::Empty
    }
}

impl QasmProgram {
    /// Creates a new instance.
    pub fn new() -> Self {
        Self {
            statements: [QasmStatement::Empty; 256],
            statement_count: 0,
        }
    }
}

impl Default for QasmProgram {
    fn default() -> Self {
        Self::new()
    }
}

impl QasmProgram {
    /// Pushes an item.
    pub fn push(&mut self, statement: QasmStatement) -> Result<(), &'static str> {
        if self.statement_count >= 256 {
            return Err("AST capacity exceeded");
        }
        self.statements[self.statement_count] = statement;
        self.statement_count += 1;
        Ok(())
    }

    /// Expand composite gate calls in the program using declared composite gate definitions.
    pub fn expand_composite_gates(&self) -> Result<QasmProgram, &'static str> {
        let mut expanded = QasmProgram::new();
        for i in 0..self.statement_count {
            match self.statements[i] {
                QasmStatement::GateCall {
                    name_hash,
                    target_qubits,
                    num_targets,
                    params: _,
                    num_params: _,
                } => {
                    let mut found_decl = None;
                    for j in 0..self.statement_count {
                        if let QasmStatement::GateDecl {
                            name_hash: decl_hash,
                            num_qubits,
                            body_count,
                            body_ops,
                        } = self.statements[j]
                        {
                            if decl_hash == name_hash && body_count > 0 {
                                found_decl = Some((num_qubits, body_count, body_ops));
                                break;
                            }
                        }
                    }

                    if let Some((_num_qubits, body_count, body_ops)) = found_decl {
                        for k in 0..body_count as usize {
                            let op = body_ops[k];
                            let mut mapped_targets = [0u16; 4];
                            for t in 0..op.num_targets as usize {
                                let formal_idx = op.target_indices[t] as usize;
                                if formal_idx < num_targets as usize {
                                    mapped_targets[t] = target_qubits[formal_idx];
                                }
                            }
                            expanded.push(QasmStatement::GateCall {
                                name_hash: op.name_hash,
                                target_qubits: mapped_targets,
                                num_targets: op.num_targets,
                                params: [0.0; 4],
                                num_params: 0,
                            })?;
                        }
                    } else {
                        expanded.push(self.statements[i])?;
                    }
                }
                other => {
                    expanded.push(other)?;
                }
            }
        }
        Ok(expanded)
    }
}
