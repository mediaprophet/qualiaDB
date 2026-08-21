//! AST → bytecode compiler.
//!
//! Compiles a checked `poet_vibe::ast::Program` into a `Chunk`. The
//! compiler walks each function body and top-level statement, emitting
//! stack-based opcodes. Local variables are assigned slot indices; string
//! and IRI literals are interned into the constant pool.
//!
//! ## Compilation model
//!
//! - Each user function gets a `FuncMeta` entry and a code region.
//! - Top-level statements are emitted into a preamble that runs before
//!   any function call.
//! - Control flow uses forward/backward jumps with patching.
//! - `return` emits `Op::Return`; falling off the end of a function emits
//!   `Op::ReturnNull`.

use crate::ast::*;
use crate::bytecode::op::{Chunk, Const, FuncMeta, Op};
use crate::span::Span;
use std::collections::HashMap;

/// Compiler error.
#[derive(Debug, Clone, PartialEq)]
pub enum CompileError {
    /// Too many constants in the pool.
    TooManyConstants,
    /// Too many local variable slots.
    TooManyLocals,
    /// Too many functions.
    TooManyFunctions,
    /// Code segment exceeds 64 KiB.
    CodeTooLarge,
    /// Feature not yet supported in bytecode.
    Unsupported(&'static str),
    /// A `break`/`continue` appeared outside a loop.
    BreakOutsideLoop,
    /// A `continue` appeared outside a loop.
    ContinueOutsideLoop,
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooManyConstants => write!(f, "bytecode: too many constants (>65535)"),
            Self::TooManyLocals => write!(f, "bytecode: too many locals (>65535)"),
            Self::TooManyFunctions => write!(f, "bytecode: too many functions (>65535)"),
            Self::CodeTooLarge => write!(f, "bytecode: code segment exceeds 64 KiB"),
            Self::Unsupported(what) => write!(f, "bytecode: unsupported feature: {what}"),
            Self::BreakOutsideLoop => write!(f, "bytecode: break outside loop"),
            Self::ContinueOutsideLoop => write!(f, "bytecode: continue outside loop"),
        }
    }
}

impl std::error::Error for CompileError {}

/// A pending jump that needs its target patched once the destination offset is known.
struct PendingJump {
    /// Offset of the u16 operand in `code` that needs patching.
    operand_offset: usize,
}

/// Loop context for break/continue patching.
struct LoopCtx {
    /// Pending `JumpIfFalse` operands for `continue` (jump to loop start).
    continues: Vec<PendingJump>,
    /// Pending `Jump` operands for `break` (jump to loop end).
    breaks: Vec<PendingJump>,
}

/// The compiler state.
struct Compiler {
    chunk: Chunk,
    /// Current scope's variable name → slot index (relative to current frame).
    scopes: Vec<HashMap<String, u16>>,
    /// Loop stack for break/continue.
    loops: Vec<LoopCtx>,
    /// Map from function name → index in `chunk.functions`.
    func_indices: HashMap<String, u16>,
    /// Base offset for the current function's locals.
    /// Top-level preamble uses 0; each function resets to 0.
    local_base: u16,
    /// Next local slot for the current frame.
    next_local: u16,
}

impl Compiler {
    fn new() -> Self {
        Self {
            chunk: Chunk::new(),
            scopes: vec![HashMap::new()],
            loops: Vec::new(),
            func_indices: HashMap::new(),
            local_base: 0,
            next_local: 0,
        }
    }

    fn current_scope(&mut self) -> &mut HashMap<String, u16> {
        self.scopes.last_mut().expect("scope stack")
    }

    fn lookup(&self, name: &str) -> Option<u16> {
        for scope in self.scopes.iter().rev() {
            if let Some(&slot) = scope.get(name) {
                return Some(slot);
            }
        }
        None
    }

    fn declare_var(&mut self, name: &str) -> Result<u16, CompileError> {
        let slot = self.next_local;
        if slot as usize >= crate::bytecode::op::MAX_LOCALS {
            return Err(CompileError::TooManyLocals);
        }
        self.current_scope().insert(name.to_string(), slot);
        self.next_local += 1;
        // Track the maximum number of locals needed for the top-level preamble.
        if self.local_base == 0 && slot >= self.chunk.top_locals {
            self.chunk.top_locals = slot + 1;
        }
        Ok(slot)
    }

    fn add_string(&mut self, s: &str) -> Result<u16, CompileError> {
        if self.chunk.constants.len() >= crate::bytecode::op::MAX_CONSTANTS {
            return Err(CompileError::TooManyConstants);
        }
        Ok(self.chunk.add_string(s))
    }

    fn add_iri(&mut self, s: &str) -> Result<u16, CompileError> {
        if self.chunk.constants.len() >= crate::bytecode::op::MAX_CONSTANTS {
            return Err(CompileError::TooManyConstants);
        }
        Ok(self.chunk.add_iri(s))
    }

    fn check_code_size(&self) -> Result<(), CompileError> {
        if self.chunk.code.len() >= crate::bytecode::op::MAX_CODE {
            Err(CompileError::CodeTooLarge)
        } else {
            Ok(())
        }
    }

    fn emit_op(&mut self, op: Op) -> Result<(), CompileError> {
        self.chunk.emit_op(op);
        self.check_code_size()
    }

    fn emit_u16(&mut self, v: u16) -> Result<(), CompileError> {
        self.chunk.emit_u16(v);
        self.check_code_size()
    }

    fn emit_u8(&mut self, v: u8) -> Result<(), CompileError> {
        self.chunk.emit_u8(v);
        self.check_code_size()
    }

    fn emit_i64(&mut self, v: i64) -> Result<(), CompileError> {
        self.chunk.emit_i64(v);
        self.check_code_size()
    }

    fn emit_u64(&mut self, v: u64) -> Result<(), CompileError> {
        self.chunk.emit_u64(v);
        self.check_code_size()
    }

    fn emit_f64(&mut self, v: f64) -> Result<(), CompileError> {
        self.chunk.emit_f64(v);
        self.check_code_size()
    }

    /// Reserve a u16 operand placeholder, returning the offset for later patching.
    fn emit_placeholder(&mut self) -> Result<usize, CompileError> {
        let off = self.chunk.code.len();
        self.chunk.emit_u16(0);
        self.check_code_size()?;
        Ok(off)
    }

    fn patch(&mut self, operand_offset: usize, target: u16) {
        self.chunk.patch_u16(operand_offset, target);
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    // ── Expression compilation ───────────────────────────────────

    fn compile_expr(&mut self, e: &Expr) -> Result<(), CompileError> {
        match &e.kind {
            ExprKind::Literal(lit) => self.compile_literal(lit),
            ExprKind::Ident(name) => {
                if let Some(slot) = self.lookup(name) {
                    self.emit_op(Op::LoadVar)?;
                    self.emit_u16(slot)?;
                } else {
                    // Unresolved identifier — emit as a string for host dispatch
                    // to resolve (e.g. module function names like `math.sqrt`).
                    let idx = self.add_string(name)?;
                    self.emit_op(Op::PushString)?;
                    self.emit_u16(idx)?;
                }
                Ok(())
            }
            ExprKind::QueryVar(name) => {
                let idx = self.add_string(&format!("?{name}"))?;
                self.emit_op(Op::PushString)?;
                self.emit_u16(idx)
            }
            ExprKind::Iri(iri) => {
                let idx = self.add_iri(iri)?;
                self.emit_op(Op::PushIri)?;
                self.emit_u16(idx)
            }
            ExprKind::Prefixed(prefix, local) => {
                let idx = self.add_iri(&format!("{prefix}:{local}"))?;
                self.emit_op(Op::PushIri)?;
                self.emit_u16(idx)
            }
            ExprKind::Blank(label) => {
                let idx = self.add_string(&format!("_:{label}"))?;
                self.emit_op(Op::PushString)?;
                self.emit_u16(idx)
            }
            ExprKind::Binary { op, left, right } => self.compile_binary(op, left, right),
            ExprKind::Unary { op, expr } => self.compile_unary(op, expr),
            ExprKind::Await(inner) => {
                // Await in 0.1 bytecode: evaluate the inner expression.
                // True async scheduling is handled by the host.
                self.compile_expr(inner)
            }
            ExprKind::Member { recv, name } => {
                self.compile_expr(recv)?;
                let idx = self.add_string(name)?;
                self.emit_op(Op::GetMember)?;
                self.emit_u16(idx)
            }
            ExprKind::Call { callee, args } => self.compile_call(callee, args),
            ExprKind::Index { recv, index } => {
                self.compile_expr(recv)?;
                self.compile_expr(index)?;
                self.emit_op(Op::GetIndex)
            }
            ExprKind::Try(inner) => {
                // Try: compile inner, then TryUnwrap with a catch that pushes an error.
                self.compile_expr(inner)?;
                // For 0.1, Try just passes through — the VM's error handling
                // catches diagnostics. We emit TryUnwrap with a no-op catch.
                self.emit_op(Op::TryUnwrap)?;
                let catch_off = self.emit_placeholder()?;
                // Patch catch to point to the instruction after.
                let after = self.chunk.current_offset();
                self.patch(catch_off, after);
                Ok(())
            }
            ExprKind::List(elements) => {
                for el in elements {
                    self.compile_expr(el)?;
                }
                self.emit_op(Op::MakeList)?;
                self.emit_u16(elements.len() as u16)
            }
            ExprKind::Record(fields) => {
                for na in fields {
                    let key_idx = self.add_string(&na.name)?;
                    self.emit_op(Op::PushString)?;
                    self.emit_u16(key_idx)?;
                    self.compile_expr(&na.value)?;
                }
                self.emit_op(Op::MakeRecord)?;
                self.emit_u16(fields.len() as u16)
            }
            ExprKind::Triple { .. } | ExprKind::Reified { .. } => Err(CompileError::Unsupported(
                "triple/reified terms in bytecode",
            )),
        }
    }

    fn compile_literal(&mut self, lit: &Literal) -> Result<(), CompileError> {
        match lit {
            Literal::Null => self.emit_op(Op::PushNull),
            Literal::Bool(b) => {
                self.emit_op(Op::PushBool)?;
                self.emit_u8(if *b { 1 } else { 0 })
            }
            Literal::Int(v) => {
                self.emit_op(Op::PushInt)?;
                self.emit_i64(*v)
            }
            Literal::UInt(v) => {
                self.emit_op(Op::PushUInt)?;
                self.emit_u64(*v)
            }
            Literal::Float(bits) => {
                self.emit_op(Op::PushFloat)?;
                self.emit_u64(*bits)
            }
            Literal::String(s) => {
                let idx = self.add_string(s)?;
                self.emit_op(Op::PushString)?;
                self.emit_u16(idx)
            }
        }
    }

    fn compile_binary(
        &mut self,
        op: &BinOp,
        left: &Expr,
        right: &Expr,
    ) -> Result<(), CompileError> {
        match op {
            BinOp::And => {
                // Short-circuit: if left is false, push false and skip right.
                self.compile_expr(left)?;
                self.emit_op(Op::Dup)?;
                self.emit_op(Op::JumpIfFalse)?;
                let skip = self.emit_placeholder()?;
                // If we didn't jump, pop the duplicated false, eval right.
                self.emit_op(Op::Pop)?;
                self.compile_expr(right)?;
                let after = self.chunk.current_offset();
                self.patch(skip, after);
                Ok(())
            }
            BinOp::Or => {
                // Short-circuit: if left is true, push true and skip right.
                self.compile_expr(left)?;
                self.emit_op(Op::Dup)?;
                self.emit_op(Op::JumpIfTrue)?;
                let skip = self.emit_placeholder()?;
                self.emit_op(Op::Pop)?;
                self.compile_expr(right)?;
                let after = self.chunk.current_offset();
                self.patch(skip, after);
                Ok(())
            }
            _ => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                let bc_op = match op {
                    BinOp::Add => Op::Add,
                    BinOp::Sub => Op::Sub,
                    BinOp::Mul => Op::Mul,
                    BinOp::Div => Op::Div,
                    BinOp::Rem => Op::Rem,
                    BinOp::Eq => Op::Eq,
                    BinOp::Ne => Op::Ne,
                    BinOp::Lt => Op::Lt,
                    BinOp::Le => Op::Le,
                    BinOp::Gt => Op::Gt,
                    BinOp::Ge => Op::Ge,
                    BinOp::And | BinOp::Or => unreachable!(),
                };
                self.emit_op(bc_op)
            }
        }
    }

    fn compile_unary(&mut self, op: &UnOp, expr: &Expr) -> Result<(), CompileError> {
        self.compile_expr(expr)?;
        match op {
            UnOp::Neg => self.emit_op(Op::Neg),
            UnOp::Not => self.emit_op(Op::Not),
            UnOp::Plus => Ok(()), // no-op
        }
    }

    fn compile_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<(), CompileError> {
        // Determine if this is a user function call or a host call.
        let callee_name = callee.ident_name();

        // Compile arguments onto the stack first.
        let arg_count = args.len();
        if arg_count > 255 {
            return Err(CompileError::Unsupported("more than 255 call arguments"));
        }

        // Check if it's a user-defined function.
        if let Some(name) = callee_name {
            if let Some(&fn_idx) = self.func_indices.get(name) {
                for arg in args {
                    self.compile_arg(arg)?;
                }
                self.emit_op(Op::CallUser)?;
                self.emit_u16(fn_idx)?;
                self.emit_u8(arg_count as u8)?;
                return Ok(());
            }
        }

        // Otherwise, treat as a host call. The callee path is a string constant.
        // For member expressions like `math.sqrt`, the path is the full dotted name.
        let path = callee_name.unwrap_or_else(|| {
            // For non-ident callees (e.g. member expressions), we'd need to
            // compile the receiver. For 0.1, we handle the common case.
            ""
        });
        if path.is_empty() {
            // Complex callee — compile it and use a generic host call.
            // This handles `obj.method(args)` patterns.
            self.compile_expr(callee)?;
            for arg in args {
                self.compile_arg(arg)?;
            }
            // Use CallHost with a special path index 0xFFFF meaning "callee on stack".
            self.emit_op(Op::CallHost)?;
            self.emit_u16(0xFFFF)?;
            self.emit_u8(arg_count as u8)?;
            return Ok(());
        }

        let path_idx = self.add_string(path)?;
        for arg in args {
            self.compile_arg(arg)?;
        }
        self.emit_op(Op::CallHost)?;
        self.emit_u16(path_idx)?;
        self.emit_u8(arg_count as u8)
    }

    fn compile_arg(&mut self, arg: &Arg) -> Result<(), CompileError> {
        match arg {
            Arg::Pos(e) => self.compile_expr(e),
            Arg::Named(na) => {
                // Named arg: push key string then value.
                let key_idx = self.add_string(&na.name)?;
                self.emit_op(Op::PushString)?;
                self.emit_u16(key_idx)?;
                self.compile_expr(&na.value)
            }
        }
    }

    // ── Statement compilation ────────────────────────────────────

    fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), CompileError> {
        match stmt {
            Stmt::Let { name, value, .. } => {
                if let Some(v) = value {
                    self.compile_expr(v)?;
                } else {
                    self.emit_op(Op::PushNull)?;
                }
                let slot = self.declare_var(name)?;
                self.emit_op(Op::StoreVar)?;
                self.emit_u16(slot)
            }
            Stmt::Assign { target, value, .. } => {
                // Only simple variable assignment supported in 0.1 bytecode.
                if let Some(name) = target.ident_name() {
                    if let Some(slot) = self.lookup(name) {
                        self.compile_expr(value)?;
                        self.emit_op(Op::StoreVar)?;
                        self.emit_u16(slot)?;
                        Ok(())
                    } else {
                        Err(CompileError::Unsupported(
                            "assignment to undeclared variable",
                        ))
                    }
                } else {
                    Err(CompileError::Unsupported("compound assignment target"))
                }
            }
            Stmt::If {
                cond,
                then_block,
                else_block,
                ..
            } => {
                self.compile_expr(cond)?;
                self.emit_op(Op::JumpIfFalse)?;
                let else_jump = self.emit_placeholder()?;
                self.compile_block(then_block)?;
                if let Some(else_stmt) = else_block {
                    // Jump over the else branch.
                    self.emit_op(Op::Jump)?;
                    let end_jump = self.emit_placeholder()?;
                    // Patch else_jump to here.
                    let else_start = self.chunk.current_offset();
                    self.patch(else_jump, else_start);
                    self.compile_stmt(else_stmt)?;
                    let end = self.chunk.current_offset();
                    self.patch(end_jump, end);
                } else {
                    let end = self.chunk.current_offset();
                    self.patch(else_jump, end);
                }
                Ok(())
            }
            Stmt::For {
                name, iter, body, ..
            } => {
                // For loop: store iter in a local, use index counter, loop
                // fetching elements until GetIndex returns Null.
                self.push_scope();
                let list_slot = self.declare_var("__for_list")?;
                let idx_slot = self.declare_var("__for_idx")?;
                let elem_slot = self.declare_var(name)?;

                // Store the iterator list.
                self.compile_expr(iter)?;
                self.emit_op(Op::StoreVar)?;
                self.emit_u16(list_slot)?;

                // Initialize index to 0.
                self.emit_op(Op::PushInt)?;
                self.emit_i64(0)?;
                self.emit_op(Op::StoreVar)?;
                self.emit_u16(idx_slot)?;

                let loop_start = self.chunk.current_offset();

                // Load list[idx] — VM returns Null for out-of-bounds.
                self.emit_op(Op::LoadVar)?;
                self.emit_u16(list_slot)?;
                self.emit_op(Op::LoadVar)?;
                self.emit_u16(idx_slot)?;
                self.emit_op(Op::GetIndex)?;

                // Check for Null (end of list).
                self.emit_op(Op::Dup)?;
                self.emit_op(Op::PushNull)?;
                self.emit_op(Op::Eq)?;
                self.emit_op(Op::JumpIfTrue)?;
                let end_jump = self.emit_placeholder()?;

                // Not null — store element to loop variable.
                self.emit_op(Op::StoreVar)?;
                self.emit_u16(elem_slot)?;

                // Compile body.
                self.loops.push(LoopCtx {
                    continues: Vec::new(),
                    breaks: Vec::new(),
                });
                self.compile_block(body)?;
                let loop_ctx = self.loops.pop().unwrap();

                // Increment index.
                self.emit_op(Op::LoadVar)?;
                self.emit_u16(idx_slot)?;
                self.emit_op(Op::PushInt)?;
                self.emit_i64(1)?;
                self.emit_op(Op::Add)?;
                self.emit_op(Op::StoreVar)?;
                self.emit_u16(idx_slot)?;

                // Patch continues to jump here (before the back-jump).
                let continue_target = self.chunk.current_offset();
                for pj in &loop_ctx.continues {
                    self.patch(pj.operand_offset, continue_target);
                }

                // Jump back to loop start.
                self.emit_op(Op::Jump)?;
                self.emit_u16(loop_start)?;

                // Loop end — patch end_jump and breaks.
                let loop_end = self.chunk.current_offset();
                self.patch(end_jump, loop_end);
                for pj in &loop_ctx.breaks {
                    self.patch(pj.operand_offset, loop_end);
                }

                // Pop the Null left on the stack from the end check.
                self.emit_op(Op::Pop)?;

                self.pop_scope();
                Ok(())
            }
            Stmt::While { cond, body, .. } => {
                let loop_start = self.chunk.current_offset();
                self.compile_expr(cond)?;
                self.emit_op(Op::JumpIfFalse)?;
                let end_jump = self.emit_placeholder()?;

                self.loops.push(LoopCtx {
                    continues: Vec::new(),
                    breaks: Vec::new(),
                });
                self.compile_block(body)?;
                let loop_ctx = self.loops.pop().unwrap();

                // Patch continues to loop start.
                for pj in &loop_ctx.continues {
                    self.patch(pj.operand_offset, loop_start);
                }

                // Jump back to condition.
                self.emit_op(Op::Jump)?;
                self.emit_u16(loop_start)?;

                let loop_end = self.chunk.current_offset();
                self.patch(end_jump, loop_end);
                for pj in &loop_ctx.breaks {
                    self.patch(pj.operand_offset, loop_end);
                }
                Ok(())
            }
            Stmt::Match {
                scrutinee, arms, ..
            } => {
                // Match: compile scrutinee, then a chain of equality checks.
                self.compile_expr(scrutinee)?;
                let mut end_jumps: Vec<usize> = Vec::new();

                for arm in arms {
                    // Duplicate scrutinee for comparison.
                    self.emit_op(Op::Dup)?;
                    self.compile_pattern_check(&arm.pattern)?;
                    self.emit_op(Op::JumpIfFalse)?;
                    let next_jump = self.emit_placeholder()?;

                    // Pattern matched — pop the duplicated scrutinee.
                    self.emit_op(Op::Pop)?;
                    // Bind pattern variables.
                    self.compile_pattern_bind(&arm.pattern)?;
                    // Compile arm body.
                    match &arm.body {
                        ArmBody::Block(b) => self.compile_block(b)?,
                        ArmBody::Expr(e) => self.compile_expr(e)?,
                    }
                    // Jump to end.
                    self.emit_op(Op::Jump)?;
                    let end_jump = self.emit_placeholder()?;
                    end_jumps.push(end_jump);

                    // Patch next_jump to here.
                    let next = self.chunk.current_offset();
                    self.patch(next_jump, next);
                }

                // No arm matched — pop scrutinee, push null.
                self.emit_op(Op::Pop)?;
                self.emit_op(Op::PushNull)?;

                let end = self.chunk.current_offset();
                for ej in &end_jumps {
                    self.patch(*ej, end);
                }
                Ok(())
            }
            Stmt::Return { value, .. } => {
                if let Some(v) = value {
                    self.compile_expr(v)?;
                    self.emit_op(Op::Return)
                } else {
                    self.emit_op(Op::ReturnNull)
                }
            }
            Stmt::Yield { value, .. } => {
                // Yield in 0.1: evaluate and push (treated like return in generators).
                if let Some(v) = value {
                    self.compile_expr(v)?;
                } else {
                    self.emit_op(Op::PushNull)?;
                }
                self.emit_op(Op::Return)
            }
            Stmt::Transaction { body, .. } => {
                // Transaction: compile body as a block. Transaction semantics
                // are handled by the host.
                self.compile_block(body)
            }
            Stmt::Effect { expr, .. } => {
                self.compile_expr(expr)?;
                self.emit_op(Op::Effect)
            }
            Stmt::Expr { expr, .. } => self.compile_expr(expr),
            Stmt::Block(b) => self.compile_block(b),
        }
    }

    /// Compile a pattern check — pushes a boolean indicating whether the
    /// stack-top value matches the pattern. Consumes the duplicated value
    /// on success check (via EQ), leaves the original on the stack.
    fn compile_pattern_check(&mut self, pat: &Pattern) -> Result<(), CompileError> {
        match pat {
            Pattern::Wildcard => {
                // Always matches. Pop the duplicate, push true.
                self.emit_op(Op::Pop)?;
                self.emit_op(Op::PushBool)?;
                self.emit_u8(1)
            }
            Pattern::Ident(_) => {
                // Always matches (binding). Pop duplicate, push true.
                self.emit_op(Op::Pop)?;
                self.emit_op(Op::PushBool)?;
                self.emit_u8(1)
            }
            Pattern::Literal(lit) => {
                self.compile_literal(lit)?;
                self.emit_op(Op::Eq)
            }
            Pattern::None => {
                self.compile_literal(&Literal::Null)?;
                self.emit_op(Op::Eq)
            }
            Pattern::Some(inner) => {
                // For 0.1, treat Some(x) as "not null and matches inner".
                // This is a simplification.
                self.compile_pattern_check(inner)
            }
            Pattern::Ok(inner) => self.compile_pattern_check(inner),
            Pattern::Err(inner) => self.compile_pattern_check(inner),
            Pattern::Variant { .. } => {
                // Variant pattern matching requires runtime type info.
                // For 0.1, we do a simplified check: always match.
                self.emit_op(Op::Pop)?;
                self.emit_op(Op::PushBool)?;
                self.emit_u8(1)
            }
        }
    }

    /// Bind pattern variables to the value on the stack.
    /// The value remains on the stack (consumed by the arm body or popped).
    fn compile_pattern_bind(&mut self, pat: &Pattern) -> Result<(), CompileError> {
        match pat {
            Pattern::Ident(name) => {
                self.emit_op(Op::Dup)?;
                let slot = self.declare_var(name)?;
                self.emit_op(Op::StoreVar)?;
                self.emit_u16(slot)
            }
            Pattern::Wildcard | Pattern::Literal(_) | Pattern::None => Ok(()),
            Pattern::Some(_) | Pattern::Ok(_) | Pattern::Err(_) | Pattern::Variant { .. } => Ok(()),
        }
    }

    fn compile_block(&mut self, block: &Block) -> Result<(), CompileError> {
        self.push_scope();
        for stmt in &block.stmts {
            self.compile_stmt(stmt)?;
        }
        self.pop_scope();
        Ok(())
    }

    // ── Function compilation ─────────────────────────────────────

    fn compile_function(&mut self, fd: &FunctionDecl) -> Result<(), CompileError> {
        let fn_idx = self.chunk.functions.len() as u16;
        if fn_idx as usize >= crate::bytecode::op::MAX_FUNCTIONS {
            return Err(CompileError::TooManyFunctions);
        }

        let code_offset = self.chunk.current_offset();

        // Register the function name → index mapping so calls can resolve.
        self.func_indices.insert(fd.name.clone(), fn_idx);

        // Parse budget steps if present.
        let budget_steps = fd
            .budget
            .iter()
            .find(|a| a.name == "steps")
            .and_then(|a| match &a.value.kind {
                ExprKind::Literal(Literal::UInt(n)) => Some(*n),
                ExprKind::Literal(Literal::Int(n)) if *n >= 0 => Some(*n as u64),
                _ => None,
            })
            .unwrap_or(0);

        // Set up function scope with parameters.
        let saved_next_local = self.next_local;
        self.next_local = 0;
        self.push_scope();
        for p in &fd.params {
            let _ = self.declare_var(&p.name)?;
        }
        let local_count = self.next_local;

        // Compile body.
        self.compile_block(&fd.body)?;

        // If the function doesn't end with an explicit return, emit ReturnNull.
        let last_byte = *self.chunk.code.last().unwrap_or(&0);
        if last_byte != Op::Return as u8 && last_byte != Op::ReturnNull as u8 {
            self.emit_op(Op::ReturnNull)?;
        }

        self.pop_scope();
        self.next_local = saved_next_local;

        // Record function metadata.
        self.chunk.functions.push(FuncMeta {
            name: fd.name.clone(),
            param_count: fd.params.len() as u8,
            local_count,
            code_offset,
            budget_steps,
        });

        Ok(())
    }

    // ── Program compilation ──────────────────────────────────────

    fn compile_program(&mut self, program: &Program) -> Result<&Chunk, CompileError> {
        // First pass: register all function names so calls can resolve
        // even if a function is called before its definition.
        for item in &program.items {
            if let Item::Function(fd) = item {
                let idx = self.chunk.functions.len() as u16;
                self.func_indices.insert(fd.name.clone(), idx);
                // Reserve a FuncMeta slot — we'll fill it in the second pass.
                self.chunk.functions.push(FuncMeta {
                    name: fd.name.clone(),
                    param_count: fd.params.len() as u8,
                    local_count: 0,
                    code_offset: 0, // placeholder
                    budget_steps: 0,
                });
            }
        }

        // Second pass: compile top-level statements and functions.
        // Top-level statements go into the preamble (before function code).
        // Functions are compiled after.

        // First, compile top-level statements (consts, top-level stmts).
        for item in &program.items {
            match item {
                Item::Const(cd) => {
                    self.compile_expr(&cd.value)?;
                    let slot = self.declare_var(&cd.name)?;
                    self.emit_op(Op::StoreVar)?;
                    self.emit_u16(slot)?;
                }
                Item::Statement(stmt) => {
                    self.compile_stmt(stmt)?;
                }
                _ => {}
            }
        }

        // Emit Halt after the preamble.
        self.emit_op(Op::Halt)?;

        // Now compile functions, updating their code_offset.
        let mut func_iter = 0;
        for item in &program.items {
            if let Item::Function(fd) = item {
                let code_offset = self.chunk.current_offset();

                // Reset local counter for this function.
                // Each function has its own local slot space starting at 0.
                let saved_next_local = self.next_local;
                self.next_local = 0;

                // Set up function scope.
                self.push_scope();
                for p in &fd.params {
                    let _ = self.declare_var(&p.name)?;
                }
                let local_count = self.next_local;

                self.compile_block(&fd.body)?;

                let last_byte = *self.chunk.code.last().unwrap_or(&0);
                if last_byte != Op::Return as u8 && last_byte != Op::ReturnNull as u8 {
                    self.emit_op(Op::ReturnNull)?;
                }

                self.pop_scope();

                // Restore the local counter.
                self.next_local = saved_next_local;

                // Update the FuncMeta that was reserved in the first pass.
                let budget_steps = fd
                    .budget
                    .iter()
                    .find(|a| a.name == "steps")
                    .and_then(|a| match &a.value.kind {
                        ExprKind::Literal(Literal::UInt(n)) => Some(*n),
                        ExprKind::Literal(Literal::Int(n)) if *n >= 0 => Some(*n as u64),
                        _ => None,
                    })
                    .unwrap_or(0);

                self.chunk.functions[func_iter].code_offset = code_offset;
                self.chunk.functions[func_iter].local_count = local_count;
                self.chunk.functions[func_iter].budget_steps = budget_steps;

                func_iter += 1;
            }
        }

        Ok(&self.chunk)
    }
}

/// Compile a full `Program` AST into a `Chunk`.
pub fn compile(program: &Program) -> Result<Chunk, CompileError> {
    let mut compiler = Compiler::new();
    let chunk = compiler.compile_program(program)?;
    Ok(chunk.clone())
}

/// Compile a single expression (for cell bodies) into a `Chunk`.
/// The chunk's preamble evaluates the expression and returns it.
pub fn compile_expr(expr: &Expr) -> Result<Chunk, CompileError> {
    let mut compiler = Compiler::new();
    compiler.compile_expr(expr)?;
    compiler.emit_op(Op::Return)?;
    Ok(compiler.chunk)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse_program;

    fn compile_src(src: &str) -> Result<Chunk, CompileError> {
        let prog = parse_program(src).expect("parse should succeed");
        compile(&prog)
    }

    #[test]
    fn compile_simple_arithmetic() {
        // Cell expression: 1 + 2 * 3
        let chunk = compile_expr(&crate::parse::parse_cell("= 1 + 2 * 3").expect("parse"))
            .expect("compile");
        // Should contain: PushInt(1), PushInt(2), PushInt(3), Mul, Add, Return
        assert!(chunk.code.contains(&(Op::PushInt as u8)));
        assert!(chunk.code.contains(&(Op::Mul as u8)));
        assert!(chunk.code.contains(&(Op::Add as u8)));
        assert_eq!(*chunk.code.last().unwrap(), Op::Return as u8);
    }

    #[test]
    fn compile_let_and_return() {
        let src = "fn add(a: i64, b: i64) -> i64 { let s = a + b; return s; }";
        let chunk = compile_src(src).expect("compile");
        assert!(chunk.code.contains(&(Op::StoreVar as u8)));
        assert!(chunk.code.contains(&(Op::Return as u8)));
        assert_eq!(chunk.functions.len(), 1);
        assert_eq!(chunk.functions[0].name, "add");
        assert_eq!(chunk.functions[0].param_count, 2);
    }

    #[test]
    fn compile_if_else() {
        let src = "fn f(x: i64) -> i64 { if x > 0 { return 1; } else { return 0; } }";
        let chunk = compile_src(src).expect("compile");
        assert!(chunk.code.contains(&(Op::JumpIfFalse as u8)));
        assert!(chunk.code.contains(&(Op::Jump as u8)));
    }

    #[test]
    fn compile_while_loop() {
        let src =
            "fn f() budget(steps: 100) -> i64 { let i = 0; while i < 10 { i = i + 1; } return i; }";
        let chunk = compile_src(src).expect("compile");
        assert!(chunk.code.contains(&(Op::JumpIfFalse as u8)));
        assert!(chunk.code.contains(&(Op::Jump as u8)));
        assert_eq!(chunk.functions[0].budget_steps, 100);
    }

    #[test]
    fn compile_list_literal() {
        let chunk = compile_expr(&crate::parse::parse_cell("= [1, 2, 3]").expect("parse"))
            .expect("compile");
        assert!(chunk.code.contains(&(Op::MakeList as u8)));
    }

    #[test]
    fn compile_record_literal() {
        let chunk = compile_expr(&crate::parse::parse_cell("= { x: 1, y: 2 }").expect("parse"))
            .expect("compile");
        assert!(chunk.code.contains(&(Op::MakeRecord as u8)));
    }

    #[test]
    fn compile_string_constant_dedup() {
        let chunk = compile_expr(&crate::parse::parse_cell(r#"= "hello""#).expect("parse"))
            .expect("compile");
        assert!(!chunk.constants.is_empty());
        if let Some(Const::String(s)) = chunk.constants.first() {
            assert_eq!(s, "hello");
        } else {
            panic!("expected string constant");
        }
    }

    #[test]
    fn compile_function_call() {
        let src =
            "fn double(x: i64) -> i64 { return x + x; } fn main() -> i64 { return double(5); }";
        let chunk = compile_src(src).expect("compile");
        assert!(chunk.code.contains(&(Op::CallUser as u8)));
        assert_eq!(chunk.functions.len(), 2);
    }

    #[test]
    fn compile_host_call() {
        let src = "fn f() -> f64 { return math.sqrt(16.0); }";
        let chunk = compile_src(src).expect("compile");
        assert!(chunk.code.contains(&(Op::CallHost as u8)));
    }

    #[test]
    fn compile_for_loop() {
        let src = "fn f() budget(steps: 100) -> i64 { let s = 0; for x in [1, 2, 3] { s = s + x; } return s; }";
        let chunk = compile_src(src).expect("compile");
        assert!(chunk.code.contains(&(Op::GetIndex as u8)));
        assert!(chunk.code.contains(&(Op::Jump as u8)));
    }

    #[test]
    fn compile_logical_and() {
        let chunk = compile_expr(&crate::parse::parse_cell("= true && false").expect("parse"))
            .expect("compile");
        assert!(chunk.code.contains(&(Op::Dup as u8)));
        assert!(chunk.code.contains(&(Op::JumpIfFalse as u8)));
    }

    #[test]
    fn compile_match() {
        let src = "fn f(x: i64) -> i64 { match x { 1 => { return 10; } _ => { return 0; } } }";
        let chunk = compile_src(src).expect("compile");
        assert!(chunk.code.contains(&(Op::Eq as u8)));
        assert!(chunk.code.contains(&(Op::JumpIfFalse as u8)));
    }
}
