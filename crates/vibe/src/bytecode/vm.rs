//! Stack-based virtual machine for `vibe-bc-0.1` bytecode.
//!
//! Executes a compiled `Chunk` by walking the code segment, maintaining a
//! value stack, and dispatching opcodes. Host capability calls are routed
//! through the `Host` trait, same as the AST interpreter.

use crate::bind::Host;
use crate::budget::Budget;
use crate::bytecode::op::{Chunk, Const, Op};
use crate::error::{DiagCode, Diagnostic};
use crate::extension_registry::{
    route_call, CallExecutionContext, DispatchOutcome, ExtensionRegistry, PreparedCallCache,
};
use crate::span::Span;
use crate::value::{EnumValue, Value};
use std::collections::BTreeMap;
use vibescript_abi::{q_hash_str, ExecutionMode, SentinelPolicyGate};

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

/// The virtual machine.
pub struct Vm<'a, H: Host> {
    chunk: &'a Chunk,
    host: &'a mut H,
    budget: Budget,
    /// Value stack.
    stack: Vec<Value>,
    /// Call frames: each frame has its own locals array.
    frames: Vec<Frame>,
    /// Generational extension registry (`Stage S6`).
    extension_registry: ExtensionRegistry<'a>,
    /// Zero-heap prepared call handle cache (`Stage S6`).
    prepared_cache: PreparedCallCache,
    /// Active extension execution context.
    call_context: CallExecutionContext,
    /// Optional Sentinel policy gate for governed/simulated execution modes.
    sentinel_gate: Option<&'a mut dyn SentinelPolicyGate>,
    /// Most recent extension `DispatchOutcome` (receipt + audit `NQuin48`s).
    last_extension_outcome: Option<DispatchOutcome>,
}

/// A call frame.
struct Frame {
    /// Code offset (return address).
    return_pc: usize,
    /// Local variable slots for this frame.
    locals: Vec<Value>,
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
            extension_registry: ExtensionRegistry::with_builtin_providers(),
            prepared_cache: PreparedCallCache::new(),
            call_context: CallExecutionContext {
                actor_did_hash: q_hash_str("did:q42:local_vm"),
                context_world_hash: 1,
                lamport_timestamp: 1,
                mode: ExecutionMode::StandaloneDeveloper,
            },
            sentinel_gate: None,
            last_extension_outcome: None,
        }
    }

    /// Configures governed extension execution (`CallExecutionContext` + optional `SentinelPolicyGate`).
    pub fn with_extension_governance(
        mut self,
        call_context: CallExecutionContext,
        sentinel_gate: Option<&'a mut dyn SentinelPolicyGate>,
    ) -> Self {
        self.call_context = call_context;
        self.sentinel_gate = sentinel_gate;
        self
    }

    /// Mutable reference to the VM's `ExtensionRegistry`.
    pub fn extension_registry_mut(&mut self) -> &mut ExtensionRegistry<'a> {
        &mut self.extension_registry
    }

    /// Returns the most recent extension `DispatchOutcome` (receipt + audit `NQuin48`s), if any.
    pub fn last_extension_outcome(&self) -> Option<&DispatchOutcome> {
        self.last_extension_outcome.as_ref()
    }

    /// Execute the chunk's top-level preamble and return the last value on the stack.
    pub fn run(&mut self) -> Result<Value, VmError> {
        super::task_admission::validate(self.chunk)?;
        let mut pc: usize = 0;
        // Top-level locals.
        let top_locals = vec![Value::Null; self.chunk.top_locals as usize];
        self.frames.push(Frame {
            return_pc: usize::MAX,
            locals: top_locals,
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
                self.push(Value::Null)?;
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
        super::task_admission::validate(self.chunk)?;
        let meta = self
            .chunk
            .functions
            .get(fn_idx as usize)
            .ok_or(VmError::InvalidFunction(fn_idx))?
            .clone();

        if meta.budget_steps > 0 {
            self.budget.steps_left = self.budget.steps_left.min(meta.budget_steps);
        }

        let mut locals = vec![Value::Null; meta.local_count as usize];
        for (i, arg) in args.iter().enumerate() {
            if i < meta.param_count as usize && i < locals.len() {
                locals[i] = arg.clone();
            }
        }

        // Set up frame and jump to function code.
        self.frames.push(Frame {
            return_pc: usize::MAX,
            locals,
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
                // Same truthiness as the tree-walking interpreter.
                let is_false = !cond.is_truthy();
                if is_false {
                    *pc = target;
                }
                Ok(())
            }
            Op::JumpIfTrue => {
                let target = self.read_u16(pc)? as usize;
                let cond = self.pop()?;
                let is_true = cond.is_truthy();
                if is_true {
                    *pc = target;
                }
                Ok(())
            }

            // Calls
            Op::CallHost => {
                let path_idx = self.read_u16(pc)?;
                let argc = self.read_u8(pc)? as usize;
                self.call_host(path_idx, argc, 0)
            }
            Op::CallHostNamed => {
                let path_idx = self.read_u16(pc)?;
                let pos_argc = self.read_u8(pc)? as usize;
                let named_pairs = self.read_u8(pc)? as usize;
                self.call_host(path_idx, pos_argc, named_pairs)
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
                let catch_off = self.read_u16(pc)? as usize;
                match self.pop()? {
                    Value::Ok(inner) => self.push(*inner),
                    Value::Err(err_val) => {
                        if catch_off != *pc {
                            self.push(Value::Err(err_val))?;
                            *pc = catch_off;
                            Ok(())
                        } else {
                            Err(VmError::Diagnostic(Diagnostic::new(
                                DiagCode::E600,
                                Span::new(0, 0),
                                format!("? on Err: {err_val:?}"),
                            )))
                        }
                    }
                    other => self.push(other),
                }
            }
            Op::Effect => {
                // Pop and discard (side effect already happened).
                self.pop()?;
                Ok(())
            }
            Op::IsOk => {
                let v = self.pop()?;
                self.push(Value::Bool(matches!(v, Value::Ok(_))))
            }
            Op::UnwrapOk => match self.pop()? {
                Value::Ok(inner) => self.push(*inner),
                _ => Err(VmError::TypeMismatch("unwrap Ok")),
            },
            Op::IsErr => {
                let v = self.pop()?;
                self.push(Value::Bool(matches!(v, Value::Err(_))))
            }
            Op::UnwrapErr => match self.pop()? {
                Value::Err(inner) => self.push(*inner),
                _ => Err(VmError::TypeMismatch("unwrap Err")),
            },
            Op::IsVariant => {
                let enum_idx = self.read_u16(pc)?;
                let var_idx = self.read_u16(pc)?;
                let arity = self.read_u8(pc)? as usize;
                let enum_name = self.get_string_const(enum_idx)?;
                let variant_name = self.get_string_const(var_idx)?;
                let v = self.pop()?;
                let matched = matches!(
                    v,
                    Value::Enum(ref e)
                        if e.enum_name == enum_name
                            && e.variant_name == variant_name
                            && e.payload.len() == arity
                );
                self.push(Value::Bool(matched))
            }
            Op::GetVariantField => {
                let idx = self.read_u8(pc)? as usize;
                match self.pop()? {
                    Value::Enum(e) => {
                        let field = e
                            .payload
                            .get(idx)
                            .cloned()
                            .ok_or(VmError::TypeMismatch("enum field index"))?;
                        self.push(field)
                    }
                    _ => Err(VmError::TypeMismatch("enum variant field")),
                }
            }
            Op::IsRecord => {
                let v = self.pop()?;
                self.push(Value::Bool(matches!(v, Value::Record(_))))
            }
            Op::HasMember => {
                let key_idx = self.read_u16(pc)?;
                let key = self.get_string_const(key_idx)?;
                let v = self.pop()?;
                let matched = matches!(v, Value::Record(ref r) if r.contains_key(&key));
                self.push(Value::Bool(matched))
            }
            Op::IsListLen => {
                let len = self.read_u16(pc)? as usize;
                let v = self.pop()?;
                let matched = matches!(v, Value::List(ref xs) if xs.len() == len);
                self.push(Value::Bool(matched))
            }
            Op::TrapMatch => Err(VmError::Diagnostic(Diagnostic::new(
                DiagCode::E600,
                Span::new(0, 0),
                "destructuring let pattern failed to match",
            ))),
            Op::MakeVariant => {
                let enum_idx = self.read_u16(pc)?;
                let var_idx = self.read_u16(pc)?;
                let argc = self.read_u8(pc)? as usize;
                let enum_name = self.get_string_const(enum_idx)?;
                let variant_name = self.get_string_const(var_idx)?;
                let mut payload = Vec::with_capacity(argc);
                for _ in 0..argc {
                    payload.push(self.pop()?);
                }
                payload.reverse();
                self.push(Value::Enum(EnumValue::with_payload(
                    &enum_name,
                    &variant_name,
                    payload,
                )))
            }
            Op::MakeOk => {
                let v = self.pop()?;
                self.push(Value::Ok(Box::new(v)))
            }
            Op::MakeErr => {
                let v = self.pop()?;
                self.push(Value::Err(Box::new(v)))
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
        frame
            .locals
            .get(slot as usize)
            .cloned()
            .ok_or(VmError::InvalidLocal(slot))
    }

    fn store_local(&mut self, slot: u16, value: Value) -> Result<(), VmError> {
        let frame = self.frames.last_mut().ok_or(VmError::StackUnderflow)?;
        if slot as usize >= frame.locals.len() {
            return Err(VmError::InvalidLocal(slot));
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

    fn call_host(
        &mut self,
        path_idx: u16,
        pos_argc: usize,
        named_pairs: usize,
    ) -> Result<(), VmError> {
        let mut named_args = Vec::with_capacity(named_pairs);
        for _ in 0..named_pairs {
            let val = self.pop()?;
            let key = match self.pop()? {
                Value::String(k) => k,
                _ => return Err(VmError::TypeMismatch("named argument key")),
            };
            named_args.push((key, val));
        }
        named_args.reverse();

        let mut pos_args = Vec::with_capacity(pos_argc);
        for _ in 0..pos_argc {
            pos_args.push(self.pop()?);
        }
        pos_args.reverse();

        if path_idx == 0xFFFF {
            let _callee = self.pop()?;
            self.push(Value::Null)?;
            return Ok(());
        }

        let path = self.get_string_const(path_idx)?;
        let span = Span::new(0, 0);

        let result = route_call(
            self.host,
            &self.extension_registry,
            &mut self.prepared_cache,
            &self.call_context,
            self.sentinel_gate.as_deref_mut(),
            &mut self.last_extension_outcome,
            &mut self.budget,
            &path,
            &pos_args,
            &named_args,
            span,
        )?;
        self.push(result)
    }

    fn call_user(&mut self, fn_idx: u16, argc: usize, pc: &mut usize) -> Result<(), VmError> {
        let meta = self
            .chunk
            .functions
            .get(fn_idx as usize)
            .ok_or(VmError::InvalidFunction(fn_idx))?
            .clone();

        if meta.budget_steps > 0 {
            self.budget.steps_left = self.budget.steps_left.min(meta.budget_steps);
        }

        // Pop arguments in reverse.
        let mut args = Vec::with_capacity(argc);
        for _ in 0..argc {
            args.push(self.pop()?);
        }
        args.reverse();

        // Set up new frame.
        let mut locals = vec![Value::Null; meta.local_count as usize];
        for (i, arg) in args.iter().enumerate() {
            if i < meta.param_count as usize && i < locals.len() {
                locals[i] = arg.clone();
            }
        }

        let return_pc = *pc;
        self.frames.push(Frame { return_pc, locals });

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

    #[test]
    fn vm_ast_parity_patterns_letpat_try_budget_and_args() {
        use crate::eval::{Engine, Env};
        use crate::parse::parse_program;

        // 1. Destructuring LetPat + Match on Some/None/Ok/Err/List/Record/Enum + Try unwrap
        let src = r#"
enum Color { Red, Rgb(i64, i64, i64) }
fn inspect_all(opt: i64, res: i64, tag: i64) -> i64 {
    let [a, b] = [10, 20];
    let { x, y } = { x: 3, y: 4 };
    let mut v_opt = 0;
    match opt {
        Some(n) => { v_opt = n + 1; }
        None => { v_opt = 0; }
    }
    let r_val = if res > 0 { Ok(res * 2) } else { Err("neg") };
    let mut v_res = 0;
    match r_val {
        Ok(k) => { v_res = k; }
        Err(_e) => { v_res = -1; }
    }
    let c = if tag == 1 { Color.Rgb(1, 2, 3) } else { Color.Red() };
    let mut v_col = 0;
    match c {
        Color.Rgb(r, g, bl) => { v_col = r + g + bl; }
        Color.Red() => { v_col = 9; }
        _ => { v_col = 0; }
    }
    let unwrapped = Ok(5)?;
    return a + b + x + y + v_opt + v_res + v_col + unwrapped;
}
"#;
        let prog = parse_program(src).expect("parse");
        let chunk = compile(&prog).expect("compile");
        let idx = chunk.find_function("inspect_all").expect("fn");

        for (opt, res, tag) in [
            (Value::I64(7), Value::I64(4), Value::I64(1)),
            (Value::Null, Value::I64(-1), Value::I64(0)),
        ] {
            let mut h1 = LocalHost::default();
            let mut vm = Vm::new(&chunk, &mut h1, Budget::default());
            vm.run().expect("preamble");
            let vm_out = vm
                .call_function(idx, &[opt.clone(), res.clone(), tag.clone()])
                .expect("vm call");

            let mut h2 = LocalHost::default();
            let mut engine = Engine::new(&mut h2, Budget::default());
            let mut env = Env::default();
            let ast_out = engine
                .call_function(&prog, "inspect_all", vec![opt, res, tag], &mut env)
                .expect("ast call");
            assert_eq!(vm_out, ast_out);
        }

        // 2. Try (?) on Err propagates E600
        let err_try = parse_program(r#"fn fail_try() -> i64 { return Err("boom")?; }"#).unwrap();
        let err_chunk = compile(&err_try).unwrap();
        let mut h = LocalHost::default();
        let mut vm = Vm::new(&err_chunk, &mut h, Budget::default());
        vm.run().unwrap();
        let f_idx = err_chunk.find_function("fail_try").unwrap();
        assert!(matches!(
            vm.call_function(f_idx, &[]),
            Err(VmError::Diagnostic(d)) if d.code == DiagCode::E600
        ));

        // 3. Named + positional args on host call + positional string pairs NOT misclassified
        let query_src = r#"fn q() -> List { return graph.query("s", "p", take: 3); }"#;
        let q_prog = parse_program(query_src).unwrap();
        let q_chunk = compile(&q_prog).unwrap();
        let mut qh = LocalHost {
            query_rows: 10,
            ..LocalHost::default()
        };
        let mut qvm = Vm::new(&q_chunk, &mut qh, Budget::default());
        qvm.run().unwrap();
        let q_idx = q_chunk.find_function("q").unwrap();
        let q_res = qvm.call_function(q_idx, &[]).expect("query");
        assert_eq!(q_res, Value::List(vec![Value::I64(1); 3]));

        // Positional even string pair ("hello", "world") stays positional
        let str_src = r#"fn eqs() -> bool { return str.eq("hello", "hello"); }"#;
        let str_prog = parse_program(str_src).unwrap();
        let str_chunk = compile(&str_prog).unwrap();
        let mut sh = LocalHost::default();
        let mut svm = Vm::new(&str_chunk, &mut sh, Budget::default());
        svm.run().unwrap();
        let eqs_idx = str_chunk.find_function("eqs").unwrap();
        assert_eq!(svm.call_function(eqs_idx, &[]).unwrap(), Value::Bool(true));

        // 4. Function budget_steps is enforced by the VM
        let spin_src = r#"fn spin() budget(steps: 5) -> i64 { let i = 0; while i < 100 { i = i + 1; } return i; }"#;
        let spin_prog = parse_program(spin_src).unwrap();
        let spin_chunk = compile(&spin_prog).unwrap();
        let mut bh = LocalHost::default();
        let mut bvm = Vm::new(&spin_chunk, &mut bh, Budget::default());
        bvm.run().unwrap();
        let s_idx = spin_chunk.find_function("spin").unwrap();
        assert!(matches!(
            bvm.call_function(s_idx, &[]),
            Err(VmError::Diagnostic(d)) if d.code == DiagCode::E400
        ));

        // 5. Out-of-bounds local load fails closed instead of returning Null
        let mut bad_chunk = Chunk::new();
        bad_chunk.top_locals = 0;
        bad_chunk.emit_op(Op::LoadVar);
        bad_chunk.emit_u16(0);
        bad_chunk.emit_op(Op::Halt);
        let mut bh2 = LocalHost::default();
        let mut bvm2 = Vm::new(&bad_chunk, &mut bh2, Budget::default());
        assert!(matches!(bvm2.run(), Err(VmError::InvalidLocal(0))));
    }
}
