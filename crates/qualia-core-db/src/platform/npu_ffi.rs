//! GPU sieve and native host domain bridges
//! Uses `wgpu` to execute cross-platform GPU compute shaders (Vulkan/DX12/Metal/WebGPU)
//! that filter the 5th Vector of the NQuin parallel arrays.

#[cfg(all(not(target_arch = "wasm32"), feature = "gpu-runtime"))]
pub mod gpu_sieve {
    use crate::NQuin;
    use etalon_wgsl_forge::numerical::compute_passes::bitmask::BitmaskFilter;
    pub use etalon_wgsl_forge::numerical::compute_passes::bitmask::FilterMask64;

    pub struct SieveOrchestrator {
        filter: BitmaskFilter,
    }
    impl SieveOrchestrator {
        pub async fn new() -> Option<Self> {
            Some(Self {
                filter: BitmaskFilter::new(include_str!("../shaders/sieve.wgsl")).await?,
            })
        }
        pub async fn execute_sieve(&self, quins: &[NQuin], mask: u64) -> Option<Vec<u32>> {
            self.filter
                .execute_sieve(bytemuck::cast_slice(quins), quins.len(), mask, || {
                    crate::telemetry::SIEVE_OPS_COUNT
                        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                })
                .await
        }
    }
}

/// Core 2 FFI Bindings for Non-Euclidean Tropical Sieve (NETS)
/// These C-ABI functions expose the Lorentz mapping and Min-Plus tropical arithmetic
/// directly to the GPU/NPU orchestration layer, guaranteeing $O(1)$ routing.
#[no_mangle]
pub unsafe extern "C" fn nets_map_lorentz(
    quins_ptr: *const crate::NQuin,
    quins_len: usize,
    out_lorentz_ptr: *mut crate::domains::mathematical::geometric::LorentzVector,
) {
    let quins = std::slice::from_raw_parts(quins_ptr, quins_len);
    let out_lorentz = std::slice::from_raw_parts_mut(out_lorentz_ptr, quins_len);

    for i in 0..quins_len {
        out_lorentz[i] =
            crate::domains::mathematical::geometric::LorentzVector::from_quin(&quins[i]);
    }
}

#[no_mangle]
/// NPU FFI: tropical-Voronoi route on the accelerator.
pub unsafe extern "C" fn nets_tropical_voronoi_route(
    queries_ptr: *const crate::domains::mathematical::geometric::LorentzVector,
    queries_len: usize,
    centroids_ptr: *const crate::domains::mathematical::geometric::MinPlusVoronoiCell,
    centroids_len: usize,
    out_cell_ids_ptr: *mut u32,
) {
    let queries = std::slice::from_raw_parts(queries_ptr, queries_len);
    let centroids = std::slice::from_raw_parts(centroids_ptr, centroids_len);
    let out_cell_ids = std::slice::from_raw_parts_mut(out_cell_ids_ptr, queries_len);

    for i in 0..queries_len {
        let query = &queries[i];
        let mut best_id = 0;
        let mut min_dist = f32::MAX;

        for centroid in centroids {
            let dist = centroid.tropical_distance(query);
            if dist < min_dist {
                min_dist = dist;
                best_id = centroid.cell_id;
            }
        }

        out_cell_ids[i] = best_id;
        crate::telemetry::SIEVE_OPS_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Pure Rust Valency & Stoichiometric Bridges
/// Previously C-ABI, now migrated to native Rust to avoid FFI overhead.
pub fn nets_parse_smiles(smiles: &str) -> Option<crate::NQuin> {
    if smiles.is_empty() {
        return None;
    }

    let mol = crate::domains::chemical::organic_chemistry::parse_smiles(smiles);
    if !mol.is_valid || mol.atoms.is_empty() {
        return None;
    }

    let first_valence =
        crate::domains::chemical::organic_chemistry::default_valence(&mol.atoms[0].element) as u64;
    let total_valence: u64 = mol
        .atoms
        .iter()
        .map(|a| crate::domains::chemical::organic_chemistry::default_valence(&a.element) as u64)
        .sum();
    let atom_count = mol.atoms.len() as u64;
    let bond_count = mol.bonds.len() as u64;

    // Pack: [0..15]: first atom valency, [16..31]: total valency, [32..47]: atom count, [48..63]: bond count
    let object = (first_valence & 0xFFFF)
        | ((total_valence & 0xFFFF) << 16)
        | ((atom_count & 0xFFFF) << 32)
        | ((bond_count & 0xFFFF) << 48);

    let subject = crate::q_hash(smiles);
    let predicate = crate::q_hash("IS_SMILES");
    let context = 0;
    let metadata = 0b01 << 61;
    let parity = subject ^ predicate ^ object ^ context;

    Some(crate::NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    })
}

/// NPU valency for the net.
pub fn nets_calculate_valency(molecule_quin: &crate::NQuin) -> i32 {
    if molecule_quin.predicate == crate::q_hash("IS_SMILES") {
        (molecule_quin.object & 0xFFFF) as i32
    } else {
        -1
    }
}
