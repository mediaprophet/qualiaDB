//! Whole-chunk admission precedes any host effect, including decoded/manual chunks.
use super::{
    op::{Chunk, Const, Op, MAX_CODE},
    vm::VmError,
};
use crate::{
    ast::{Expr, ExprKind},
    error::{DiagCode, Diagnostic},
    span::Span,
};
use std::collections::HashMap;

fn scoped_path(path: &str) -> bool {
    path.starts_with("task.") || path == "code.spawn_at"
}
pub(super) fn scoped_callee(callee: &Expr, aliases: &HashMap<String, String>) -> bool {
    match &callee.kind {
        ExprKind::Ident(path) => scoped_path(path),
        ExprKind::Member { recv, name } => {
            let ExprKind::Ident(namespace) = &recv.kind else {
                return false;
            };
            let namespace = aliases
                .get(namespace)
                .map(String::as_str)
                .unwrap_or(namespace);
            namespace == "task" || (namespace == "code" && name == "spawn_at")
        }
        _ => false,
    }
}
fn unsupported() -> VmError {
    VmError::Diagnostic(Diagnostic::new(DiagCode::E400, Span::point(0),
        "bytecode profile does not support scoped tasks or dynamic host callees; select the admitted AST evaluator before execution"))
}
fn operand_u16(code: &[u8], pc: usize) -> u16 {
    u16::from_le_bytes([code[pc + 1], code[pc + 2]])
}
fn boundary(bits: &[u64; 1024], target: usize, code_len: usize) -> bool {
    target < code_len && bits[target / 64] & (1u64 << (target % 64)) != 0
}

pub(super) fn validate(chunk: &Chunk) -> Result<(), VmError> {
    if chunk.code.len() > MAX_CODE {
        return Err(VmError::CodeOutOfBounds);
    }
    // 8 KiB caller-stack bitmap, bounded by the existing 64 KiB bytecode profile.
    let mut boundaries = [0u64; 1024];
    let mut pc = 0;
    while pc < chunk.code.len() {
        let opcode = chunk.code[pc];
        let op = Op::from_byte(opcode).ok_or(VmError::InvalidOpcode(opcode))?;
        let end = pc
            .checked_add(1 + op.operand_size())
            .ok_or(VmError::CodeOutOfBounds)?;
        if end > chunk.code.len() {
            return Err(VmError::CodeOutOfBounds);
        }
        boundaries[pc / 64] |= 1u64 << (pc % 64);
        if op == Op::CallHost {
            let index = operand_u16(&chunk.code, pc);
            if index == u16::MAX {
                return Err(unsupported());
            }
            let path = match chunk.constants.get(index as usize) {
                Some(Const::String(path)) => path,
                _ => return Err(VmError::InvalidConstant(index)),
            };
            if scoped_path(path) {
                return Err(unsupported());
            }
        }
        pc = end;
    }
    for function in &chunk.functions {
        if !boundary(&boundaries, function.code_offset as usize, chunk.code.len()) {
            return Err(VmError::CodeOutOfBounds);
        }
    }
    pc = 0;
    while pc < chunk.code.len() {
        let op = Op::from_byte(chunk.code[pc]).ok_or(VmError::InvalidOpcode(chunk.code[pc]))?;
        if matches!(
            op,
            Op::Jump | Op::JumpIfFalse | Op::JumpIfTrue | Op::TryUnwrap
        ) {
            if !boundary(
                &boundaries,
                operand_u16(&chunk.code, pc) as usize,
                chunk.code.len(),
            ) {
                return Err(VmError::CodeOutOfBounds);
            }
        }
        if op == Op::CallUser {
            let index = operand_u16(&chunk.code, pc);
            if index as usize >= chunk.functions.len() {
                return Err(VmError::InvalidFunction(index));
            }
        }
        pc += 1 + op.operand_size();
    }
    Ok(())
}
