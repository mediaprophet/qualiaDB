use crate::solvers::linear_algebra::{FixedLanczosEigensolver, Matrix4x4, Vector4};
use crate::solvers::{SolverConfig, SolverResult, SolverState, SolversError};
use crate::{q_hash, NQuin};

pub const MANIFOLD_HEAD_PREDICATE: u64 = q_hash("q42:manifold10d:head");
pub const MANIFOLD_TAIL_PREDICATE: u64 = q_hash("q42:manifold10d:tail");
pub const MANIFOLD_THRESHOLD_HOLDS: u64 = q_hash("q42:manifold10d:threshold-holds");
pub const MANIFOLD_THRESHOLD_MISS: u64 = q_hash("q42:manifold10d:threshold-miss");

pub const MANIFOLD_ATOM_COHERENT: u64 = q_hash("q42:manifold10d:coherent");
pub const MANIFOLD_ATOM_RECURRENT: u64 = q_hash("q42:manifold10d:recurrent");
pub const MANIFOLD_ATOM_DENSE: u64 = q_hash("q42:manifold10d:dense");
pub const MANIFOLD_ATOM_CURVED: u64 = q_hash("q42:manifold10d:curved");
pub const MANIFOLD_ATOM_STABLE: u64 = q_hash("q42:manifold10d:stable-topology");
pub const MANIFOLD_ASP_ATOMS: [u64; 5] = [
    MANIFOLD_ATOM_COHERENT,
    MANIFOLD_ATOM_RECURRENT,
    MANIFOLD_ATOM_DENSE,
    MANIFOLD_ATOM_CURVED,
    MANIFOLD_ATOM_STABLE,
];

/// Defines a tensor's precise location in the 10D geometric frameset.
/// This replaces the concept of integer chronological layers (e.g. "Layer 12")
/// with a continuous spatial coordinate in P64 containers.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ManifoldCoordinate10D {
    pub scale: f32,
    pub attention_depth: f32,
    pub epistemic_weight: f32,
    pub topological_spin: f32,
    pub temporal_decay: f32,
    pub entropy_bias: f32,
    pub spatial_phase: f32,
    pub recurrence_frequency: f32,
    pub density_threshold: f32,
    pub manifold_curvature: f32,
}

impl ManifoldCoordinate10D {
    pub const DIMENSIONS: usize = 10;

    /// Raw f32 representation used by the fixed 64-byte P64 manifold record.
    pub fn as_f32_array(&self) -> [f32; Self::DIMENSIONS] {
        [
            self.scale,
            self.attention_depth,
            self.epistemic_weight,
            self.topological_spin,
            self.temporal_decay,
            self.entropy_bias,
            self.spatial_phase,
            self.recurrence_frequency,
            self.density_threshold,
            self.manifold_curvature,
        ]
    }

    /// Decode one cache-line-sized P64 manifold record.
    pub fn from_p64_bytes(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() < 40 {
            return Err("p64: truncated 10D manifold coordinate".to_string());
        }
        let value = |index: usize| {
            let start = index * 4;
            f32::from_le_bytes(bytes[start..start + 4].try_into().unwrap())
        };
        let coordinate = Self {
            scale: value(0),
            attention_depth: value(1),
            epistemic_weight: value(2),
            topological_spin: value(3),
            temporal_decay: value(4),
            entropy_bias: value(5),
            spatial_phase: value(6),
            recurrence_frequency: value(7),
            density_threshold: value(8),
            manifold_curvature: value(9),
        };
        if coordinate
            .as_f32_array()
            .iter()
            .any(|value| !value.is_finite())
        {
            return Err("p64: non-finite 10D manifold coordinate".to_string());
        }
        Ok(coordinate)
    }

    /// Convert to the raw 10D array for math solvers
    pub fn as_array(&self) -> [f64; 10] {
        [
            self.scale as f64,
            self.attention_depth as f64,
            self.epistemic_weight as f64,
            self.topological_spin as f64,
            self.temporal_decay as f64,
            self.entropy_bias as f64,
            self.spatial_phase as f64,
            self.recurrence_frequency as f64,
            self.density_threshold as f64,
            self.manifold_curvature as f64,
        ]
    }

    /// Map a legacy 1D sequential Transformer layer onto the 10D geometry
    pub fn from_sequential_layer(layer: u32, total_layers: u32) -> Self {
        let max_l = total_layers.max(1) as f32;
        let l = layer as f32;
        let depth = l / max_l;
        Self {
            scale: depth,
            attention_depth: 1.0 - depth,
            epistemic_weight: 1.0,
            topological_spin: (depth * std::f32::consts::PI).sin(),
            temporal_decay: 0.1,
            entropy_bias: 0.5,
            spatial_phase: (depth * std::f32::consts::TAU).cos(),
            recurrence_frequency: 1.0,
            density_threshold: 0.8,
            manifold_curvature: 0.0,
        }
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ManifoldDimension {
    Scale = 0,
    AttentionDepth = 1,
    EpistemicWeight = 2,
    TopologicalSpin = 3,
    TemporalDecay = 4,
    EntropyBias = 5,
    SpatialPhase = 6,
    RecurrenceFrequency = 7,
    DensityThreshold = 8,
    ManifoldCurvature = 9,
}

impl ManifoldDimension {
    pub fn from_u8(value: u8) -> Option<Self> {
        Some(match value {
            0 => Self::Scale,
            1 => Self::AttentionDepth,
            2 => Self::EpistemicWeight,
            3 => Self::TopologicalSpin,
            4 => Self::TemporalDecay,
            5 => Self::EntropyBias,
            6 => Self::SpatialPhase,
            7 => Self::RecurrenceFrequency,
            8 => Self::DensityThreshold,
            9 => Self::ManifoldCurvature,
            _ => return None,
        })
    }

    pub fn value(self, coordinate: &ManifoldCoordinate10D) -> f32 {
        coordinate.as_f32_array()[self as usize]
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ManifoldState10D {
    /// Unique state identifier (for example `tensor_hash ^ logical_clock`).
    pub state_id: u64,
    pub timestamp: u64,
    pub coordinate: ManifoldCoordinate10D,
}

#[inline]
fn pack_f32_pair(low: f32, high: f32) -> u64 {
    low.to_bits() as u64 | ((high.to_bits() as u64) << 32)
}

#[inline]
fn unpack_f32_pair(value: u64) -> (f32, f32) {
    (
        f32::from_bits(value as u32),
        f32::from_bits((value >> 32) as u32),
    )
}

#[inline]
fn quin_with_parity(
    subject: u64,
    predicate: u64,
    object: u64,
    context: u64,
    metadata: u64,
) -> NQuin {
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity: subject ^ predicate ^ object ^ context,
    }
}

/// Encode one 10D state into two normal 48-byte Quins.
///
/// The head carries dimensions 0..=5 in three packed f32 pairs. The tail
/// carries dimensions 6..=9 and the logical timestamp. Both retain the normal
/// XOR parity contract. These are VM-internal geometry records: their packed
/// numeric fields are not resolver literals and therefore avoid the known
/// object type-tag conflict.
pub fn encode_manifold_state(state: &ManifoldState10D, out: &mut [NQuin; 2]) {
    let d = state.coordinate.as_f32_array();
    out[0] = quin_with_parity(
        state.state_id,
        MANIFOLD_HEAD_PREDICATE,
        pack_f32_pair(d[0], d[1]),
        pack_f32_pair(d[2], d[3]),
        pack_f32_pair(d[4], d[5]),
    );
    out[1] = quin_with_parity(
        state.state_id,
        MANIFOLD_TAIL_PREDICATE,
        pack_f32_pair(d[6], d[7]),
        pack_f32_pair(d[8], d[9]),
        state.timestamp,
    );
}

pub fn decode_manifold_state(head: &NQuin, tail: &NQuin) -> Option<ManifoldState10D> {
    if head.subject != tail.subject
        || head.predicate != MANIFOLD_HEAD_PREDICATE
        || tail.predicate != MANIFOLD_TAIL_PREDICATE
        || head.parity != head.subject ^ head.predicate ^ head.object ^ head.context
        || tail.parity != tail.subject ^ tail.predicate ^ tail.object ^ tail.context
    {
        return None;
    }
    let (d0, d1) = unpack_f32_pair(head.object);
    let (d2, d3) = unpack_f32_pair(head.context);
    let (d4, d5) = unpack_f32_pair(head.metadata);
    let (d6, d7) = unpack_f32_pair(tail.object);
    let (d8, d9) = unpack_f32_pair(tail.context);
    let coordinate = ManifoldCoordinate10D {
        scale: d0,
        attention_depth: d1,
        epistemic_weight: d2,
        topological_spin: d3,
        temporal_decay: d4,
        entropy_bias: d5,
        spatial_phase: d6,
        recurrence_frequency: d7,
        density_threshold: d8,
        manifold_curvature: d9,
    };
    if coordinate
        .as_f32_array()
        .iter()
        .any(|value| !value.is_finite())
    {
        return None;
    }
    Some(ManifoldState10D {
        state_id: head.subject,
        timestamp: tail.metadata,
        coordinate,
    })
}

/// Decode and chronologically order manifold pairs from an arena snapshot.
/// Invalid/incomplete pairs are ignored. Caller owns the bounded output.
pub fn collect_manifold_states(quins: &[NQuin], out: &mut [ManifoldState10D]) -> usize {
    let mut count = 0usize;
    for head in quins {
        if head.predicate != MANIFOLD_HEAD_PREDICATE || count == out.len() {
            continue;
        }
        let Some(tail) = quins.iter().find(|candidate| {
            candidate.subject == head.subject && candidate.predicate == MANIFOLD_TAIL_PREDICATE
        }) else {
            continue;
        };
        if let Some(state) = decode_manifold_state(head, tail) {
            out[count] = state;
            count += 1;
        }
    }
    // Stable bounded insertion sort; no allocation and deterministic for equal timestamps.
    for index in 1..count {
        let state = out[index];
        let mut cursor = index;
        while cursor > 0 && out[cursor - 1].timestamp > state.timestamp {
            out[cursor] = out[cursor - 1];
            cursor -= 1;
        }
        out[cursor] = state;
    }
    count
}

/// Convert a 10D coordinate trace into propositions consumable by the existing
/// LTL evaluator. `at_least=true` means `dimension >= threshold`; false means
/// `dimension <= threshold`.
pub fn project_manifold_ltl_trace(
    states: &[ManifoldState10D],
    dimension: ManifoldDimension,
    threshold: f32,
    at_least: bool,
    out: &mut [NQuin],
) -> usize {
    let count = states.len().min(out.len());
    for (target, state) in out[..count].iter_mut().zip(states) {
        let value = dimension.value(&state.coordinate);
        let holds = if at_least {
            value >= threshold
        } else {
            value <= threshold
        };
        *target = quin_with_parity(
            state.state_id,
            if holds {
                MANIFOLD_THRESHOLD_HOLDS
            } else {
                MANIFOLD_THRESHOLD_MISS
            },
            value.to_bits() as u64,
            dimension as u64,
            state.timestamp,
        );
    }
    count
}

/// Derive bounded topology facts from manifold states and run the real
/// Gelfond-Lifschitz answer-set evaluator.
pub fn evaluate_manifold_answer_sets(states: &[ManifoldState10D], out: &mut [u64]) -> usize {
    use crate::modalities::asp::{compute_answer_sets, AspRule};

    let mut facts = [false; 4];
    for state in states {
        let coordinate = &state.coordinate;
        facts[0] |= coordinate.epistemic_weight >= coordinate.entropy_bias;
        facts[1] |= coordinate.recurrence_frequency > coordinate.temporal_decay;
        facts[2] |= coordinate.scale >= coordinate.density_threshold;
        facts[3] |= coordinate.manifold_curvature.abs() > 0.25;
    }

    let mut rules = [AspRule::fact(0); 5];
    let mut rule_count = 0usize;
    for (present, atom) in facts.iter().zip(MANIFOLD_ASP_ATOMS.iter().take(4)) {
        if *present {
            rules[rule_count] = AspRule::fact(*atom);
            rule_count += 1;
        }
    }
    // stable_topology :- coherent, recurrent, not curved.
    rules[rule_count] = AspRule::new(
        MANIFOLD_ATOM_STABLE,
        &[MANIFOLD_ATOM_COHERENT, MANIFOLD_ATOM_RECURRENT],
        &[MANIFOLD_ATOM_CURVED],
    );
    rule_count += 1;
    compute_answer_sets(&MANIFOLD_ASP_ATOMS, &rules[..rule_count], out)
}

/// Project a continuous 10D symmetric matrix representation into a valid 4D unit quaternion.
/// This avoids gimbal lock and inverse-image discontinuities common in neural orientation regression.
pub fn project_10d_to_quaternion(parameters: &[f64; 10]) -> SolverResult<Vector4> {
    // Reconstruct the 4x4 symmetric matrix from the 10 parameters
    let mut matrix = Matrix4x4::zero();
    matrix.set(0, 0, parameters[0]);
    matrix.set(0, 1, parameters[1]);
    matrix.set(0, 2, parameters[2]);
    matrix.set(0, 3, parameters[3]);

    matrix.set(1, 0, parameters[1]);
    matrix.set(1, 1, parameters[4]);
    matrix.set(1, 2, parameters[5]);
    matrix.set(1, 3, parameters[6]);

    matrix.set(2, 0, parameters[2]);
    matrix.set(2, 1, parameters[5]);
    matrix.set(2, 2, parameters[7]);
    matrix.set(2, 3, parameters[8]);

    matrix.set(3, 0, parameters[3]);
    matrix.set(3, 1, parameters[6]);
    matrix.set(3, 2, parameters[8]);
    matrix.set(3, 3, parameters[9]);

    let mut solver = FixedLanczosEigensolver {
        iteration: 0,
        alpha: [0.0; 100],
        beta: [0.0; 100],
        vectors: [Vector4::zero(); 3],
        eigenvalues: [0.0; 4],
        config: SolverConfig::default(),
        solver_state: SolverState::default(),
    };

    // We solve for the smallest eigenvector.
    match solver.solve_smallest_eigenvector(&matrix) {
        Ok(vec) => Ok(vec),
        Err(e) => Err(e),
    }
}

impl FixedLanczosEigensolver {
    /// Solves for the eigenvector corresponding to the smallest eigenvalue.
    /// This is a deterministic numerical method.
    pub fn solve_smallest_eigenvector(&mut self, matrix: &Matrix4x4) -> SolverResult<Vector4> {
        // Mock implementation of Lanczos iteration for a 4x4 symmetric matrix
        // In reality, this would run the tridiagonalization and then QR algorithm.
        // For zero-allocation, we perform power iteration on (c*I - A) to find the smallest eigenpair.

        let mut v = Vector4 {
            data: [1.0, 0.5, 0.25, 0.125],
        };

        let mut max_row_sum = 0.0;
        for i in 0..4 {
            let mut sum = 0.0;
            for j in 0..4 {
                sum += matrix.data[i][j].abs();
            }
            if sum > max_row_sum {
                max_row_sum = sum;
            }
        }
        let c = max_row_sum + 1.0; // Shift to make (c*I - A) positive definite

        for _ in 0..self.config.max_iterations {
            let mut shifted_matrix = Matrix4x4::zero();
            for i in 0..4 {
                for j in 0..4 {
                    if i == j {
                        shifted_matrix.set(i, j, c - matrix.get(i, j));
                    } else {
                        shifted_matrix.set(i, j, -matrix.get(i, j));
                    }
                }
            }

            let mut next_v = shifted_matrix.multiply_vector(&v);

            // Normalize next_v
            let norm = (next_v.data[0].powi(2)
                + next_v.data[1].powi(2)
                + next_v.data[2].powi(2)
                + next_v.data[3].powi(2))
            .sqrt();

            if norm == 0.0 {
                return Err(SolversError::SingularMatrix);
            }

            next_v.data[0] /= norm;
            next_v.data[1] /= norm;
            next_v.data[2] /= norm;
            next_v.data[3] /= norm;

            // Check convergence
            let mut diff = 0.0;
            for i in 0..4 {
                diff += (next_v.data[i] - v.data[i]).abs();
            }

            v = next_v;
            self.iteration += 1;

            if diff < self.config.tolerance {
                self.solver_state.converged = true;
                return Ok(v);
            }
        }

        Err(SolversError::ConvergenceFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modalities::asp::atom_index;
    use crate::modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};

    fn state(id: u64, timestamp: u64, scale: f32, curvature: f32) -> ManifoldState10D {
        let mut coordinate = ManifoldCoordinate10D::from_sequential_layer(timestamp as u32, 10);
        coordinate.scale = scale;
        coordinate.density_threshold = 0.5;
        coordinate.manifold_curvature = curvature;
        ManifoldState10D {
            state_id: id,
            timestamp,
            coordinate,
        }
    }

    #[test]
    fn two_quin_encoding_round_trips_and_sorts() {
        let older = state(10, 1, 0.7, 0.0);
        let newer = state(20, 2, 0.8, 0.0);
        let mut older_pair = [NQuin::default(); 2];
        let mut newer_pair = [NQuin::default(); 2];
        encode_manifold_state(&older, &mut older_pair);
        encode_manifold_state(&newer, &mut newer_pair);
        assert_eq!(
            decode_manifold_state(&older_pair[0], &older_pair[1]),
            Some(older)
        );

        let arena_order = [newer_pair[1], older_pair[0], newer_pair[0], older_pair[1]];
        let mut decoded = [ManifoldState10D::default(); 2];
        let count = collect_manifold_states(&arena_order, &mut decoded);
        assert_eq!(count, 2);
        assert_eq!(decoded[0], older);
        assert_eq!(decoded[1], newer);
    }

    #[test]
    fn manifold_projection_drives_existing_ltl_evaluator() {
        let states = [state(1, 1, 0.6, 0.0), state(2, 2, 0.9, 0.0)];
        let mut trace = [NQuin::default(); 2];
        let count =
            project_manifold_ltl_trace(&states, ManifoldDimension::Scale, 0.5, true, &mut trace);
        assert!(evaluate_ltl_trace(
            &trace[..count],
            &LtlFormula::Globally(MANIFOLD_THRESHOLD_HOLDS)
        ));

        let count =
            project_manifold_ltl_trace(&states, ManifoldDimension::Scale, 0.8, true, &mut trace);
        assert!(!evaluate_ltl_trace(
            &trace[..count],
            &LtlFormula::Globally(MANIFOLD_THRESHOLD_HOLDS)
        ));
        assert!(evaluate_ltl_trace(
            &trace[..count],
            &LtlFormula::Finally(MANIFOLD_THRESHOLD_HOLDS)
        ));
    }

    #[test]
    fn manifold_topology_drives_real_answer_set_semantics() {
        let states = [state(1, 1, 0.7, 0.0), state(2, 2, 0.8, 0.0)];
        let mut models = [0u64; 8];
        let count = evaluate_manifold_answer_sets(&states, &mut models);
        assert_eq!(count, 1);
        let stable_index = atom_index(&MANIFOLD_ASP_ATOMS, MANIFOLD_ATOM_STABLE).unwrap();
        assert_ne!(models[0] & (1u64 << stable_index), 0);

        let curved = [state(3, 3, 0.8, 0.75)];
        let count = evaluate_manifold_answer_sets(&curved, &mut models);
        assert_eq!(count, 1);
        assert_eq!(models[0] & (1u64 << stable_index), 0);
    }
}
