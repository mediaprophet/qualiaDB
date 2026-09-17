//! `vibe-bc-0.1` bytecode compiler, VM, and binary codec.
//!
//! Stack-based bytecode for VibeScript 0.1. Compiles a checked `Program`
//! AST into a `Chunk`, which can be executed by the `Vm` or serialized
//! to/from bytes.
//!
//! ## Module layout
//!
//! - [`op`] — opcode definitions, `Chunk`, `Const`, `FuncMeta`
//! - [`compiler`] — AST → `Chunk` compiler
//! - [`vm`] — stack-based virtual machine
//! - [`codec`] — binary encode/decode of `Chunk`

pub mod codec;
pub mod compiler;
pub mod op;
pub mod vm;

pub use codec::{decode_chunk, encode_chunk, ChunkDecodeError};
pub use compiler::{compile, compile_expr, CompileError};
pub use op::{Chunk, Const, FuncMeta, Op, MAGIC, MAX_CODE, MAX_CONSTANTS, MAX_FUNCTIONS, VERSION};
pub use vm::{Vm, VmError};
