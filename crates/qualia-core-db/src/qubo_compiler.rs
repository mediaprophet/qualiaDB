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

/// Classical QUBO solver (placeholder - actual implementation would use simulated annealing or other algorithms)
pub fn solve_classical(matrix: &QuboMatrix, assignment: &mut [u8]) -> f32 {
    // Placeholder: simple greedy assignment
    let mut energy = 0.0f32;
    for i in 0..matrix.num_vars.min(assignment.len()) {
        assignment[i] = if matrix.linear[i] > 0.0 { 0 } else { 1 };
        energy += matrix.linear[i] as f32 * assignment[i] as f32;
    }
    energy
}

/// Compile Quins to QUBO matrix (placeholder)
pub fn compile_quins_to_qubo(quins: &[NQuin], matrix: &mut QuboMatrix) -> Result<(), String> {
    // Placeholder: simple conversion
    for quin in quins.iter().take(MAX_QUBO_VARS) {
        let var = quin.object as usize % MAX_QUBO_VARS;
        matrix.set_linear(var, quin.predicate as f64);
    }
    Ok(())
}

/// Rehydrate solution from assignment (placeholder)
pub fn rehydrate_solution(matrix: &mut QuboMatrix, assignment: &[u8], out: &mut [NQuin]) -> usize {
    // Placeholder: simple conversion
    let mut count = 0;
    for (i, &val) in assignment.iter().enumerate().take(out.len()) {
        if i < matrix.num_vars {
            out[count] = NQuin {
                subject: i as u64,
                predicate: val as u64,
                object: 0,
                context: 0,
                metadata: 0,
                parity: 0,
            };
            count += 1;
        }
    }
    count
}

/// Scrub personal URIs from index_map to ensure no sovereign data leaks
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
pub fn publish_to_commons(matrix: &mut QuboMatrix, storage_path: &std::path::Path) -> Result<String, String> {
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
    };
    
    // Check if there is an existing seed and deprecate it
    if let Some(_existing) = crate::webtorrent_seeder::lookup_seed(&info_hash) {
        crate::webtorrent_seeder::deprecate_seed(&info_hash);
    }
    
    crate::webtorrent_seeder::register_seed(req)?;
    Ok(crate::webtorrent_seeder::build_magnet_uri(&info_hash, "quantum_cache.q42", 4242))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_rigorous_metadata_scrubbing() {
        let mut matrix = QuboMatrix::new(2);
        // Inject mock personal URI data
        matrix.index_map.push((12345, 67890));
        matrix.index_count = 1;
        
        // Scrub
        scrub_metadata(&mut matrix);
        
        // Assert no traces of personal URIs
        assert_eq!(matrix.index_count, 0);
        assert!(matrix.index_map.is_empty(), "Index map was not fully scrubbed!");
    }
}