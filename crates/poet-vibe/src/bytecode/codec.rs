//! Binary codec for `vibe-bc-0.1` chunks.
//!
//! Serializes a `Chunk` to/from a compact binary format suitable for
//! storage, transmission, or caching. The format is:
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │ Header                                                       │
//! │  magic         4 bytes   "VBC1"                              │
//! │  version       2 bytes   u16 LE                               │
//! │  top_locals    2 bytes   u16 LE                               │
//! │  const_count   2 bytes   u16 LE                               │
//! │  func_count    2 bytes   u16 LE                               │
//! │  code_len      2 bytes   u16 LE                               │
//! ├──────────────────────────────────────────────────────────────┤
//! │ Constants                                                    │
//! │  For each constant:                                          │
//! │    tag       1 byte    0=String, 1=Iri                        │
//! │    len       2 bytes   u16 LE (byte length, UTF-8)            │
//! │    data      len bytes  UTF-8 string                          │
//! ├──────────────────────────────────────────────────────────────┤
//! │ Functions                                                    │
//! │  For each function:                                          │
//! │    name_len   1 byte                                         │
//! │    name       name_len bytes                                 │
//! │    param_ct   1 byte                                         │
//! │    local_ct   2 bytes   u16 LE                                │
//! │    code_off   2 bytes   u16 LE                                │
//! │    budget     8 bytes   u64 LE                                │
//! ├──────────────────────────────────────────────────────────────┤
//! │ Code                                                         │
//! │  raw bytes   code_len bytes                                  │
//! └──────────────────────────────────────────────────────────────┘
//! ```

use crate::bytecode::op::{
    Chunk, Const, FuncMeta, MAGIC, MAX_CODE, MAX_CONSTANTS, MAX_FUNCTIONS, VERSION,
};

/// Codec error.
#[derive(Debug, Clone, PartialEq)]
pub enum ChunkDecodeError {
    /// Data too short for even the header.
    TooShort,
    /// Magic bytes don't match.
    BadMagic([u8; 4]),
    /// Version mismatch.
    BadVersion(u16),
    /// Truncated data (declared length exceeds remaining bytes).
    Truncated,
    /// Invalid constant tag.
    InvalidConstTag(u8),
    /// Too many constants.
    TooManyConstants,
    /// Too many functions.
    TooManyFunctions,
    /// Code segment too large.
    CodeTooLarge,
    /// Invalid string length.
    InvalidStringLength(u16),
    /// Invalid function name length.
    InvalidNameLength(u8),
}

impl std::fmt::Display for ChunkDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort => write!(f, "chunk: data too short for header"),
            Self::BadMagic(m) => write!(
                f,
                "chunk: bad magic {:02X}{:02X}{:02X}{:02X}",
                m[0], m[1], m[2], m[3]
            ),
            Self::BadVersion(v) => write!(f, "chunk: unsupported version {v}"),
            Self::Truncated => write!(f, "chunk: truncated data"),
            Self::InvalidConstTag(t) => write!(f, "chunk: invalid constant tag {t}"),
            Self::TooManyConstants => write!(f, "chunk: too many constants"),
            Self::TooManyFunctions => write!(f, "chunk: too many functions"),
            Self::CodeTooLarge => write!(f, "chunk: code segment too large"),
            Self::InvalidStringLength(n) => write!(f, "chunk: invalid string length {n}"),
            Self::InvalidNameLength(n) => write!(f, "chunk: invalid function name length {n}"),
        }
    }
}

impl std::error::Error for ChunkDecodeError {}

/// Encode a `Chunk` into binary bytes.
pub fn encode_chunk(chunk: &Chunk) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64 + chunk.code.len());

    // Header
    buf.extend_from_slice(&MAGIC);
    buf.extend_from_slice(&VERSION.to_le_bytes());
    buf.extend_from_slice(&chunk.top_locals.to_le_bytes());
    buf.extend_from_slice(&(chunk.constants.len() as u16).to_le_bytes());
    buf.extend_from_slice(&(chunk.functions.len() as u16).to_le_bytes());
    buf.extend_from_slice(&(chunk.code.len() as u16).to_le_bytes());

    // Constants
    for c in &chunk.constants {
        match c {
            Const::String(s) => {
                buf.push(0);
                let bytes = s.as_bytes();
                buf.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
                buf.extend_from_slice(bytes);
            }
            Const::Iri(s) => {
                buf.push(1);
                let bytes = s.as_bytes();
                buf.extend_from_slice(&(bytes.len() as u16).to_le_bytes());
                buf.extend_from_slice(bytes);
            }
        }
    }

    // Functions
    for f in &chunk.functions {
        let name_bytes = f.name.as_bytes();
        buf.push(name_bytes.len() as u8);
        buf.extend_from_slice(name_bytes);
        buf.push(f.param_count);
        buf.extend_from_slice(&f.local_count.to_le_bytes());
        buf.extend_from_slice(&f.code_offset.to_le_bytes());
        buf.extend_from_slice(&f.budget_steps.to_le_bytes());
    }

    // Code
    buf.extend_from_slice(&chunk.code);

    buf
}

/// Decode a `Chunk` from binary bytes.
pub fn decode_chunk(data: &[u8]) -> Result<Chunk, ChunkDecodeError> {
    let mut pos = 0;

    // Header
    if data.len() < 14 {
        return Err(ChunkDecodeError::TooShort);
    }

    let mut magic = [0u8; 4];
    magic.copy_from_slice(&data[pos..pos + 4]);
    pos += 4;
    if magic != MAGIC {
        return Err(ChunkDecodeError::BadMagic(magic));
    }

    let version = u16::from_le_bytes([data[pos], data[pos + 1]]);
    pos += 2;
    if version != VERSION {
        return Err(ChunkDecodeError::BadVersion(version));
    }

    let top_locals = u16::from_le_bytes([data[pos], data[pos + 1]]);
    pos += 2;

    let const_count = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    if const_count > MAX_CONSTANTS {
        return Err(ChunkDecodeError::TooManyConstants);
    }

    let func_count = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    if func_count > MAX_FUNCTIONS {
        return Err(ChunkDecodeError::TooManyFunctions);
    }

    let code_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    if code_len > MAX_CODE {
        return Err(ChunkDecodeError::CodeTooLarge);
    }

    // Constants
    let mut constants = Vec::with_capacity(const_count);
    for _ in 0..const_count {
        if pos >= data.len() {
            return Err(ChunkDecodeError::Truncated);
        }
        let tag = data[pos];
        pos += 1;
        if pos + 2 > data.len() {
            return Err(ChunkDecodeError::Truncated);
        }
        let len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;
        if pos + len > data.len() {
            return Err(ChunkDecodeError::Truncated);
        }
        let s = std::str::from_utf8(&data[pos..pos + len])
            .map_err(|_| ChunkDecodeError::InvalidStringLength(len as u16))?
            .to_string();
        pos += len;
        constants.push(match tag {
            0 => Const::String(s),
            1 => Const::Iri(s),
            _ => return Err(ChunkDecodeError::InvalidConstTag(tag)),
        });
    }

    // Functions
    let mut functions = Vec::with_capacity(func_count);
    for _ in 0..func_count {
        if pos >= data.len() {
            return Err(ChunkDecodeError::Truncated);
        }
        let name_len = data[pos] as usize;
        pos += 1;
        if name_len == 0 {
            return Err(ChunkDecodeError::InvalidNameLength(0));
        }
        if pos + name_len > data.len() {
            return Err(ChunkDecodeError::Truncated);
        }
        let name = std::str::from_utf8(&data[pos..pos + name_len])
            .map_err(|_| ChunkDecodeError::InvalidNameLength(name_len as u8))?
            .to_string();
        pos += name_len;

        if pos + 13 > data.len() {
            return Err(ChunkDecodeError::Truncated);
        }
        let param_count = data[pos];
        pos += 1;
        let local_count = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        let code_offset = u16::from_le_bytes([data[pos], data[pos + 1]]);
        pos += 2;
        let budget_steps = u64::from_le_bytes([
            data[pos],
            data[pos + 1],
            data[pos + 2],
            data[pos + 3],
            data[pos + 4],
            data[pos + 5],
            data[pos + 6],
            data[pos + 7],
        ]);
        pos += 8;

        functions.push(FuncMeta {
            name,
            param_count,
            local_count,
            code_offset,
            budget_steps,
        });
    }

    // Code
    if pos + code_len > data.len() {
        return Err(ChunkDecodeError::Truncated);
    }
    let code = data[pos..pos + code_len].to_vec();

    Ok(Chunk {
        constants,
        code,
        functions,
        top_locals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bind::MockHost;
    use crate::budget::Budget;
    use crate::bytecode::compiler::compile_expr;
    use crate::bytecode::vm::Vm;
    use crate::parse::parse_cell;

    #[test]
    fn encode_decode_roundtrip_simple() {
        let expr = parse_cell("= 1 + 2 * 3").expect("parse");
        let chunk = compile_expr(&expr).expect("compile");
        let bytes = encode_chunk(&chunk);
        let decoded = decode_chunk(&bytes).expect("decode");
        assert_eq!(chunk, decoded);
    }

    #[test]
    fn encode_decode_roundtrip_strings() {
        let expr = parse_cell(r#"= "hello world""#).expect("parse");
        let chunk = compile_expr(&expr).expect("compile");
        let bytes = encode_chunk(&chunk);
        let decoded = decode_chunk(&bytes).expect("decode");
        assert_eq!(chunk, decoded);
    }

    #[test]
    fn encode_decode_roundtrip_list() {
        let expr = parse_cell("= [1, 2, 3, 4, 5]").expect("parse");
        let chunk = compile_expr(&expr).expect("compile");
        let bytes = encode_chunk(&chunk);
        let decoded = decode_chunk(&bytes).expect("decode");
        assert_eq!(chunk, decoded);
    }

    #[test]
    fn encode_decode_roundtrip_record() {
        let expr = parse_cell("= { x: 1, y: 2, z: 3 }").expect("parse");
        let chunk = compile_expr(&expr).expect("compile");
        let bytes = encode_chunk(&chunk);
        let decoded = decode_chunk(&bytes).expect("decode");
        assert_eq!(chunk, decoded);
    }

    #[test]
    fn encode_decode_roundtrip_functions() {
        use crate::bytecode::compiler::compile;
        use crate::parse::parse_program;
        let src =
            "fn double(x: i64) -> i64 { return x + x; } fn main() -> i64 { return double(21); }";
        let prog = parse_program(src).expect("parse");
        let chunk = compile(&prog).expect("compile");
        let bytes = encode_chunk(&chunk);
        let decoded = decode_chunk(&bytes).expect("decode");
        assert_eq!(chunk, decoded);
    }

    #[test]
    fn decode_bad_magic() {
        let mut bad = vec![0u8; 14];
        bad[0] = b'X';
        let err = decode_chunk(&bad).expect_err("should fail");
        assert!(matches!(err, ChunkDecodeError::BadMagic(_)));
    }

    #[test]
    fn decode_bad_version() {
        let mut buf = Vec::new();
        buf.extend_from_slice(&MAGIC);
        buf.extend_from_slice(&999u16.to_le_bytes()); // bad version
        buf.extend_from_slice(&0u16.to_le_bytes()); // top_locals
        buf.extend_from_slice(&0u16.to_le_bytes()); // const_count
        buf.extend_from_slice(&0u16.to_le_bytes()); // func_count
        buf.extend_from_slice(&0u16.to_le_bytes()); // code_len
        let err = decode_chunk(&buf).expect_err("should fail");
        assert!(matches!(err, ChunkDecodeError::BadVersion(999)));
    }

    #[test]
    fn decode_too_short() {
        let err = decode_chunk(&[0u8; 3]).expect_err("should fail");
        assert!(matches!(err, ChunkDecodeError::TooShort));
    }

    #[test]
    fn decode_truncated() {
        let expr = parse_cell("= 1 + 2").expect("parse");
        let chunk = compile_expr(&expr).expect("compile");
        let mut bytes = encode_chunk(&chunk);
        bytes.truncate(bytes.len() - 1); // truncate last byte
        let err = decode_chunk(&bytes).expect_err("should fail");
        assert!(matches!(err, ChunkDecodeError::Truncated));
    }

    #[test]
    fn vm_executes_decoded_chunk() {
        // Compile, encode, decode, then execute — verifying the full pipeline.
        let expr = parse_cell("= 6 * 7").expect("parse");
        let chunk = compile_expr(&expr).expect("compile");
        let bytes = encode_chunk(&chunk);
        let decoded = decode_chunk(&bytes).expect("decode");

        let mut host = MockHost::default();
        let mut vm = Vm::new(&decoded, &mut host, Budget::default());
        let result = vm.run().expect("vm run");
        assert_eq!(result, crate::value::Value::I64(42));
    }

    #[test]
    fn vm_executes_decoded_function() {
        use crate::bytecode::compiler::compile;
        use crate::parse::parse_program;
        use crate::value::Value;

        // Note: recursion is not supported in the 0.1 VM (no self-call).
        // Use iterative version instead.
        let src = "fn fact(n: i64) budget(steps: 10000) -> i64 { let r = 1; let i = 1; while i <= n { r = r * i; i = i + 1; } return r; }";
        let prog = parse_program(src).expect("parse");
        let chunk = compile(&prog).expect("compile");
        let bytes = encode_chunk(&chunk);
        let decoded = decode_chunk(&bytes).expect("decode");

        let mut host = MockHost::default();
        let mut vm = Vm::new(&decoded, &mut host, Budget::default());
        vm.run().expect("preamble");

        let idx = decoded.find_function("fact").expect("fact exists");
        let result = vm.call_function(idx, &[Value::I64(5)]).expect("call");
        assert_eq!(result, Value::I64(120)); // 5! = 120
    }
}
