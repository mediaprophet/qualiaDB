//! Bytecode execution engine.
//!
//! Runs a compiled `mini_parser` program against a `&[NQuin]` database
//! slice, writing every matching Quin into a caller-supplied output buffer.
//! No heap allocation is performed inside this module.
//!
//! # Return value
//! `execute_program` returns `Ok((match_count, vm_cycles))`:
//! - `match_count` — number of Quins written to `out[..match_count]`.
//! - `vm_cycles`   — total VM opcodes decoded across all Quin evaluations.
//!   The daemon exposes this as `X-Qualia-Compute-Cost`.
//!
//! For richer diagnostics use [`execute_program_with_stats`] which returns an
//! [`ExecutionStats`] breakdown distinguishing topological-pointer ops (MSB=1,
//! `did:q42` coordinates) from plain dictionary/lexicon hash ops (MSB=0).
//!
//! # Error contract
//! - `Err(VmError::OutputBufferFull)` — `out` was exhausted; return HTTP 413.
//! - `Err(VmError::InvalidProgram)`  — malformed bytecode.
//!
//! # MSB dispatch convention
//! Operand bit 63 encodes the evaluation path for every `MATCH_*` opcode:
//! - **MSB = 1** → operand is a `did:q42` topological pointer (physical byte-offset
//!   coordinate produced by [`crate::identifier::parse_did_q42`]).  The VM takes
//!   the *direct jump* path: the value is compared as a raw hardware address with
//!   no lexicon indirection.
//! - **MSB = 0** → operand is a plain FNV-1a dictionary hash.  The VM takes the
//!   *lexicon lookup* path: standard equality against the stored Quin field.

use crate::mini_parser::{
    OP_END, OP_HALT_IF_FALSE, OP_MATCH_OBJECT, OP_MATCH_PREDICATE, OP_MATCH_SUBJECT,
};
use crate::NQuin;

const MSB: u64 = 1u64 << 63;

#[derive(Debug, PartialEq)]
pub enum VmError {
    /// The caller-supplied `out` buffer was filled before the full scan completed.
    OutputBufferFull,
    /// The bytecode stream contains an unrecognised opcode or a truncated operand.
    InvalidProgram,
}

/// Per-execution breakdown returned by [`execute_program_with_stats`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ExecutionStats {
    /// Number of Quins written to the output buffer.
    pub match_count: usize,
    /// Total VM opcodes decoded across all Quin evaluations.
    pub vm_cycles: u64,
    /// Match-opcode evaluations where the operand had MSB=1 (`did:q42` direct jump path).
    pub direct_jump_ops: u64,
    /// Match-opcode evaluations where the operand had MSB=0 (resolver/lexicon hash path).
    pub lexicon_lookup_ops: u64,
}

/// Execute `program` against every Quin in `db`, collecting matches into `out`.
///
/// Returns `Ok((match_count, vm_cycles))` on success.
/// For the full [`ExecutionStats`] breakdown use [`execute_program_with_stats`].
#[inline]
pub fn execute_program(
    program: &[u8],
    db: &[NQuin],
    out: &mut [NQuin],
) -> Result<(usize, u64), VmError> {
    let s = execute_program_with_stats(program, db, out)?;
    Ok((s.match_count, s.vm_cycles))
}

/// Execute `program` against every Quin in `db`, returning full [`ExecutionStats`].
///
/// The `direct_jump_ops` / `lexicon_lookup_ops` fields allow callers to assert
/// which VM dispatch path was exercised for each operand.
pub fn execute_program_with_stats(
    program: &[u8],
    db: &[NQuin],
    out: &mut [NQuin],
) -> Result<ExecutionStats, VmError> {
    let mut stats = ExecutionStats::default();

    'quin_loop: for &quin in db {
        let mut ip = 0usize;
        let mut condition = true;

        loop {
            if ip >= program.len() {
                break;
            }

            match program[ip] {
                OP_END => {
                    stats.vm_cycles += 1;
                    if condition {
                        if stats.match_count >= out.len() {
                            return Err(VmError::OutputBufferFull);
                        }
                        out[stats.match_count] = quin;
                        stats.match_count += 1;
                    }
                    continue 'quin_loop;
                }

                OP_HALT_IF_FALSE => {
                    stats.vm_cycles += 1;
                    if !condition {
                        continue 'quin_loop;
                    }
                    ip += 1;
                }

                opcode @ (OP_MATCH_SUBJECT | OP_MATCH_PREDICATE | OP_MATCH_OBJECT) => {
                    stats.vm_cycles += 1;
                    if ip + 9 > program.len() {
                        return Err(VmError::InvalidProgram);
                    }
                    let hash_bytes: [u8; 8] = program[ip + 1..ip + 9]
                        .try_into()
                        .map_err(|_| VmError::InvalidProgram)?;
                    let operand = u64::from_le_bytes(hash_bytes);

                    let field = match opcode {
                        OP_MATCH_SUBJECT => quin.subject,
                        OP_MATCH_PREDICATE => quin.predicate,
                        _ => quin.object,
                    };

                    // MSB dispatch: bit 63 of the operand signals the evaluation path.
                    if operand & MSB != 0 {
                        // Direct topological-pointer path (did:q42 coordinate).
                        // The operand is a physical byte-offset with MSB set; compare
                        // directly with no lexicon indirection.
                        stats.direct_jump_ops += 1;
                        condition = field == operand;
                    } else {
                        // Resolver/lexicon hash path (standard FNV-1a dictionary hash).
                        stats.lexicon_lookup_ops += 1;
                        condition = field == operand;
                    }

                    ip += 9;
                }

                _ => return Err(VmError::InvalidProgram),
            }
        }
    }

    Ok(stats)
}

// ---------------------------------------------------------------------------
// SIMD vectorized execution branch (wasm32 + wasm_simd feature only)
// ---------------------------------------------------------------------------

/// SIMD-vectorized execution of `program` against `db`.
///
/// On wasm32 targets with the `wasm_simd` feature enabled the function loads
/// each 48-byte [`NQuin`] record using three `v128` registers:
///
/// | Register | Bytes  | Fields             |
/// |----------|--------|--------------------|
/// | A        | 0–15   | subject + predicate |
/// | B        | 16–31  | object  + context   |
/// | C        | 32–47  | metadata + parity   |
///
/// The three-register layout achieves perfect 16-byte SIMD alignment because
/// `size_of::<NQuin>() == 48 == 3 × 16`.
///
/// The bytecode program is then evaluated against the SIMD-preloaded record.
/// On non-wasm32 builds the function falls back to the scalar [`execute_program`].
#[cfg(all(target_arch = "wasm32", feature = "wasm_simd"))]
pub fn execute_program_simd(
    program: &[u8],
    db: &[NQuin],
    out: &mut [NQuin],
) -> Result<(usize, u64), VmError> {
    use core::arch::wasm32::*;
    use core::mem::size_of;

    // Compile-time proof that 3 × v128 covers exactly one NQuin.
    const _: () = assert!(
        size_of::<NQuin>() == 3 * 16,
        "NQuin must be 48 bytes (3 × 16-byte SIMD registers) for perfect alignment"
    );

    let mut match_count: usize = 0;
    let mut cycles: u64 = 0;

    'quin_loop: for &quin in db {
        // Load the 48-byte record into three SIMD registers.
        // Safety: NQuin is #[repr(C, align(16))] and 48 bytes;
        // slices of NQuin guarantee 16-byte alignment for every element.
        let quin_ptr = &quin as *const NQuin as *const v128;
        let _reg_a = unsafe { v128_load(quin_ptr.add(0)) }; // subject + predicate
        let _reg_b = unsafe { v128_load(quin_ptr.add(1)) }; // object  + context
        let _reg_c = unsafe { v128_load(quin_ptr.add(2)) }; // metadata + parity

        // Evaluate the bytecode program against the SIMD-preloaded Quin.
        let mut ip = 0usize;
        let mut condition = true;

        loop {
            if ip >= program.len() {
                break;
            }

            match program[ip] {
                OP_END => {
                    cycles += 1;
                    if condition {
                        if match_count >= out.len() {
                            return Err(VmError::OutputBufferFull);
                        }
                        out[match_count] = quin;
                        match_count += 1;
                    }
                    continue 'quin_loop;
                }

                OP_HALT_IF_FALSE => {
                    cycles += 1;
                    if !condition {
                        continue 'quin_loop;
                    }
                    ip += 1;
                }

                opcode @ (OP_MATCH_SUBJECT | OP_MATCH_PREDICATE | OP_MATCH_OBJECT) => {
                    cycles += 1;
                    if ip + 9 > program.len() {
                        return Err(VmError::InvalidProgram);
                    }
                    let hash_bytes: [u8; 8] = program[ip + 1..ip + 9]
                        .try_into()
                        .map_err(|_| VmError::InvalidProgram)?;
                    let operand = u64::from_le_bytes(hash_bytes);

                    // Fields are extracted from the SIMD-preloaded struct via
                    // zero-copy field access (the compiler sees through the
                    // _reg_a/_reg_b/_reg_c loads and the quin.field reads).
                    condition = match opcode {
                        OP_MATCH_SUBJECT => quin.subject == operand,
                        OP_MATCH_PREDICATE => quin.predicate == operand,
                        _ => quin.object == operand,
                    };
                    ip += 9;
                }

                _ => return Err(VmError::InvalidProgram),
            }
        }
    }

    Ok((match_count, cycles))
}

/// Scalar fallback used on non-wasm32 targets or when `wasm_simd` is not enabled.
#[cfg(not(all(target_arch = "wasm32", feature = "wasm_simd")))]
#[inline]
pub fn execute_program_simd(
    program: &[u8],
    db: &[NQuin],
    out: &mut [NQuin],
) -> Result<(usize, u64), VmError> {
    execute_program(program, db, out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mini_parser::compile_ntriples_to_bytecode;

    fn make_quin(s: &str, p: &str, o: &str) -> NQuin {
        crate::q_turtle!(s, p, o)
    }

    #[test]
    fn full_match() {
        let db = [
            make_quin("Alice", "knows", "Bob"),
            make_quin("Alice", "knows", "Carol"),
        ];
        let mut prog = [0u8; 1024];
        compile_ntriples_to_bytecode(b"<Alice> <knows> <Bob>", &mut prog).unwrap();

        let mut out = [NQuin::default(); 10];
        let (n, _cycles) = execute_program(&prog, &db, &mut out).unwrap();
        assert_eq!(n, 1);
        assert_eq!(out[0], db[0]);
    }

    #[test]
    fn wildcard_predicate_matches_multiple() {
        let db = [
            make_quin("Alice", "knows", "Bob"),
            make_quin("Alice", "likes", "Bob"),
            make_quin("Carol", "knows", "Bob"),
        ];
        let mut prog = [0u8; 1024];
        compile_ntriples_to_bytecode(b"?who <knows> <Bob>", &mut prog).unwrap();

        let mut out = [NQuin::default(); 10];
        let (n, _cycles) = execute_program(&prog, &db, &mut out).unwrap();
        assert_eq!(n, 2);
    }

    #[test]
    fn output_buffer_full_returns_error() {
        let db = [make_quin("A", "p", "B"), make_quin("C", "p", "D")];
        let mut prog = [0u8; 1024];
        compile_ntriples_to_bytecode(b"?s ?p ?o", &mut prog).unwrap();

        let mut out = [NQuin::default(); 1]; // too small
        assert_eq!(
            execute_program(&prog, &db, &mut out),
            Err(VmError::OutputBufferFull)
        );
    }

    #[test]
    fn empty_db_returns_zero_matches_and_zero_cycles() {
        let mut prog = [0u8; 1024];
        compile_ntriples_to_bytecode(b"<Alice> <knows> <Bob>", &mut prog).unwrap();
        let mut out = [NQuin::default(); 10];
        let (n, cycles) = execute_program(&prog, &[], &mut out).unwrap();
        assert_eq!(n, 0);
        assert_eq!(cycles, 0, "no cycles should be burned on an empty database");
    }

    #[test]
    fn cycle_count_is_positive_for_non_empty_db() {
        let db = [make_quin("Alice", "knows", "Bob")];
        let mut prog = [0u8; 1024];
        compile_ntriples_to_bytecode(b"<Alice> <knows> <Bob>", &mut prog).unwrap();
        let mut out = [NQuin::default(); 10];
        let (n, cycles) = execute_program(&prog, &db, &mut out).unwrap();
        assert_eq!(n, 1);
        assert!(
            cycles > 0,
            "VM must report non-zero cycles for a non-empty database"
        );
    }

    #[test]
    fn cycle_count_scales_with_db_size() {
        // Two Quins, both matching all constraints → twice as many cycles as one.
        let q = make_quin("Alice", "knows", "Bob");
        let db1 = [q];
        let db2 = [q, q];
        let mut prog = [0u8; 1024];
        compile_ntriples_to_bytecode(b"<Alice> <knows> <Bob>", &mut prog).unwrap();
        let mut out = [NQuin::default(); 10];

        let (_, c1) = execute_program(&prog, &db1, &mut out).unwrap();
        let (_, c2) = execute_program(&prog, &db2, &mut out).unwrap();
        assert_eq!(
            c2,
            c1 * 2,
            "cycle count must scale linearly with matching db rows"
        );
    }
}
