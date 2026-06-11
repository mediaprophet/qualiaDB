//! Semantic-to-QUBO boil-down compiler for zero-context quantum offloading.
//!
//! Strips DIDs and URIs into ephemeral local indices, emits linear biases and
//! quadratic coupler weights, then re-hydrates binary solutions back to Quins.

use crate::NQuin;

pub const OP_EMIT_WEIGHT: u8 = 0x50;
pub const MAX_QUBO_VARS: usize = 64;
pub const MAX_COUPLERS: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum QuboCompileError {
    VarBufferFull,
    CouplerBufferFull,
    IndexMapFull,
    ClassifiedEgress,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct QuboWeightEmit {
    pub var_a: u8,
    pub var_b: u8,
    pub weight: f32,
}

#[derive(Debug, Clone)]
pub struct QuboMatrix {
    pub num_vars: u8,
    pub linear: [f32; MAX_QUBO_VARS],
    pub couplers: [QuboWeightEmit; MAX_COUPLERS],
    pub coupler_count: usize,
    pub index_map: [(u64, u8); MAX_QUBO_VARS],
    pub index_count: usize,
}

impl Default for QuboMatrix {
    fn default() -> Self {
        Self {
            num_vars: 0,
            linear: [0.0; MAX_QUBO_VARS],
            couplers: [QuboWeightEmit {
                var_a: 0,
                var_b: 0,
                weight: 0.0,
            }; MAX_COUPLERS],
            coupler_count: 0,
            index_map: [(0, 0); MAX_QUBO_VARS],
            index_count: 0,
        }
    }
}

impl QuboMatrix {
    fn map_var(&mut self, entity_hash: u64) -> Result<u8, QuboCompileError> {
        for i in 0..self.index_count {
            if self.index_map[i].0 == entity_hash {
                return Ok(self.index_map[i].1);
            }
        }
        if self.index_count >= MAX_QUBO_VARS {
            return Err(QuboCompileError::IndexMapFull);
        }
        let idx = self.index_count as u8;
        self.index_map[self.index_count] = (entity_hash, idx);
        self.index_count += 1;
        if idx as usize + 1 > self.num_vars as usize {
            self.num_vars = idx + 1;
        }
        Ok(idx)
    }

    pub fn emit_linear(&mut self, var: u8, bias: f32) -> Result<(), QuboCompileError> {
        if var as usize >= MAX_QUBO_VARS {
            return Err(QuboCompileError::VarBufferFull);
        }
        self.linear[var as usize] += bias;
        Ok(())
    }

    pub fn emit_coupler(&mut self, a: u8, b: u8, weight: f32) -> Result<(), QuboCompileError> {
        if self.coupler_count >= MAX_COUPLERS {
            return Err(QuboCompileError::CouplerBufferFull);
        }
        self.couplers[self.coupler_count] = QuboWeightEmit {
            var_a: a,
            var_b: b,
            weight,
        };
        self.coupler_count += 1;
        Ok(())
    }

    pub fn wipe_index_map(&mut self) {
        for slot in &mut self.index_map {
            unsafe {
                std::ptr::write_volatile(&mut slot.0, 0);
            }
        }
        self.index_count = 0;
    }
}

/// Walk constraint Quins and compile a blind QUBO matrix.
pub fn compile_quins_to_qubo(
    quins: &[NQuin],
    out: &mut QuboMatrix,
) -> Result<(), QuboCompileError> {
    *out = QuboMatrix::default();
    for q in quins {
        if q.get_sensitivity_byte() == NQuin::SENSITIVITY_CLASSIFIED {
            return Err(QuboCompileError::ClassifiedEgress);
        }
        let subj = out.map_var(q.subject)?;
        let obj = out.map_var(q.object)?;
        let pred_low = (q.predicate & 0xFF) as u8;
        let weight = if pred_low == OP_EMIT_WEIGHT {
            decode_inline_weight(q.object)
        } else {
            penalty_from_predicate(pred_low)
        };
        if subj == obj {
            out.emit_linear(subj, weight)?;
        } else {
            out.emit_coupler(subj, obj, weight)?;
            out.emit_linear(subj, weight * 0.5)?;
            out.emit_linear(obj, weight * 0.5)?;
        }
    }
    Ok(())
}

/// VM opcode handler: push a float weight from the object register.
pub fn emit_weight_from_quin(
    quin: &NQuin,
    matrix: &mut QuboMatrix,
) -> Result<(), QuboCompileError> {
    let subj = matrix.map_var(quin.subject)?;
    let obj = matrix.map_var(quin.object)?;
    let w = decode_inline_weight(quin.object);
    if subj == obj {
        matrix.emit_linear(subj, w)
    } else {
        matrix.emit_coupler(subj, obj, w)
    }
}

fn decode_inline_weight(object: u64) -> f32 {
    let tag = (object >> 60) & 0x7;
    if tag == 0b010 {
        let scaled = (object & 0x0FFF_FFFF_FFFF_FFFF) as i64;
        (scaled as f32) / 1_000_000.0
    } else {
        let raw = (object & 0xFFFF) as i32;
        (raw as f32) / 100.0
    }
}

fn penalty_from_predicate(opcode: u8) -> f32 {
    match opcode {
        0x10 => 5.0,  // OP_OBLIGATE — violation penalty
        0x11 => -2.0, // OP_PERMIT — reward
        0x12 => 8.0,  // OP_FORBID — hard penalty
        _ => 1.0,
    }
}

/// Classical fallback: greedy energy minimization on small QUBO.
pub fn solve_classical(matrix: &QuboMatrix, assignment: &mut [u8; MAX_QUBO_VARS]) -> f32 {
    let n = matrix.num_vars as usize;
    for i in 0..n {
        assignment[i] = 0;
    }
    let mut improved = true;
    let mut energy = qubo_energy(matrix, assignment, n);
    while improved {
        improved = false;
        for i in 0..n {
            assignment[i] = 1 - assignment[i];
            let new_e = qubo_energy(matrix, assignment, n);
            if new_e < energy {
                energy = new_e;
                improved = true;
            } else {
                assignment[i] = 1 - assignment[i];
            }
        }
    }
    energy
}

fn qubo_energy(matrix: &QuboMatrix, assignment: &[u8; MAX_QUBO_VARS], n: usize) -> f32 {
    let mut e = 0.0f32;
    for i in 0..n {
        if assignment[i] == 1 {
            e += matrix.linear[i];
        }
    }
    for c in 0..matrix.coupler_count {
        let cw = matrix.couplers[c];
        let a = cw.var_a as usize;
        let b = cw.var_b as usize;
        if a < n && b < n && assignment[a] == 1 && assignment[b] == 1 {
            e += cw.weight;
        }
    }
    e
}

/// Re-hydrate a binary solution using the ephemeral index map.
pub fn rehydrate_solution(
    matrix: &mut QuboMatrix,
    assignment: &[u8; MAX_QUBO_VARS],
    out: &mut [NQuin],
) -> usize {
    let mut count = 0;
    for i in 0..matrix.index_count {
        if count >= out.len() {
            break;
        }
        let (entity, var) = matrix.index_map[i];
        let val = assignment[var as usize];
        let predicate = crate::q_hash("q42:quantumAssignment");
        let context = crate::q_hash("q42:rehydrated");
        let object = if val == 1 { 1 } else { 0 };
        let q = NQuin {
            subject: entity,
            predicate,
            object,
            context,
            metadata: 0xC000_0000_0000_0003,
            parity: entity ^ predicate ^ object ^ context,
        };
        out[count] = q;
        count += 1;
    }
    matrix.wipe_index_map();
    count
}

/// Pre-flight gate: reject prompts that signal classified quantum egress.
pub fn quantum_prompt_gate(prompt: &str) -> Option<&'static str> {
    let lower = prompt.to_lowercase();
    if lower.contains("classified") && (lower.contains("[qpu:") || lower.contains("quantum")) {
        return Some("FATAL: Cannot egress 0x02_CLASSIFIED assertions to a remote QPU.");
    }
    if lower.contains("sensitivitylabel") && lower.contains("0x02") {
        return Some("FATAL: Classified sensitivity label blocks QPU egress.");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_empty_quins() {
        let mut m = QuboMatrix::default();
        assert!(compile_quins_to_qubo(&[], &mut m).is_ok());
        assert_eq!(m.num_vars, 0);
    }

    #[test]
    fn classified_quin_blocks_egress() {
        let mut q = NQuin {
            subject: 1,
            predicate: 0,
            object: 2,
            context: 0,
            metadata: 0,
            parity: 0,
        };
        q.set_sensitivity_byte(NQuin::SENSITIVITY_CLASSIFIED);
        let mut m = QuboMatrix::default();
        assert_eq!(
            compile_quins_to_qubo(&[q], &mut m),
            Err(QuboCompileError::ClassifiedEgress)
        );
    }

    #[test]
    fn classical_solver_finds_low_energy() {
        let mut m = QuboMatrix::default();
        m.num_vars = 2;
        m.linear[0] = -1.0;
        m.linear[1] = -1.0;
        m.couplers[0] = QuboWeightEmit {
            var_a: 0,
            var_b: 1,
            weight: 2.0,
        };
        m.coupler_count = 1;
        let mut assign = [0u8; MAX_QUBO_VARS];
        let e = solve_classical(&m, &mut assign);
        assert!(e <= 0.0);
    }
}
