//! Fixed-capacity resident graph–tensor SOA for U1 background search (Track B3.3).
//!
//! Cold path may parse heap buffers; hot path (`tensor_search_into`) uses caller stack only.

use core::cell::UnsafeCell;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::OnceLock;

use super::buffer_export::{parse_header, read_tensor_at, TENSOR_STRIDE};
use super::q42_integration::{
    Q42TensorView, TensorMetadata, TensorVolumeConfig, TensorVolumeError,
};
use super::Tensor10D;
use crate::NQuin;

/// Maximum nodes pinned in U1 ledger (structural cap, not heap growth).
pub const MAX_RESIDENT_NODES: usize = 4096;
pub const MAX_KNN_HITS: usize = 32;

/// Process-wide resident substrate (U1 pin). Writer: cold load; reader: U1 producer.
pub struct ResidentTensorSubstrate {
    nquins: UnsafeCell<[NQuin; MAX_RESIDENT_NODES]>,
    metadata: UnsafeCell<[TensorMetadata; MAX_RESIDENT_NODES]>,
    subject_hashes: UnsafeCell<[u64; MAX_RESIDENT_NODES]>,
    node_count: AtomicU32,
    load_generation: AtomicU32,
}

// SAFETY: Writers synchronize via `load_generation` bump + Release; readers snapshot count.
unsafe impl Sync for ResidentTensorSubstrate {}

fn empty_nquin() -> NQuin {
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
    pub fn new() -> Self {
        Self {
            nquins: UnsafeCell::new([empty_nquin(); MAX_RESIDENT_NODES]),
            metadata: UnsafeCell::new([TensorMetadata::default(); MAX_RESIDENT_NODES]),
            subject_hashes: UnsafeCell::new([0u64; MAX_RESIDENT_NODES]),
            node_count: AtomicU32::new(0),
            load_generation: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn node_count(&self) -> u32 {
        self.node_count.load(Ordering::Acquire)
    }

    #[inline]
    pub fn load_generation(&self) -> u32 {
        self.load_generation.load(Ordering::Acquire)
    }

    #[inline]
    pub fn subject_hash_at(&self, index: u32) -> u64 {
        let count = self.node_count();
        if index >= count {
            return 0;
        }
        unsafe { (*self.subject_hashes.get())[index as usize] }
    }

    #[inline]
    pub fn tensor_at(&self, index: u32) -> Option<Tensor10D> {
        let count = self.node_count();
        if index >= count {
            return None;
        }
        let meta = unsafe { (*self.metadata.get())[index as usize] };
        if meta.has_tensor {
            Some(meta.tensor)
        } else {
            None
        }
    }

    /// Cold-path load from exported tensor buffer (spatial encode, daemon slice).
    pub fn load_from_tensor_buffer(
        &self,
        bytes: &[u8],
        default_subject_hash: u64,
    ) -> Result<u32, &'static str> {
        let (header, header_len) = parse_header(bytes)?;
        let count = header.node_count as usize;
        if count > MAX_RESIDENT_NODES {
            return Err("resident substrate capacity exceeded");
        }

        let nquins = unsafe { &mut *self.nquins.get() };
        let metadata = unsafe { &mut *self.metadata.get() };
        let subject_hashes = unsafe { &mut *self.subject_hashes.get() };

        for i in 0..count {
            let tensor = read_tensor_at(bytes, i)?;
            let subject = if default_subject_hash != 0 {
                default_subject_hash ^ (i as u64)
            } else {
                crate::q_hash(&format!("tensor:node:{i}"))
            };
            nquins[i] = stub_nquin_for_tensor(i, subject, &tensor);
            metadata[i] = TensorMetadata::from_nquin_and_tensor(&nquins[i], tensor);
            subject_hashes[i] = subject;
        }

        self.node_count.store(count as u32, Ordering::Release);
        self.load_generation.fetch_add(1, Ordering::AcqRel);
        Ok(count as u32)
    }

    /// Zero-heap kNN into caller buffer (U1 producer / SIMD path).
    pub fn tensor_search_into(
        &self,
        query: &Tensor10D,
        max_distance: f32,
        out: &mut [usize],
    ) -> Result<usize, TensorVolumeError> {
        let count = self.node_count() as usize;
        if count == 0 {
            return Ok(0);
        }
        let nquins =
            unsafe { std::slice::from_raw_parts(self.nquins.get() as *const NQuin, count) };
        let meta = unsafe {
            std::slice::from_raw_parts(self.metadata.get() as *const TensorMetadata, count)
        };
        let config = TensorVolumeConfig::default();
        let view = Q42TensorView::new(nquins, meta, &config)?;
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
        let nquins = unsafe { &mut *self.nquins.get() };
        let metadata = unsafe { &mut *self.metadata.get() };
        let subject_hashes = unsafe { &mut *self.subject_hashes.get() };

        for (i, tensor) in tensors.iter().enumerate() {
            let subject = if subject_hash != 0 {
                subject_hash ^ (i as u64)
            } else {
                crate::q_hash(&format!("tensor:node:{i}"))
            };
            nquins[i] = stub_nquin_for_tensor(i, subject, tensor);
            metadata[i] = TensorMetadata::from_nquin_and_tensor(&nquins[i], *tensor);
            subject_hashes[i] = subject;
        }

        self.node_count
            .store(tensors.len() as u32, Ordering::Release);
        self.load_generation.fetch_add(1, Ordering::AcqRel);
        Ok(tensors.len() as u32)
    }
}

#[inline]
fn stub_nquin_for_tensor(index: usize, subject: u64, tensor: &Tensor10D) -> NQuin {
    let object = ((tensor.x.to_bits() as u64) << 20)
        ^ (tensor.y.to_bits() as u64)
        ^ (tensor.z.to_bits() as u64);
    NQuin {
        subject,
        predicate: 0,
        object,
        context: 0,
        metadata: index as u64,
        parity: subject ^ object,
    }
}

static RESIDENT_SUBSTRATE: OnceLock<Box<ResidentTensorSubstrate>> = OnceLock::new();

#[inline]
pub fn global_resident_substrate() -> &'static ResidentTensorSubstrate {
    RESIDENT_SUBSTRATE.get_or_init(|| Box::new(ResidentTensorSubstrate::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::buffer_export::{write_tensor_buffer, TensorBufferHeader};

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
