//! Fixed-capacity resident graph–tensor SOA for U1 background search (Track B3.3).
//!
//! Cold path may parse heap buffers; hot path (`tensor_search_into`) uses caller stack only.

use std::sync::{OnceLock, RwLock};

use super::buffer_export::{parse_header, read_tensor_at};
use super::q42_integration::{
    Q42TensorView, TensorMetadata, TensorVolumeConfig, TensorVolumeError,
};
use super::Tensor10D;
use crate::NQuin;

/// Maximum nodes pinned in U1 ledger (structural cap, not heap growth).
pub const MAX_RESIDENT_NODES: usize = 4096;
pub const MAX_KNN_HITS: usize = 32;

/// Process-wide resident substrate (U1 pin). Writer: cold load; reader: U1 producer.
/// Each call observes one complete load; separate calls may observe different generations.
pub struct ResidentTensorSubstrate {
    state: RwLock<ResidentState>,
}

struct ResidentState {
    nquins: [NQuin; MAX_RESIDENT_NODES],
    metadata: [TensorMetadata; MAX_RESIDENT_NODES],
    subject_hashes: [u64; MAX_RESIDENT_NODES],
    node_count: u32,
    load_generation: u32,
}

const fn empty_nquin() -> NQuin {
    NQuin {
        subject: 0,
        predicate: 0,
        object: 0,
        context: 0,
        metadata: 0,
        parity: 0,
    }
}

impl ResidentTensorSubstrate {
    /// Creates a default `ResidentTensorSubstrate`.
    pub fn new() -> Self {
        // Constant initialization avoids large intermediate arrays on debug-build stacks.
        const {
            Self {
                state: RwLock::new(ResidentState {
                    nquins: [empty_nquin(); MAX_RESIDENT_NODES],
                    metadata: [TensorMetadata {
                        tensor: Tensor10D {
                            q: 0.0,
                            v: 0.0,
                            w: 0.0,
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                            t: 0.0,
                            alpha: 1.0,
                            mu: 0.0,
                            sigma: 0.0,
                        },
                        has_tensor: false,
                        tensor_version: 1,
                    }; MAX_RESIDENT_NODES],
                    subject_hashes: [0u64; MAX_RESIDENT_NODES],
                    node_count: 0,
                    load_generation: 0,
                }),
            }
        }
    }

    #[inline]
    /// Published node count, or zero after a writer panic poisons the substrate.
    pub fn node_count(&self) -> u32 {
        self.state.read().map_or(0, |state| state.node_count)
    }

    #[inline]
    /// Wrapping load generation, or zero if the substrate is poisoned.
    pub fn load_generation(&self) -> u32 {
        self.state.read().map_or(0, |state| state.load_generation)
    }

    #[inline]
    /// Published subject hash, or zero for a missing index or poisoned substrate.
    pub fn subject_hash_at(&self, index: u32) -> u64 {
        let Ok(state) = self.state.read() else {
            return 0;
        };
        if index >= state.node_count {
            return 0;
        }
        state.subject_hashes[index as usize]
    }

    #[inline]
    /// Copies a published tensor, or returns `None` if missing or poisoned.
    pub fn tensor_at(&self, index: u32) -> Option<Tensor10D> {
        let state = self.state.read().ok()?;
        if index >= state.node_count {
            return None;
        }
        let meta = state.metadata[index as usize];
        if meta.has_tensor {
            Some(meta.tensor)
        } else {
            None
        }
    }

    /// Cold-path load from exported tensor buffer (spatial encode, daemon slice).
    /// Invalid buffers leave the published generation unchanged.
    pub fn load_from_tensor_buffer(
        &self,
        bytes: &[u8],
        default_subject_hash: u64,
    ) -> Result<u32, &'static str> {
        let (header, _header_len) = parse_header(bytes)?;
        let count = header.node_count as usize;
        if count > MAX_RESIDENT_NODES {
            return Err("resident substrate capacity exceeded");
        }

        // Validate every record before replacing any published data.
        for i in 0..count {
            read_tensor_at(bytes, i)?;
        }
        let mut state = self
            .state
            .write()
            .map_err(|_| "resident substrate poisoned")?;

        for i in 0..count {
            let tensor = read_tensor_at(bytes, i)?;
            let subject = if default_subject_hash != 0 {
                default_subject_hash ^ (i as u64)
            } else {
                crate::q_hash(&format!("tensor:node:{i}"))
            };
            state.nquins[i] = synthesize_nquin_for_tensor(i, subject, &tensor);
            state.metadata[i] = TensorMetadata::from_nquin_and_tensor(&state.nquins[i], tensor);
            state.subject_hashes[i] = subject;
        }

        state.node_count = count as u32;
        state.load_generation = state.load_generation.wrapping_add(1);
        Ok(count as u32)
    }

    /// Zero-heap kNN into caller buffer (U1 producer / SIMD path).
    /// A poisoned substrate reports `TensorVolumeError::MismatchedStorage`.
    pub fn tensor_search_into(
        &self,
        query: &Tensor10D,
        max_distance: f32,
        out: &mut [usize],
    ) -> Result<usize, TensorVolumeError> {
        let state = self
            .state
            .read()
            .map_err(|_| TensorVolumeError::MismatchedStorage)?;
        let count = state.node_count as usize;
        if count == 0 {
            return Ok(0);
        }
        let config = TensorVolumeConfig::default();
        let view = Q42TensorView::new(&state.nquins[..count], &state.metadata[..count], &config)?;
        view.tensor_search_into(query, max_distance, out)
    }

    /// Load tensors directly (native encode path without full buffer round-trip).
    pub fn load_from_tensors(
        &self,
        tensors: &[Tensor10D],
        subject_hash: u64,
    ) -> Result<u32, &'static str> {
        if tensors.len() > MAX_RESIDENT_NODES {
            return Err("resident substrate capacity exceeded");
        }
        let mut state = self
            .state
            .write()
            .map_err(|_| "resident substrate poisoned")?;

        for (i, tensor) in tensors.iter().enumerate() {
            let subject = if subject_hash != 0 {
                subject_hash ^ (i as u64)
            } else {
                crate::q_hash(&format!("tensor:node:{i}"))
            };
            state.nquins[i] = synthesize_nquin_for_tensor(i, subject, tensor);
            state.metadata[i] = TensorMetadata::from_nquin_and_tensor(&state.nquins[i], *tensor);
            state.subject_hashes[i] = subject;
        }

        state.node_count = tensors.len() as u32;
        state.load_generation = state.load_generation.wrapping_add(1);
        Ok(tensors.len() as u32)
    }
}

#[inline]
fn synthesize_nquin_for_tensor(index: usize, subject: u64, tensor: &Tensor10D) -> NQuin {
    let xi = ((tensor.x * 1000.0).clamp(-524288.0, 524287.0) as i32) as u64 & 0xFFFFF;
    let yi = ((tensor.y * 1000.0).clamp(-524288.0, 524287.0) as i32) as u64 & 0xFFFFF;
    let zi = ((tensor.z * 1000.0).clamp(-524288.0, 524287.0) as i32) as u64 & 0xFFFFF;
    let object = (xi << 40) | (yi << 20) | zi;
    let predicate = super::bake_pipeline::PRED_GEO_VERTEX;
    let context = crate::q_hash("urn:qualia:context:tensor_substrate");
    let metadata = index as u64;
    NQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity: subject ^ predicate ^ object ^ context,
    }
}

static RESIDENT_SUBSTRATE: OnceLock<Box<ResidentTensorSubstrate>> = OnceLock::new();

#[inline]
/// `ResidentTensorSubstrate::global_resident_substrate`.
pub fn global_resident_substrate() -> &'static ResidentTensorSubstrate {
    RESIDENT_SUBSTRATE.get_or_init(|| Box::new(ResidentTensorSubstrate::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;
    use crate::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};

    #[test]
    fn initial_state_matches_metadata_default() {
        let sub = ResidentTensorSubstrate::new();
        let state = sub.state.read().unwrap();
        assert_eq!(state.node_count, 0);
        assert_eq!(state.load_generation, 0);
        for i in 0..MAX_RESIDENT_NODES {
            assert_eq!(state.nquins[i], NQuin::default());
            assert_eq!(state.metadata[i], TensorMetadata::default());
            assert_eq!(state.subject_hashes[i], 0);
        }
    }

    #[test]
    fn malformed_reload_preserves_published_generation() {
        let sub = ResidentTensorSubstrate::new();
        let original = Tensor10D::default();
        sub.load_from_tensors(&[original; 2], 42).unwrap();
        let changed = Tensor10D {
            x: 10.0,
            ..original
        };
        let mut bytes = vec![0; TensorBufferHeader::total_bytes(2)];
        write_tensor_buffer(&[changed; 2], &mut bytes).unwrap();
        bytes.pop();

        assert_eq!(
            sub.load_from_tensor_buffer(&bytes, 99),
            Err("buffer truncated")
        );
        assert_eq!(sub.node_count(), 2);
        assert_eq!(sub.load_generation(), 1);
        for i in 0..2 {
            assert_eq!(sub.tensor_at(i), Some(original));
            assert_eq!(sub.subject_hash_at(i), 42 ^ u64::from(i));
        }
        assert_eq!(sub.tensor_at(2), None);
        assert_eq!(sub.subject_hash_at(2), 0);
        let mut hits = [usize::MAX; 2];
        assert_eq!(sub.tensor_search_into(&original, 0.5, &mut hits), Ok(2));
        assert_eq!(hits, [0, 1]);
    }

    #[test]
    fn concurrent_reloads_publish_complete_generations() {
        const RELOADS: usize = 500;
        let sub = ResidentTensorSubstrate::new();
        let near = [Tensor10D::default(); 32];
        let far = [Tensor10D {
            x: 10.0,
            ..Tensor10D::default()
        }; 16];
        let mut bytes = vec![0; TensorBufferHeader::total_bytes(far.len())];
        write_tensor_buffer(&far, &mut bytes).unwrap();
        sub.load_from_tensors(&near, 32).unwrap();
        let start = std::sync::Barrier::new(5);

        std::thread::scope(|scope| {
            for _ in 0..2 {
                scope.spawn(|| {
                    start.wait();
                    for _ in 0..RELOADS {
                        sub.load_from_tensor_buffer(&bytes, 16).unwrap();
                        sub.load_from_tensors(&near, 32).unwrap();
                        std::thread::yield_now();
                    }
                });
            }
            for _ in 0..2 {
                scope.spawn(|| {
                    start.wait();
                    let mut generation = 0;
                    let mut hits = [usize::MAX; 32];
                    for _ in 0..RELOADS {
                        let n = sub.tensor_search_into(&near[0], 0.5, &mut hits).unwrap();
                        assert!(n == 0 || n == near.len());
                        for (index, hit) in hits[..n].iter().enumerate() {
                            assert_eq!(*hit, index);
                        }
                        assert!(matches!(sub.node_count(), 16 | 32));
                        assert!(matches!(sub.subject_hash_at(0), 16 | 32));
                        let tensor = sub.tensor_at(0).unwrap();
                        assert!(tensor == near[0] || tensor == far[0]);

                        let state = sub.state.read().unwrap();
                        assert!(state.load_generation >= generation);
                        generation = state.load_generation;
                        let expected = if state.node_count == 32 {
                            near[0]
                        } else {
                            far[0]
                        };
                        for i in 0..state.node_count as usize {
                            assert_eq!(state.metadata[i].tensor, expected);
                            assert_eq!(
                                state.subject_hashes[i],
                                u64::from(state.node_count) ^ i as u64
                            );
                            assert_eq!(state.nquins[i].subject, state.subject_hashes[i]);
                        }
                    }
                });
            }
            start.wait();
        });
        assert_eq!(sub.load_generation(), 1 + 4 * RELOADS as u32);
    }

    #[test]
    fn poisoned_substrate_fails_closed() {
        let sub = ResidentTensorSubstrate::new();
        let tensor = Tensor10D::default();
        let mut bytes = vec![0; TensorBufferHeader::total_bytes(1)];
        write_tensor_buffer(&[tensor], &mut bytes).unwrap();
        sub.load_from_tensors(&[tensor], 42).unwrap();
        assert!(std::panic::catch_unwind(|| {
            let _guard = sub.state.write().unwrap();
            panic!("interrupt resident reload");
        })
        .is_err());

        assert_eq!(sub.node_count(), 0);
        assert_eq!(sub.load_generation(), 0);
        assert_eq!(sub.tensor_at(0), None);
        assert_eq!(sub.subject_hash_at(0), 0);
        assert_eq!(
            sub.load_from_tensors(&[tensor], 42),
            Err("resident substrate poisoned")
        );
        assert_eq!(
            sub.load_from_tensor_buffer(&bytes, 42),
            Err("resident substrate poisoned")
        );
        assert_eq!(
            sub.tensor_search_into(&tensor, 1.0, &mut [0; 1]),
            Err(TensorVolumeError::MismatchedStorage)
        );
    }

    #[test]
    fn resident_reads_are_zero_heap() {
        let sub = ResidentTensorSubstrate::new();
        let tensor = Tensor10D::default();
        sub.load_from_tensors(&[tensor; 2], 42).unwrap();
        let mut hits = [0; 2];
        sub.tensor_search_into(&tensor, 0.5, &mut hits).unwrap();
        assert_zero_alloc("resident tensor reads", || {
            for _ in 0..32 {
                assert_eq!(sub.node_count(), 2);
                assert_eq!(sub.load_generation(), 1);
                assert_eq!(sub.tensor_at(0), Some(tensor));
                assert_eq!(sub.subject_hash_at(0), 42);
                assert_eq!(sub.tensor_search_into(&tensor, 0.5, &mut hits), Ok(2));
            }
        });
    }

    #[test]
    fn load_and_search_resident_substrate() {
        let sub = ResidentTensorSubstrate::new();
        let tensors = [
            Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            Tensor10D::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        ];
        let need = TensorBufferHeader::total_bytes(2);
        let mut buf = vec![0u8; need];
        write_tensor_buffer(&tensors, &mut buf).unwrap();
        assert_eq!(sub.load_from_tensor_buffer(&buf, 42).unwrap(), 2);

        let query = Tensor10D::new(0.0, 0.0, 0.0, 0.1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let mut hits = [0usize; MAX_KNN_HITS];
        let n = sub.tensor_search_into(&query, 0.5, &mut hits).unwrap();
        assert!(n >= 1);
        assert_eq!(hits[0], 0);
    }
}
