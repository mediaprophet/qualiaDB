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

/// A zero-allocation statement.
#[derive(Debug, Clone, Copy)]
pub enum QasmStatement {
    Empty,
    GateDecl {
        name_hash: u64,
        num_qubits: u8,
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
    pub fn push(&mut self, statement: QasmStatement) -> Result<(), &'static str> {
        if self.statement_count >= 256 {
            return Err("AST capacity exceeded");
        }
        self.statements[self.statement_count] = statement;
        self.statement_count += 1;
        Ok(())
    }
}
