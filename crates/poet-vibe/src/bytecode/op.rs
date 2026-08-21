//! VibeScript bytecode opcodes (`vibe-bc-0.1`).
//!
//! Stack-based bytecode for the VibeScript 0.1 language. Each opcode is a
//! single byte, optionally followed by operand bytes. The VM is a simple
//! stack machine: expressions push values onto the stack, statements
//! consume them.
//!
//! ## Encoding
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │ Chunk                                                       │
//! │ ┌──────────┬──────────────┬───────────────────────────────┐  │
//! │ │ magic    │ version      │ constants + code + functions  │  │
//! │ │ 4 bytes  │ 2 bytes      │ ...                           │  │
//! │ └──────────┴──────────────┴───────────────────────────────┘  │
//! └─────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Opcode table
//!
//! | Opcode          | Hex  | Operand          | Stack effect         |
//! |-----------------|------|------------------|----------------------|
//! | `PUSH_NULL`     | 0x00 | —                | → null               |
//! | `PUSH_BOOL`     | 0x01 | u8 (0 or 1)      | → bool               |
//! | `PUSH_INT`      | 0x02 | i64 LE           | → i64                |
//! | `PUSH_UINT`     | 0x03 | u64 LE           | → u64                |
//! | `PUSH_FLOAT`    | 0x04 | f64 LE bits      | → f64                |
//! | `PUSH_STRING`   | 0x05 | u16 const_idx    | → string             |
//! | `PUSH_IRI`      | 0x06 | u16 const_idx    | → iri                |
//! | `LOAD_VAR`      | 0x07 | u16 slot         | → value              |
//! | `STORE_VAR`     | 0x08 | u16 slot         | value →              |
//! | `POP`           | 0x09 | —                | value →              |
//! | `DUP`           | 0x0A | —                | value → value value  |
//! | `ADD`           | 0x10 | —                | a b → a+b            |
//! | `SUB`           | 0x11 | —                | a b → a-b            |
//! | `MUL`           | 0x12 | —                | a b → a*b            |
//! | `DIV`           | 0x13 | —                | a b → a/b            |
//! | `REM`           | 0x14 | —                | a b → a%b            |
//! | `NEG`           | 0x15 | —                | a → -a               |
//! | `NOT`           | 0x16 | —                | a → !a               |
//! | `EQ`            | 0x17 | —                | a b → a==b           |
//! | `NE`            | 0x18 | —                | a b → a!=b           |
//! | `LT`            | 0x19 | —                | a b → a<b            |
//! | `LE`            | 0x1A | —                | a b → a<=b           |
//! | `GT`            | 0x1B | —                | a b → a>b            |
//! | `GE`            | 0x1C | —                | a b → a>=b           |
//! | `AND`           | 0x1D | —                | a b → a&&b           |
//! | `OR`            | 0x1E | —                | a b → a||b           |
//! | `JUMP`          | 0x20 | u16 offset       | —                    |
//! | `JUMP_IF_FALSE` | 0x21 | u16 offset       | bool →               |
//! | `JUMP_IF_TRUE`  | 0x22 | u16 offset       | bool →               |
//! | `CALL_FN`       | 0x30 | u16 fn_idx       | args... → result     |
//! | `CALL_HOST`     | 0x31 | u16 path_idx u8  | args... → result     |
//! | `CALL_USER`     | 0x32 | u16 fn_idx u8    | args... → result     |
//! | `RETURN`        | 0x33 | —                | value → (return)     |
//! | `MAKE_LIST`     | 0x40 | u16 count        | values... → list     |
//! | `MAKE_RECORD`   | 0x41 | u16 count        | kv... → record       |
//! | `GET_MEMBER`    | 0x42 | u16 name_idx     | record → value       |
//! | `GET_INDEX`     | 0x43 | —                | list index → value   |
//! | `TRY`           | 0x44 | u16 catch_off    | value → unwrapped    |
//! | `EFFECT`        | 0x45 | —                | value →              |
//! | `HALT`          | 0xFF | —                | —                    |

#![allow(dead_code)]

/// Magic bytes identifying a `vibe-bc-0.1` chunk.
pub const MAGIC: [u8; 4] = *b"VBC1";

/// Bytecode format version.
pub const VERSION: u16 = 1;

/// Maximum number of constants in a chunk.
pub const MAX_CONSTANTS: usize = 65535;

/// Maximum number of local variable slots.
pub const MAX_LOCALS: usize = 65535;

/// Maximum number of user-defined functions.
pub const MAX_FUNCTIONS: usize = 65535;

/// Maximum code size in bytes.
pub const MAX_CODE: usize = 65535;

/// A single bytecode opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Op {
    /// Push `null` onto the stack.
    PushNull = 0x00,
    /// Push a boolean. Operand: 1 byte (0 = false, 1 = true).
    PushBool = 0x01,
    /// Push an i64. Operand: 8 bytes little-endian.
    PushInt = 0x02,
    /// Push a u64. Operand: 8 bytes little-endian.
    PushUInt = 0x03,
    /// Push an f64. Operand: 8 bytes (f64 bits, little-endian).
    PushFloat = 0x04,
    /// Push a string from the constant pool. Operand: u16 const index.
    PushString = 0x05,
    /// Push an IRI from the constant pool. Operand: u16 const index.
    PushIri = 0x06,
    /// Load a local variable. Operand: u16 slot index.
    LoadVar = 0x07,
    /// Store top of stack to a local variable. Operand: u16 slot index.
    StoreVar = 0x08,
    /// Pop and discard the top of stack.
    Pop = 0x09,
    /// Duplicate the top of stack.
    Dup = 0x0A,

    // ── Arithmetic ──────────────────────────────────────────────
    Add = 0x10,
    Sub = 0x11,
    Mul = 0x12,
    Div = 0x13,
    Rem = 0x14,
    Neg = 0x15,
    Not = 0x16,

    // ── Comparison ──────────────────────────────────────────────
    Eq = 0x17,
    Ne = 0x18,
    Lt = 0x19,
    Le = 0x1A,
    Gt = 0x1B,
    Ge = 0x1C,
    And = 0x1D,
    Or = 0x1E,

    // ── Control flow ────────────────────────────────────────────
    /// Unconditional jump. Operand: u16 code offset.
    Jump = 0x20,
    /// Jump if top of stack is falsy. Operand: u16 code offset. Pops the condition.
    JumpIfFalse = 0x21,
    /// Jump if top of stack is truthy. Operand: u16 code offset. Pops the condition.
    JumpIfTrue = 0x22,

    // ── Calls ───────────────────────────────────────────────────
    /// Call a host capability. Operands: u16 path const index, u8 arg count.
    CallHost = 0x30,
    /// Call a user-defined function. Operands: u16 function index, u8 arg count.
    CallUser = 0x31,
    /// Return from the current function. Pops the return value.
    Return = 0x32,
    /// Return null (void) from the current function.
    ReturnNull = 0x33,

    // ── Data structures ─────────────────────────────────────────
    /// Create a list from N stack elements. Operand: u16 count.
    MakeList = 0x40,
    /// Create a record from N key-value pairs. Operand: u16 pair count.
    /// Keys are string constants (u16 const index pushed before each value).
    MakeRecord = 0x41,
    /// Get a named member from a record. Operand: u16 name const index.
    GetMember = 0x42,
    /// Get an indexed element from a list. Pops index then list.
    GetIndex = 0x43,
    /// Unwrap a Try (Ok → value, Err → error). Operand: u16 catch offset.
    TryUnwrap = 0x44,
    /// Execute an effect expression (pop and discard, for side effects).
    Effect = 0x45,

    // ── Special ─────────────────────────────────────────────────
    /// Halt execution (end of code segment).
    Halt = 0xFF,
}

impl Op {
    /// Decode an opcode from a byte.
    pub fn from_byte(b: u8) -> Option<Self> {
        Some(match b {
            0x00 => Self::PushNull,
            0x01 => Self::PushBool,
            0x02 => Self::PushInt,
            0x03 => Self::PushUInt,
            0x04 => Self::PushFloat,
            0x05 => Self::PushString,
            0x06 => Self::PushIri,
            0x07 => Self::LoadVar,
            0x08 => Self::StoreVar,
            0x09 => Self::Pop,
            0x0A => Self::Dup,
            0x10 => Self::Add,
            0x11 => Self::Sub,
            0x12 => Self::Mul,
            0x13 => Self::Div,
            0x14 => Self::Rem,
            0x15 => Self::Neg,
            0x16 => Self::Not,
            0x17 => Self::Eq,
            0x18 => Self::Ne,
            0x19 => Self::Lt,
            0x1A => Self::Le,
            0x1B => Self::Gt,
            0x1C => Self::Ge,
            0x1D => Self::And,
            0x1E => Self::Or,
            0x20 => Self::Jump,
            0x21 => Self::JumpIfFalse,
            0x22 => Self::JumpIfTrue,
            0x30 => Self::CallHost,
            0x31 => Self::CallUser,
            0x32 => Self::Return,
            0x33 => Self::ReturnNull,
            0x40 => Self::MakeList,
            0x41 => Self::MakeRecord,
            0x42 => Self::GetMember,
            0x43 => Self::GetIndex,
            0x44 => Self::TryUnwrap,
            0x45 => Self::Effect,
            0xFF => Self::Halt,
            _ => return None,
        })
    }

    /// Number of operand bytes following this opcode (not counting the opcode itself).
    pub fn operand_size(self) -> usize {
        match self {
            Self::PushNull
            | Self::Pop
            | Self::Dup
            | Self::Add
            | Self::Sub
            | Self::Mul
            | Self::Div
            | Self::Rem
            | Self::Neg
            | Self::Not
            | Self::Eq
            | Self::Ne
            | Self::Lt
            | Self::Le
            | Self::Gt
            | Self::Ge
            | Self::And
            | Self::Or
            | Self::Return
            | Self::ReturnNull
            | Self::GetIndex
            | Self::Effect
            | Self::Halt => 0,
            Self::PushBool => 1,
            Self::PushInt | Self::PushUInt | Self::PushFloat => 8,
            Self::PushString
            | Self::PushIri
            | Self::LoadVar
            | Self::StoreVar
            | Self::Jump
            | Self::JumpIfFalse
            | Self::JumpIfTrue
            | Self::CallUser
            | Self::MakeList
            | Self::MakeRecord
            | Self::GetMember
            | Self::TryUnwrap => 2,
            Self::CallHost => 3, // u16 path + u8 argc
        }
    }
}

/// A constant value in the constant pool.
#[derive(Debug, Clone, PartialEq)]
pub enum Const {
    String(String),
    Iri(String),
}

/// Metadata for a compiled user-defined function.
#[derive(Debug, Clone, PartialEq)]
pub struct FuncMeta {
    /// Function name (for lookup by name).
    pub name: String,
    /// Number of parameters.
    pub param_count: u8,
    /// Number of local variable slots (params + locals).
    pub local_count: u16,
    /// Code offset where the function body begins.
    pub code_offset: u16,
    /// Budget steps (0 = no budget).
    pub budget_steps: u64,
}

/// A compiled bytecode chunk — the unit of compilation and execution.
#[derive(Debug, Clone, PartialEq)]
pub struct Chunk {
    /// Constant pool (strings, IRIs).
    pub constants: Vec<Const>,
    /// Bytecode instructions.
    pub code: Vec<u8>,
    /// User-defined function metadata.
    pub functions: Vec<FuncMeta>,
    /// Module-level local count (for top-level execution).
    pub top_locals: u16,
}

impl Chunk {
    /// Create an empty chunk.
    pub fn new() -> Self {
        Self {
            constants: Vec::new(),
            code: Vec::new(),
            functions: Vec::new(),
            top_locals: 0,
        }
    }

    /// Add a string constant and return its index.
    pub fn add_string(&mut self, s: &str) -> u16 {
        if let Some(idx) = self
            .constants
            .iter()
            .position(|c| matches!(c, Const::String(x) if x == s))
        {
            return idx as u16;
        }
        let idx = self.constants.len();
        self.constants.push(Const::String(s.to_string()));
        idx as u16
    }

    /// Add an IRI constant and return its index.
    pub fn add_iri(&mut self, s: &str) -> u16 {
        if let Some(idx) = self
            .constants
            .iter()
            .position(|c| matches!(c, Const::Iri(x) if x == s))
        {
            return idx as u16;
        }
        let idx = self.constants.len();
        self.constants.push(Const::Iri(s.to_string()));
        idx as u16
    }

    /// Find a function by name. Returns its index in `functions`.
    pub fn find_function(&self, name: &str) -> Option<u16> {
        self.functions
            .iter()
            .position(|f| f.name == name)
            .map(|i| i as u16)
    }

    /// Current code offset (where the next instruction will be emitted).
    pub fn current_offset(&self) -> u16 {
        self.code.len() as u16
    }

    /// Emit a raw byte.
    pub fn emit(&mut self, b: u8) {
        self.code.push(b);
    }

    /// Emit an opcode.
    pub fn emit_op(&mut self, op: Op) {
        self.emit(op as u8);
    }

    /// Emit a u16 operand (little-endian).
    pub fn emit_u16(&mut self, v: u16) {
        self.code.push((v & 0xFF) as u8);
        self.code.push((v >> 8) as u8);
    }

    /// Emit a u8 operand.
    pub fn emit_u8(&mut self, v: u8) {
        self.code.push(v);
    }

    /// Emit an i64 operand (little-endian).
    pub fn emit_i64(&mut self, v: i64) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    /// Emit a u64 operand (little-endian).
    pub fn emit_u64(&mut self, v: u64) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    /// Emit an f64 operand (little-endian bits).
    pub fn emit_f64(&mut self, v: f64) {
        self.code.extend_from_slice(&v.to_le_bytes());
    }

    /// Patch a u16 operand at the given code offset.
    pub fn patch_u16(&mut self, offset: usize, value: u16) {
        self.code[offset] = (value & 0xFF) as u8;
        self.code[offset + 1] = (value >> 8) as u8;
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn op_roundtrip() {
        for b in 0..=255u8 {
            if let Some(op) = Op::from_byte(b) {
                assert_eq!(op as u8, b, "opcode roundtrip failed for 0x{b:02X}");
            }
        }
    }

    #[test]
    fn chunk_add_string_dedup() {
        let mut c = Chunk::new();
        let a = c.add_string("hello");
        let b = c.add_string("hello");
        assert_eq!(a, b, "string constants should be deduplicated");
        let d = c.add_string("world");
        assert_ne!(a, d, "different strings should have different indices");
    }

    #[test]
    fn chunk_emit_and_patch() {
        let mut c = Chunk::new();
        c.emit_op(Op::Jump);
        let patch_off = c.code.len();
        c.emit_u16(0); // placeholder
        c.emit_op(Op::Halt);
        c.patch_u16(patch_off, 42);
        assert_eq!(c.code, vec![Op::Jump as u8, 42, 0, Op::Halt as u8]);
    }

    #[test]
    fn chunk_find_function() {
        let mut c = Chunk::new();
        c.functions.push(FuncMeta {
            name: "foo".into(),
            param_count: 2,
            local_count: 3,
            code_offset: 10,
            budget_steps: 1000,
        });
        assert_eq!(c.find_function("foo"), Some(0));
        assert_eq!(c.find_function("bar"), None);
    }
}
