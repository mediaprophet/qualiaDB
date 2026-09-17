//! Stack-based virtual machine for `vibe-bc-0.1` bytecode.
//!
//! Executes a compiled `Chunk` by walking the code segment, maintaining a
//! value stack, and dispatching opcodes. Host capability calls are routed
//! through the `Host` trait, same as the AST interpreter.

use crate::bind::{dispatch, Host};
use crate::budget::Budget;
use crate::bytecode::op::{Chunk, Const, Op};
use crate::error::{DiagCode, Diagnostic};
use crate::span::Span;
use crate::value::Value;
use std::collections::BTreeMap;

/// VM execution error.
#[derive(Debug, Clone, PartialEq)]
pub enum VmError {
    /// A `Diagnostic` from the VibeScript error system.
    Diagnostic(Diagnostic),
    /// Stack underflow (not enough values on the stack).
    StackUnderflow,
    /// Stack overflow (too many values on the stack).
    StackOverflow,
    /// Invalid opcode byte.
    InvalidOpcode(u8),
    /// Code pointer out of bounds.
    CodeOutOfBounds,
    /// Invalid constant index.
    InvalidConstant(u16),
    /// Invalid function index.
    InvalidFunction(u16),
    /// Invalid local variable slot.
    InvalidLocal(u16),
    /// Division by zero.
    DivisionByZero,
    /// Budget exhausted.
    BudgetExhausted,
    /// Type mismatch at runtime.
    TypeMismatch(&'static str),
    /// No return value on the stack when one was expected.
    NoReturnValue,
}

impl std::fmt::Display for VmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Diagnostic(d) => write!(f, "{d}"),
            Self::StackUnderflow => write!(f, "vm: stack underflow"),
            Self::StackOverflow => write!(f, "vm: stack overflow"),
            Self::InvalidOpcode(b) => write!(f, "vm: invalid opcode 0x{b:02X}"),
            Self::CodeOutOfBounds => write!(f, "vm: code pointer out of bounds"),
            Self::InvalidConstant(i) => write!(f, "vm: invalid constant index {i}"),
            Self::InvalidFunction(i) => write!(f, "vm: invalid function index {i}"),
            Self::InvalidLocal(i) => write!(f, "vm: invalid local slot {i}"),
            Self::DivisionByZero => write!(f, "vm: division by zero"),
            Self::BudgetExhausted => write!(f, "vm: budget exhausted"),
            Self::TypeMismatch(what) => write!(f, "vm: type mismatch: {what}"),
            Self::NoReturnValue => write!(f, "vm: no return value"),
        }
    }
}

impl std::error::Error for VmError {}

impl From<Diagnostic> for VmError {
    fn from(d: Diagnostic) -> Self {
        Self::Diagnostic(d)
    }
}

const MAX_STACK: usize = 1024;

/// Split a flat argument list into positional args and named args.
///
/// Named args are emitted by the compiler as alternating (String, Value) pairs.
/// We detect this pattern: if the list has even length and every even-indexed
/// element is a `Value::String`, we treat it as named args.
fn split_args(args: Vec<Value>) -> (Vec<Value>, Vec<(String, Value)>) {
    if args.len() >= 2 && args.len() % 2 == 0 {
        let mut named = Vec::with_capacity(args.len() / 2);
        let mut is_named = true;
        for chunk in args.chunks(2) {
            if let Value::String(k) = &chunk[0] {
                named.push((k.clone(), chunk[1].clone()));
            } else {
                is_named = false;
                break;
            }
        }
        if is_named {
            return (Vec::new(), named);
        }
    }
    (args, Vec::new())
}

/// The virtual machine.
pub struct Vm<'a, H: Host> {
    chunk: &'a Chunk,
    host: &'a mut H,
    budget: Budget,
    /// Value stack.
    stack: Vec<Value>,
    /// Call frames: each frame has its own locals array.
    frames: Vec<Frame>,
}

/// A call frame.
struct Frame {
    /// Code offset (return address).
    return_pc: usize,
    /// Local variable slots for this frame.
    locals: Vec<Value>,
    /// Number of arguments passed.
    #[allow(dead_code)]
    arg_count: u8,
}

impl<'a, H: Host> Vm<'a, H> {
    /// Create a new VM for executing the given chunk.
    pub fn new(chunk: &'a Chunk, host: &'a mut H, budget: Budget) -> Self {
        Self {
            chunk,
            host,
            budget,
            stack: Vec::with_capacity(256),
            frames: Vec::new(),
        }
    }

    /// Execute the chunk's top-level preamble and return the last value on the stack.
    pub fn run(&mut self) -> Result<Value, VmError> {
        let mut pc: usize = 0;
        // Top-level locals.
        let top_locals = vec![Value::Null; self.chunk.top_locals as usize];
        self.frames.push(Frame {
            return_pc: usize::MAX,
            locals: top_locals.clone(),
            arg_count: 0,
        });

        loop {
            self.budget.tick(Span::new(0, 0)).map_err(VmError::from)?;
            let op_byte = self.read_u8(&mut pc)?;
            let op = Op::from_byte(op_byte).ok_or(VmError::InvalidOpcode(op_byte))?;

            // Return at top level means "return the top of stack".
            if op == Op::Return {
                self.frames.pop();
                break;
            }
            if op == Op::ReturnNull {
                self.frames.pop();
                let _ = self.push(Value::Null);
                break;
            }

            self.exec_op(op, &mut pc)?;

            if op == Op::Halt {
                break;
            }
        }

        // Return the last value on the stack, or Null.
        if let Some(v) = self.stack.pop() {
            Ok(v)
        } else {
            Ok(Value::Null)
        }
    }

    /// Execute a specific user function by index with the given arguments.
    pub fn call_function(&mut self, fn_idx: u16, args: &[Value]) -> Result<Value, VmError> {
        let meta = self
            .chunk
            .functions
            .get(fn_idx as usize)
            .ok_or(VmError::InvalidFunction(fn_idx))?
            .clone();

        let mut locals = vec![Value::Null; meta.local_count as usize];
        for (i, arg) in args.iter().enumerate() {
            if i < meta.param_count as usize {
                locals[i] = arg.clone();
            }
        }

        // Set up frame and jump to function code.
        self.frames.push(Frame {
            return_pc: usize::MAX,
            locals,
            arg_count: args.len() as u8,
        });

        let mut pc = meta.code_offset as usize;
        self.exec_function_body(&mut pc)
    }

    fn exec_function_body(&mut self, pc: &mut usize) -> Result<Value, VmError> {
        loop {
            self.budget.tick(Span::new(0, 0)).map_err(VmError::from)?;
            let op_byte = self.read_u8(pc)?;
            let op = Op::from_byte(op_byte).ok_or(VmError::InvalidOpcode(op_byte))?;

            match op {
                Op::Return => {
                    let v = self.pop()?;
                    self.frames.pop();
                    return Ok(v);
                }
                Op::ReturnNull => {
                    self.frames.pop();
                    return Ok(Value::Null);
                }
                _ => self.exec_op(op, pc)?,
            }
        }
    }

    fn exec_op(&mut self, op: Op, pc: &mut usize) -> Result<(), VmError> {
        match op {
            Op::PushNull => self.push(Value::Null),
            Op::PushBool => {
                let b = self.read_u8(pc)?;
                self.push(Value::Bool(b != 0))
            }
            Op::PushInt => {
                let v = self.read_i64(pc)?;
                self.push(Value::I64(v))
            }
            Op::PushUInt => {
                let v = self.read_u64(pc)?;
                self.push(Value::U64(v))
            }
            Op::PushFloat => {
                let bits = self.read_u64(pc)?;
                self.push(Value::F64(f64::from_bits(bits)))
            }
            Op::PushQuantity => {
                let bits = self.read_u64(pc)?;
                let idx = self.read_u16(pc)?;
                let unit = self.get_string_const(idx)?;
                self.push(Value::Quantity(crate::value::Quantity {
                    value: f64::from_bits(bits),
                    unit,
                }))
            }
            Op::PushString => {
                let idx = self.read_u16(pc)?;
                let s = self.get_string_const(idx)?;
                self.push(Value::String(s))
            }
            Op::PushIri => {
                let idx = self.read_u16(pc)?;
                let s = self.get_iri_const(idx)?;
                self.push(Value::Iri(s))
            }
            Op::LoadVar => {
                let slot = self.read_u16(pc)?;
                let v = self.load_local(slot)?;
                self.push(v)
            }
            Op::StoreVar => {
                let slot = self.read_u16(pc)?;
                let v = self.pop()?;
                self.store_local(slot, v)
            }
            Op::Pop => {
                self.pop()?;
                Ok(())
            }
            Op::Dup => {
                let v = self.peek()?.clone();
                self.push(v)
            }

            // Arithmetic
            Op::Add => self.binop_num(
                |a, b| a + b,
                |a, b| a + b,
                |a, b| a + b,
                |a, b| a.wrapping_add(b),
            ),
            Op::Sub => self.binop_num(
                |a, b| a - b,
                |a, b| a - b,
                |a, b| a - b,
                |a, b| a.wrapping_sub(b),
            ),
            Op::Mul => self.binop_num(
                |a, b| a * b,
                |a, b| a * b,
                |a, b| a * b,
                |a, b| a.wrapping_mul(b),
            ),
            Op::Div => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.div_op(a, b)
            }
            Op::Rem => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.rem_op(a, b)
            }
            Op::Neg => {
                let a = self.pop()?;
                match a {
                    Value::I64(v) => self.push(Value::I64(-v)),
                    Value::F64(v) => self.push(Value::F64(-v)),
                    Value::U64(v) => self.push(Value::I64(-(v as i64))),
                    _ => Err(VmError::TypeMismatch("neg")),
                }
            }
            Op::Not => {
                let a = self.pop()?;
                match a {
                    Value::Bool(b) => self.push(Value::Bool(!b)),
                    _ => Err(VmError::TypeMismatch("not")),
                }
            }

            // Comparison
            Op::Eq => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a == b))
            }
            Op::Ne => {
                let b = self.pop()?;
                let a = self.pop()?;
                self.push(Value::Bool(a != b))
            }
            Op::Lt => self.cmp_op(|o| o == std::cmp::Ordering::Less),
            Op::Le => self.cmp_op(|o| o != std::cmp::Ordering::Greater),
            Op::Gt => self.cmp_op(|o| o == std::cmp::Ordering::Greater),
            Op::Ge => self.cmp_op(|o| o != std::cmp::Ordering::Less),
            Op::And => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (a, b) {
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x && y)),
                    _ => Err(VmError::TypeMismatch("and")),
                }
            }
            Op::Or => {
                let b = self.pop()?;
                let a = self.pop()?;
                match (a, b) {
                    (Value::Bool(x), Value::Bool(y)) => self.push(Value::Bool(x || y)),
                    _ => Err(VmError::TypeMismatch("or")),
                }
            }

            // Control flow
            Op::Jump => {
                let target = self.read_u16(pc)? as usize;
                *pc = target;
                Ok(())
            }
            Op::JumpIfFalse => {
                let target = self.read_u16(pc)? as usize;
                let cond = self.pop()?;
                let is_false = match cond {
                    Value::Bool(b) => !b,
                    Value::Null => true,
                    _ => false,
                };
                if is_false {
                    *pc = target;
                }
                Ok(())
            }
            Op::JumpIfTrue => {
                let target = self.read_u16(pc)? as usize;
                let cond = self.pop()?;
                let is_true = match cond {
                    Value::Bool(b) => b,
                    Value::Null => false,
                    _ => true,
                };
                if is_true {
                    *pc = target;
                }
                Ok(())
            }

            // Calls
            Op::CallHost => {
                let path_idx = self.read_u16(pc)?;
                let argc = self.read_u8(pc)? as usize;
                self.call_host(path_idx, argc)
            }
            Op::CallUser => {
                let fn_idx = self.read_u16(pc)?;
                let argc = self.read_u8(pc)? as usize;
                self.call_user(fn_idx, argc, pc)
            }
            Op::Return => {
                // Handled in exec_function_body, but for top-level it's a no-op.
                // Pop the return value and push it back (it stays on stack).
                Ok(())
            }
            Op::ReturnNull => Ok(()),

            // Data structures
            Op::MakeList => {
                let count = self.read_u16(pc)? as usize;
                let mut elements = Vec::with_capacity(count);
                for _ in 0..count {
                    elements.push(self.pop()?);
                }
                elements.reverse();
                self.push(Value::List(elements))
            }
            Op::MakeRecord => {
                let count = self.read_u16(pc)? as usize;
                let mut fields: BTreeMap<String, Value> = BTreeMap::new();
                for _ in 0..count {
                    let val = self.pop()?;
                    let key = self.pop()?;
                    if let Value::String(k) = key {
                        fields.insert(k, val);
                    } else {
                        return Err(VmError::TypeMismatch("record key"));
                    }
                }
                self.push(Value::Record(fields))
            }
            Op::GetMember => {
                let name_idx = self.read_u16(pc)?;
                let name = self.get_string_const(name_idx)?;
                let recv = self.pop()?;
                match recv {
                    Value::Record(fields) => {
                        let v = fields.get(&name).cloned().unwrap_or(Value::Null);
                        self.push(v)
                    }
                    _ => Err(VmError::TypeMismatch("member access on non-record")),
                }
            }
            Op::GetIndex => {
                let index = self.pop()?;
                let recv = self.pop()?;
                match (recv, index) {
                    (Value::List(items), Value::I64(i)) => {
                        if i >= 0 && (i as usize) < items.len() {
                            self.push(items[i as usize].clone())
                        } else {
                            self.push(Value::Null)
                        }
                    }
                    (Value::List(items), Value::U64(i)) => {
                        if (i as usize) < items.len() {
                            self.push(items[i as usize].clone())
                        } else {
                            self.push(Value::Null)
                        }
                    }
                    _ => Err(VmError::TypeMismatch("index on non-list")),
                }
            }
            Op::TryUnwrap => {
                // Read catch offset (unused in 0.1 — we just pass through).
                let _catch = self.read_u16(pc)?;
                // The value is already on the stack; TryUnwrap is a no-op for now.
                Ok(())
            }
            Op::Effect => {
                // Pop and discard (side effect already happened).
                self.pop()?;
                Ok(())
            }
            Op::Halt => Ok(()),
        }
    }

    // ── Stack helpers ────────────────────────────────────────────

    fn push(&mut self, v: Value) -> Result<(), VmError> {
        if self.stack.len() >= MAX_STACK {
            return Err(VmError::StackOverflow);
        }
        self.stack.push(v);
        Ok(())
    }

    fn pop(&mut self) -> Result<Value, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow)
    }

    fn peek(&self) -> Result<&Value, VmError> {
        self.stack.last().ok_or(VmError::StackUnderflow)
    }

    // ── Code reading ─────────────────────────────────────────────

    fn read_u8(&self, pc: &mut usize) -> Result<u8, VmError> {
        if *pc >= self.chunk.code.len() {
            return Err(VmError::CodeOutOfBounds);
        }
        let b = self.chunk.code[*pc];
        *pc += 1;
        Ok(b)
    }

    fn read_u16(&self, pc: &mut usize) -> Result<u16, VmError> {
        let lo = self.read_u8(pc)? as u16;
        let hi = self.read_u8(pc)? as u16;
        Ok(lo | (hi << 8))
    }

    fn read_i64(&self, pc: &mut usize) -> Result<i64, VmError> {
        let mut bytes = [0u8; 8];
        for b in &mut bytes {
            *b = self.read_u8(pc)?;
        }
        Ok(i64::from_le_bytes(bytes))
    }

    fn read_u64(&self, pc: &mut usize) -> Result<u64, VmError> {
        let mut bytes = [0u8; 8];
        for b in &mut bytes {
            *b = self.read_u8(pc)?;
        }
        Ok(u64::from_le_bytes(bytes))
    }

    // ── Constant pool ────────────────────────────────────────────

    fn get_string_const(&self, idx: u16) -> Result<String, VmError> {
        match self.chunk.constants.get(idx as usize) {
            Some(Const::String(s)) => Ok(s.clone()),
            Some(Const::Iri(s)) => Ok(s.clone()),
            None => Err(VmError::InvalidConstant(idx)),
        }
    }

    fn get_iri_const(&self, idx: u16) -> Result<String, VmError> {
        match self.chunk.constants.get(idx as usize) {
            Some(Const::Iri(s)) => Ok(s.clone()),
            Some(Const::String(s)) => Ok(s.clone()),
            None => Err(VmError::InvalidConstant(idx)),
        }
    }

    // ── Locals ───────────────────────────────────────────────────

    fn load_local(&self, slot: u16) -> Result<Value, VmError> {
        let frame = self.frames.last().ok_or(VmError::StackUnderflow)?;
        if slot as usize >= frame.locals.len() {
            // If the slot is beyond the frame's locals, return Null
            // (this handles top-level locals in function context).
            Ok(Value::Null)
        } else {
            Ok(frame.locals[slot as usize].clone())
        }
    }

    fn store_local(&mut self, slot: u16, value: Value) -> Result<(), VmError> {
        let frame = self.frames.last_mut().ok_or(VmError::StackUnderflow)?;
        if slot as usize >= frame.locals.len() {
            // Extend locals if needed.
            frame.locals.resize(slot as usize + 1, Value::Null);
        }
        frame.locals[slot as usize] = value;
        Ok(())
    }

    // ── Operations ───────────────────────────────────────────────

    fn binop_num(
        &mut self,
        f64f: impl Fn(f64, f64) -> f64,
        i64f: impl Fn(i64, i64) -> i64,
        u64f: impl Fn(u64, u64) -> u64,
        _i64wrap: impl Fn(i64, i64) -> i64,
    ) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
            (Value::F64(x), Value::F64(y)) => Value::F64(f64f(x, y)),
            (Value::I64(x), Value::I64(y)) => Value::I64(i64f(x, y)),
            (Value::U64(x), Value::U64(y)) => Value::U64(u64f(x, y)),
            (Value::F64(x), Value::I64(y)) => Value::F64(f64f(x, y as f64)),
            (Value::I64(x), Value::F64(y)) => Value::F64(f64f(x as f64, y)),
            (Value::U64(x), Value::F64(y)) => Value::F64(f64f(x as f64, y)),
            (Value::F64(x), Value::U64(y)) => Value::F64(f64f(x, y as f64)),
            (Value::I64(x), Value::U64(y)) => Value::I64(i64f(x, y as i64)),
            (Value::U64(x), Value::I64(y)) => Value::I64(i64f(x as i64, y)),
            (Value::Quantity(x), Value::Quantity(y)) if x.unit == y.unit => {
                Value::Quantity(crate::value::Quantity {
                    value: f64f(x.value, y.value),
                    unit: x.unit,
                })
            }
            _ => return Err(VmError::TypeMismatch("arithmetic")),
        };
        self.push(result)
    }

    fn div_op(&mut self, a: Value, b: Value) -> Result<(), VmError> {
        let result = match (a, b) {
            (Value::F64(x), Value::F64(y)) => {
                if y == 0.0 {
                    return Err(VmError::Diagnostic(Diagnostic::new(
                        DiagCode::E600,
                        Span::new(0, 0),
                        "division by zero",
                    )));
                }
                Value::F64(x / y)
            }
            (Value::I64(x), Value::I64(y)) => {
                if y == 0 {
                    return Err(VmError::Diagnostic(Diagnostic::new(
                        DiagCode::E600,
                        Span::new(0, 0),
                        "division by zero",
                    )));
                }
                Value::I64(x / y)
            }
            (Value::U64(x), Value::U64(y)) => {
                if y == 0 {
                    return Err(VmError::Diagnostic(Diagnostic::new(
                        DiagCode::E600,
                        Span::new(0, 0),
                        "division by zero",
                    )));
                }
                Value::U64(x / y)
            }
            (Value::F64(x), Value::I64(y)) => {
                if y == 0 {
                    return Err(VmError::Diagnostic(Diagnostic::new(
                        DiagCode::E600,
                        Span::new(0, 0),
                        "division by zero",
                    )));
                }
                Value::F64(x / y as f64)
            }
            (Value::I64(x), Value::F64(y)) => {
                if y == 0.0 {
                    return Err(VmError::Diagnostic(Diagnostic::new(
                        DiagCode::E600,
                        Span::new(0, 0),
                        "division by zero",
                    )));
                }
                Value::F64(x as f64 / y)
            }
            _ => return Err(VmError::TypeMismatch("division")),
        };
        self.push(result)
    }

    fn rem_op(&mut self, a: Value, b: Value) -> Result<(), VmError> {
        let result = match (a, b) {
            (Value::F64(x), Value::F64(y)) => {
                if y == 0.0 {
                    return Err(VmError::Diagnostic(Diagnostic::new(
                        DiagCode::E600,
                        Span::new(0, 0),
                        "division by zero",
                    )));
                }
                Value::F64(x % y)
            }
            (Value::I64(x), Value::I64(y)) => {
                if y == 0 {
                    return Err(VmError::Diagnostic(Diagnostic::new(
                        DiagCode::E600,
                        Span::new(0, 0),
                        "division by zero",
                    )));
                }
                Value::I64(x % y)
            }
            (Value::U64(x), Value::U64(y)) => {
                if y == 0 {
                    return Err(VmError::Diagnostic(Diagnostic::new(
                        DiagCode::E600,
                        Span::new(0, 0),
                        "division by zero",
                    )));
                }
                Value::U64(x % y)
            }
            _ => return Err(VmError::TypeMismatch("remainder")),
        };
        self.push(result)
    }

    fn cmp_op(&mut self, check: impl Fn(std::cmp::Ordering) -> bool) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let result = match (a, b) {
            (Value::F64(x), Value::F64(y)) => {
                check(x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal))
            }
            (Value::I64(x), Value::I64(y)) => check(x.cmp(&y)),
            (Value::U64(x), Value::U64(y)) => check(x.cmp(&y)),
            (Value::F64(x), Value::I64(y)) => check(
                x.partial_cmp(&(y as f64))
                    .unwrap_or(std::cmp::Ordering::Equal),
            ),
            (Value::I64(x), Value::F64(y)) => check(
                (x as f64)
                    .partial_cmp(&y)
                    .unwrap_or(std::cmp::Ordering::Equal),
            ),
            (Value::String(x), Value::String(y)) => check(x.cmp(&y)),
            _ => return Err(VmError::TypeMismatch("comparison")),
        };
        self.push(Value::Bool(result))
    }

    fn call_host(&mut self, path_idx: u16, argc: usize) -> Result<(), VmError> {
        // Pop arguments in reverse.
        let mut args = Vec::with_capacity(argc);
        for _ in 0..argc {
            args.push(self.pop()?);
        }
        args.reverse();

        if path_idx == 0xFFFF {
            // Callee on stack — pop it.
            let callee = self.pop()?;
            // For 0.1, we don't support dynamic dispatch in the VM.
            // This is a fallback that pushes null.
            let _ = callee;
            self.push(Value::Null)?;
            return Ok(());
        }

        let path = self.get_string_const(path_idx)?;
        let span = Span::new(0, 0);

        // Separate positional and named arguments.
        // The compiler emits named args as (String key, Value) pairs.
        // We detect them by checking if every other element is a String.
        let (pos_args, named_args) = split_args(args);

        let result = dispatch(self.host, &path, &pos_args, &named_args, span)?;
        self.push(result)
    }

    fn call_user(&mut self, fn_idx: u16, argc: usize, pc: &mut usize) -> Result<(), VmError> {
        let meta = self
            .chunk
            .functions
            .get(fn_idx as usize)
            .ok_or(VmError::InvalidFunction(fn_idx))?
            .clone();

        // Pop arguments in reverse.
        let mut args = Vec::with_capacity(argc);
        for _ in 0..argc {
            args.push(self.pop()?);
        }
        args.reverse();

        // Set up new frame.
        let mut locals = vec![Value::Null; meta.local_count as usize];
        for (i, arg) in args.iter().enumerate() {
            if i < meta.param_count as usize {
                locals[i] = arg.clone();
            }
        }

        let return_pc = *pc;
        self.frames.push(Frame {
            return_pc,
            locals,
            arg_count: argc as u8,
        });

        // Jump to function code.
        *pc = meta.code_offset as usize;

        // Execute function body until Return.
        // We need to run the function inline and then restore pc.
        loop {
            self.budget.tick(Span::new(0, 0)).map_err(VmError::from)?;
            let op_byte = self.read_u8(pc)?;
            let op = Op::from_byte(op_byte).ok_or(VmError::InvalidOpcode(op_byte))?;

            match op {
                Op::Return => {
                    let v = self.pop()?;
                    let frame = self.frames.pop().ok_or(VmError::StackUnderflow)?;
                    *pc = frame.return_pc;
                    self.push(v)?;
                    break;
                }
                Op::ReturnNull => {
                    let frame = self.frames.pop().ok_or(VmError::StackUnderflow)?;
                    *pc = frame.return_pc;
                    self.push(Value::Null)?;
                    break;
                }
                _ => self.exec_op(op, pc)?,
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bind::LocalHost;
    use crate::bytecode::compiler::{compile, compile_expr};
    use crate::parse::parse_cell;

    fn run_cell(src: &str) -> Result<Value, VmError> {
        let expr = parse_cell(src).expect("parse");
        let chunk = compile_expr(&expr).expect("compile");
        let mut host = LocalHost::default();
        let mut vm = Vm::new(&chunk, &mut host, Budget::default());
        vm.run()
    }

    #[test]
    fn vm_arithmetic() {
        let v = run_cell("= 1 + 2 * 3").expect("vm");
        assert_eq!(v, Value::I64(7));
    }

    #[test]
    fn vm_float_arithmetic() {
        let v = run_cell("= 1.5 + 2.5").expect("vm");
        assert_eq!(v, Value::F64(4.0));
    }

    #[test]
    fn vm_division_by_zero() {
        let err = run_cell("= 1 / 0").expect_err("should error");
        assert!(matches!(err, VmError::Diagnostic(_)));
    }

    #[test]
    fn vm_float_division_by_zero() {
        let err = run_cell("= 1.0 / 0.0").expect_err("should error");
        assert!(matches!(err, VmError::Diagnostic(_)));
    }

    #[test]
    fn vm_comparison() {
        let v = run_cell("= 3 > 2").expect("vm");
        assert_eq!(v, Value::Bool(true));

        let v = run_cell("= 2 > 3").expect("vm");
        assert_eq!(v, Value::Bool(false));
    }

    #[test]
    fn vm_logical_and() {
        let v = run_cell("= true && false").expect("vm");
        assert_eq!(v, Value::Bool(false));

        let v = run_cell("= true && true").expect("vm");
        assert_eq!(v, Value::Bool(true));
    }

    #[test]
    fn vm_logical_or() {
        let v = run_cell("= false || true").expect("vm");
        assert_eq!(v, Value::Bool(true));
    }

    #[test]
    fn vm_negation() {
        let v = run_cell("= -5").expect("vm");
        assert_eq!(v, Value::I64(-5));
    }

    #[test]
    fn vm_not() {
        let v = run_cell("= !true").expect("vm");
        assert_eq!(v, Value::Bool(false));
    }

    #[test]
    fn vm_string_literal() {
        let v = run_cell(r#"= "hello""#).expect("vm");
        assert_eq!(v, Value::String("hello".into()));
    }

    #[test]
    fn vm_list_literal() {
        let v = run_cell("= [1, 2, 3]").expect("vm");
        match v {
            Value::List(items) => {
                assert_eq!(items.len(), 3);
                assert_eq!(items[0], Value::I64(1));
                assert_eq!(items[1], Value::I64(2));
                assert_eq!(items[2], Value::I64(3));
            }
            _ => panic!("expected list"),
        }
    }

    #[test]
    fn vm_record_literal() {
        let v = run_cell("= { x: 1, y: 2 }").expect("vm");
        match v {
            Value::Record(fields) => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields.get("x"), Some(&Value::I64(1)));
                assert_eq!(fields.get("y"), Some(&Value::I64(2)));
            }
            _ => panic!("expected record"),
        }
    }

    #[test]
    fn vm_member_access() {
        let v = run_cell("= { x: 42, y: 10 }.x").expect("vm");
        assert_eq!(v, Value::I64(42));
    }

    #[test]
    fn vm_index_access() {
        let v = run_cell("= [10, 20, 30][1]").expect("vm");
        assert_eq!(v, Value::I64(20));
    }

    #[test]
    fn vm_index_out_of_bounds() {
        let v = run_cell("= [10, 20, 30][10]").expect("vm");
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn vm_remainder() {
        let v = run_cell("= 10 % 3").expect("vm");
        assert_eq!(v, Value::I64(1));
    }

    #[test]
    fn vm_null_literal() {
        let v = run_cell("= null").expect("vm");
        assert_eq!(v, Value::Null);
    }

    #[test]
    fn vm_bool_literals() {
        let v = run_cell("= true").expect("vm");
        assert_eq!(v, Value::Bool(true));

        let v = run_cell("= false").expect("vm");
        assert_eq!(v, Value::Bool(false));
    }

    #[test]
    fn vm_uint_literal() {
        let v = run_cell("= 42u64").expect("vm");
        assert_eq!(v, Value::U64(42));
    }

    #[test]
    fn vm_nested_arithmetic() {
        let v = run_cell("= (1 + 2) * (3 + 4)").expect("vm");
        assert_eq!(v, Value::I64(21));
    }

    #[test]
    fn vm_equality() {
        let v = run_cell("= 42 == 42").expect("vm");
        assert_eq!(v, Value::Bool(true));

        let v = run_cell("= 42 != 43").expect("vm");
        assert_eq!(v, Value::Bool(true));
    }

    #[test]
    fn vm_string_equality() {
        let v = run_cell(r#"= "abc" == "abc""#).expect("vm");
        assert_eq!(v, Value::Bool(true));
    }

    #[test]
    fn vm_program_with_function() {
        use crate::parse::parse_program;
        let src =
            "fn double(x: i64) -> i64 { return x + x; } fn main() -> i64 { return double(21); }";
        let prog = parse_program(src).expect("parse");
        let chunk = compile(&prog).expect("compile");
        let mut host = LocalHost::default();
        let mut vm = Vm::new(&chunk, &mut host, Budget::default());

        // Run the preamble (which just has Halt).
        vm.run().expect("run preamble");

        // Now call main.
        let main_idx = chunk.find_function("main").expect("main exists");
        let result = vm.call_function(main_idx, &[]).expect("call main");
        assert_eq!(result, Value::I64(42));
    }

    #[test]
    fn vm_program_with_if() {
        use crate::parse::parse_program;
        let src = "fn sign(x: i64) -> i64 { if x > 0 { return 1; } else { return -1; } }";
        let prog = parse_program(src).expect("parse");
        let chunk = compile(&prog).expect("compile");
        let mut host = LocalHost::default();

        let mut vm = Vm::new(&chunk, &mut host, Budget::default());
        vm.run().expect("preamble");

        let sign_idx = chunk.find_function("sign").expect("sign exists");
        let r1 = vm.call_function(sign_idx, &[Value::I64(5)]).expect("call");
        assert_eq!(r1, Value::I64(1));

        let r2 = vm.call_function(sign_idx, &[Value::I64(-5)]).expect("call");
        assert_eq!(r2, Value::I64(-1));
    }

    #[test]
    fn vm_program_with_while() {
        use crate::parse::parse_program;
        let src = "fn sum_to(n: i64) budget(steps: 10000) -> i64 { let s = 0; let i = 0; while i < n { s = s + i; i = i + 1; } return s; }";
        let prog = parse_program(src).expect("parse");
        let chunk = compile(&prog).expect("compile");
        let mut host = LocalHost::default();

        let mut vm = Vm::new(&chunk, &mut host, Budget::default());
        vm.run().expect("preamble");

        let idx = chunk.find_function("sum_to").expect("function exists");
        let result = vm.call_function(idx, &[Value::I64(10)]).expect("call");
        assert_eq!(result, Value::I64(45)); // 0+1+2+...+9 = 45
    }

    #[test]
    fn vm_program_with_for() {
        use crate::parse::parse_program;
        let src = "fn sum_list() budget(steps: 10000) -> i64 { let s = 0; for x in [1, 2, 3, 4, 5] { s = s + x; } return s; }";
        let prog = parse_program(src).expect("parse");
        let chunk = compile(&prog).expect("compile");
        let mut host = LocalHost::default();

        let mut vm = Vm::new(&chunk, &mut host, Budget::default());
        vm.run().expect("preamble");

        let idx = chunk.find_function("sum_list").expect("function exists");
        let result = vm.call_function(idx, &[]).expect("call");
        assert_eq!(result, Value::I64(15));
    }

    #[test]
    fn vm_quantity_add_same_unit() {
        let v = run_cell("= 500ms + 20ms").expect("vm");
        match v {
            Value::Quantity(q) => {
                assert_eq!(q.unit, "ms");
                assert!((q.value - 520.0).abs() < 1e-9, "got {}", q.value);
            }
            other => panic!("expected Quantity, got {other:?}"),
        }
    }

    #[test]
    fn vm_quantity_mismatched_units_fail() {
        let err = run_cell("= 500ms + 1s").expect_err("unit mismatch");
        assert!(matches!(err, VmError::TypeMismatch(_)));
    }
}
