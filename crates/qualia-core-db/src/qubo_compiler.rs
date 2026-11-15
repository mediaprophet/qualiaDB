//! QUBO (Quadratic Unconstrained Binary Optimization) compiler for quantum annealing.

use crate::NQuin;

pub const MAX_QUBO_VARS: usize = 128;

/// QUBO matrix representation for quantum annealing
#[derive(Debug, Clone)]
pub struct QuboMatrix {
    /// Linear terms (diagonal)
    pub linear: Vec<f64>,
    /// Quadratic terms (off-diagonal)
    pub quadratic: Vec<(usize, usize, f64)>,
    /// Number of variables
    pub num_vars: usize,
    /// Number of couplers
    pub coupler_count: usize,
    /// Coupler connections
    pub couplers: Vec<Coupler>,
    /// Index map for variable mapping
    pub index_map: Vec<(u64, u64)>,
    /// Index count
    pub index_count: usize,
}

/// Coupler connection with weights
#[derive(Debug, Clone)]
pub struct Coupler {
    pub var_a: usize,
    pub var_b: usize,
    pub weight: f64,
}

impl Default for QuboMatrix {
    fn default() -> Self {
        Self::new(MAX_QUBO_VARS)
    }
}

impl QuboMatrix {
    /// Create a new QUBO matrix
    pub fn new(num_vars: usize) -> Self {
        Self {
            linear: vec![0.0; num_vars],
            quadratic: Vec::new(),
            num_vars,
            coupler_count: 0,
            couplers: Vec::new(),
            index_map: vec![(0, 0); num_vars],
            index_count: 0,
        }
    }

    /// Set a linear term
    pub fn set_linear(&mut self, var: usize, value: f64) {
        if var < self.linear.len() {
            self.linear[var] = value;
        }
    }

    /// Set a quadratic term
    pub fn set_quadratic(&mut self, var1: usize, var2: usize, value: f64) {
        if var1 < self.linear.len() && var2 < self.linear.len() {
            self.coupler_count += 1;
            self.couplers.push(Coupler {
                var_a: var1,
                var_b: var2,
                weight: value,
            });
            self.quadratic.push((var1, var2, value));
        }
    }

    /// Emit a coupler
    pub fn emit_coupler(&mut self, var_a: usize, var_b: usize, weight: f64) {
        self.couplers.push(Coupler {
            var_a,
            var_b,
            weight,
        });
        self.coupler_count += 1;
    }
}

/// Calculates the exact QUBO objective energy for a given binary assignment:
/// E(x) = sum_i linear[i] * x[i] + sum_(i, j, J) J * x[i] * x[j]
pub fn calculate_energy(matrix: &QuboMatrix, assignment: &[u8]) -> f64 {
    let mut energy = 0.0;
    let n = matrix.num_vars.min(assignment.len());
    for i in 0..n {
        if assignment[i] == 1 {
            energy += matrix.linear[i];
        }
    }
    for coupler in &matrix.couplers {
        if coupler.var_a < n && coupler.var_b < n {
            if assignment[coupler.var_a] == 1 && assignment[coupler.var_b] == 1 {
                energy += coupler.weight;
            }
        }
    }
    energy
}

/// Classical QUBO solver using Simulated Annealing over linear and quadratic couplers.
pub fn solve_classical(matrix: &QuboMatrix, assignment: &mut [u8]) -> f32 {
    let n = matrix.num_vars.min(assignment.len());
    if n == 0 {
        return 0.0;
    }

    // 1. Initial assignment: greedy based on linear terms
    for i in 0..n {
        assignment[i] = if matrix.linear[i] < 0.0 { 1 } else { 0 };
    }

    let mut current_energy = calculate_energy(matrix, &assignment[..n]);
    let mut best_assignment = assignment[..n].to_vec();
    let mut best_energy = current_energy;

    // 2. Simulated Annealing schedule
    let mut t = 10.0f64;
    let t_min = 0.001f64;
    let alpha = 0.95f64;
    let sweeps_per_temp = 50;
    let mut lcg_state: u64 = 0x5155_B001_DEAD_BEEF;

    while t > t_min {
        for _ in 0..sweeps_per_temp {
            lcg_state = lcg_state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let var = ((lcg_state >> 32) as usize) % n;

            let old_val = assignment[var];
            let new_val = 1 - old_val;

            let sign: f64 = if new_val == 1 { 1.0 } else { -1.0 };
            let mut delta = sign * matrix.linear[var];

            for coupler in &matrix.couplers {
                if coupler.var_a == var && coupler.var_b < n {
                    if assignment[coupler.var_b] == 1 {
                        delta += sign * coupler.weight;
                    }
                } else if coupler.var_b == var && coupler.var_a < n {
                    if assignment[coupler.var_a] == 1 {
                        delta += sign * coupler.weight;
                    }
                }
            }

            let accept = if delta <= 0.0 {
                true
            } else {
                lcg_state = lcg_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let rand_f = ((lcg_state >> 11) as f64) / ((1u64 << 53) as f64);
                (-delta / t).exp() > rand_f
            };

            if accept {
                assignment[var] = new_val;
                current_energy += delta;
                if current_energy < best_energy {
                    best_energy = current_energy;
                    best_assignment.copy_from_slice(&assignment[..n]);
                }
            }
        }
        t *= alpha;
    }

    assignment[..n].copy_from_slice(&best_assignment);
    best_energy as f32
}

/// Compiles Quins to a QUBO matrix, mapping subjects to variable indices
/// and relationships/couplers to quadratic terms.
pub fn compile_quins_to_qubo(quins: &[NQuin], matrix: &mut QuboMatrix) -> Result<(), String> {
    for quin in quins {
        if matrix.index_count >= matrix.num_vars {
            break;
        }
        let subj = quin.subject;
        let pos = matrix.index_map[..matrix.index_count]
            .iter()
            .position(|&(s, _)| s == subj);
        let var_idx = match pos {
            Some(idx) => idx,
            None => {
                let idx = matrix.index_count;
                matrix.index_map[idx] = (subj, quin.object);
                matrix.index_count += 1;
                idx
            }
        };

        let weight = if (quin.object & 0x7FFF_FFFF) != 0 {
            (quin.object as i32) as f64 / 1000.0
        } else {
            1.0
        };

        let opcode = (quin.predicate & 0xFF) as u8;
        if opcode == 0x10 {
            matrix.set_linear(var_idx, matrix.linear[var_idx] - weight);
        } else if opcode == 0x12 {
            matrix.set_linear(var_idx, matrix.linear[var_idx] + weight);
        } else {
            matrix.set_linear(var_idx, matrix.linear[var_idx] + (weight * 0.1));
        }

        if quin.context != 0 {
            if let Some(other_idx) = matrix.index_map[..matrix.index_count]
                .iter()
                .position(|&(s, _)| s == quin.context)
            {
                if var_idx != other_idx {
                    matrix.set_quadratic(var_idx, other_idx, weight * 0.5);
                }
            }
        }
    }
    Ok(())
}

/// Rehydrates solution from assignment back into Quins with original subject URIs from index_map.
pub fn rehydrate_solution(matrix: &mut QuboMatrix, assignment: &[u8], out: &mut [NQuin]) -> usize {
    let mut count = 0;
    for (i, &val) in assignment.iter().enumerate().take(out.len()) {
        if i < matrix.index_count && count < out.len() {
            let (subj, obj) = matrix.index_map[i];
            let predicate = if val == 1 {
                crate::q_hash("q42:selected")
            } else {
                crate::q_hash("q42:unselected")
            };
            out[count] = NQuin {
                subject: subj,
                predicate,
                object: obj,
                context: 0,
                metadata: val as u64,
                parity: subj ^ predicate ^ obj,
            };
            count += 1;
        }
    }
    count
}

/// Scrub personal URIs from index_map to ensure no personal data leaks
pub fn scrub_metadata(matrix: &mut QuboMatrix) {
    matrix.index_map.clear();
    matrix.index_count = 0;
}

/// Serialize QuboMatrix to multi-dimensional HDF5-like format
pub fn serialize_matrix(matrix: &QuboMatrix) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"HDF5_Q42_MAGIC");
    bytes.extend_from_slice(&(matrix.num_vars as u64).to_le_bytes());
    bytes.extend_from_slice(&(matrix.coupler_count as u64).to_le_bytes());
    for &val in &matrix.linear {
        bytes.extend_from_slice(&val.to_le_bytes());
    }
    for coupler in &matrix.couplers {
        bytes.extend_from_slice(&(coupler.var_a as u64).to_le_bytes());
        bytes.extend_from_slice(&(coupler.var_b as u64).to_le_bytes());
        bytes.extend_from_slice(&coupler.weight.to_le_bytes());
    }
    bytes
}

/// Publish a solved matrix to the WebTorrent Commons
#[cfg(not(target_arch = "wasm32"))]
pub fn publish_to_commons(
    matrix: &mut QuboMatrix,
    storage_path: &std::path::Path,
) -> Result<String, String> {
    scrub_metadata(matrix);
    let bytes = serialize_matrix(matrix);

    let commons_dir = storage_path.join("commons");
    std::fs::create_dir_all(&commons_dir).map_err(|e| e.to_string())?;

    let file_path = commons_dir.join("quantum_cache.q42");
    std::fs::write(&file_path, bytes).map_err(|e| e.to_string())?;

    let info_hash = crate::webtorrent_seeder::sha1_file(&file_path)?;
    let req = crate::webtorrent_seeder::RegisterSeedRequest {
        info_hash: info_hash.clone(),
        file_path: file_path.to_str().unwrap().to_string(),
        display_name: "quantum_cache.q42".to_string(),
        ontology_id: "quantum_commons".to_string(),
        bandwidth_limit_kbps: 1024,
        commons_asserted: true,
    };

    // Check if there is an existing seed and deprecate it
    if let Some(_existing) = crate::webtorrent_seeder::lookup_seed(&info_hash) {
        crate::webtorrent_seeder::deprecate_seed(&info_hash);
    }

    crate::webtorrent_seeder::register_seed(req)?;
    Ok(crate::webtorrent_seeder::build_magnet_uri(
        &info_hash,
        "quantum_cache.q42",
        4242,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qubo_simulated_annealing() {
        let mut matrix = QuboMatrix::new(2);
        // Objective: E(x0, x1) = -2*x0 - 2*x1 + 5*x0*x1
        // x = (0,0): E = 0
        // x = (1,0): E = -2
        // x = (0,1): E = -2
        // x = (1,1): E = -2 - 2 + 5 = 1
        // Minimum energy is -2 (either (1,0) or (0,1))
        matrix.set_linear(0, -2.0);
        matrix.set_linear(1, -2.0);
        matrix.set_quadratic(0, 1, 5.0);

        let mut assignment = [0u8; 2];
        let energy = solve_classical(&matrix, &mut assignment);
        assert!((energy - (-2.0)).abs() < 1e-4);
        assert!((assignment[0] == 1 && assignment[1] == 0) || (assignment[0] == 0 && assignment[1] == 1));
    }
}
