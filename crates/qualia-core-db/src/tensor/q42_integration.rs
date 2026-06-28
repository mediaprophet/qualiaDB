//! Q42 volume integration for the 10D tensor system.
//!
//! This module keeps runtime tensor storage and query execution zero-heap by
//! requiring callers to supply backing slices and output buffers.

use core::ops::ControlFlow;

use super::Tensor10D;
use crate::NQuin;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TensorVolumeError {
    MismatchedStorage,
    StorageCapacityExceeded,
    OutputBufferFull,
}

/// Tensor metadata that can be stored alongside NQuin data.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TensorMetadata {
    /// 10D tensor coordinates for this NQuin.
    pub tensor: Tensor10D,
    /// Whether this NQuin has associated tensor data.
    pub has_tensor: bool,
    /// Tensor data version.
    pub tensor_version: u32,
}

impl Default for TensorMetadata {
    fn default() -> Self {
        Self {
            tensor: Tensor10D::default(),
            has_tensor: false,
            tensor_version: 1,
        }
    }
}

impl TensorMetadata {
    /// Create tensor metadata from NQuin and tensor coordinates.
    pub fn from_nquin_and_tensor(_nquin: &NQuin, tensor: Tensor10D) -> Self {
        Self {
            tensor,
            has_tensor: true,
            tensor_version: 1,
        }
    }

    /// Create tensor metadata from NQuin only (no tensor coordinates yet).
    pub fn from_nquin_only(_nquin: &NQuin) -> Self {
        Self {
            tensor: Tensor10D::default(),
            has_tensor: false,
            tensor_version: 1,
        }
    }
}

/// Lightweight entry view used by zero-heap query APIs.
#[derive(Debug, Clone, Copy)]
pub struct TensorizedEntry<'a> {
    pub nquin: &'a NQuin,
    pub metadata: &'a TensorMetadata,
}

/// Convert NQuin to Tensor10D using semantic mapping.
impl Tensor10D {
    /// Convert NQuin to Tensor10D using semantic mapping.
    pub fn from_nquin(nquin: &NQuin) -> Self {
        let q = Self::extract_quantum_context(nquin);
        let v = Self::extract_topological_class(nquin);
        let w = Self::extract_manifold_index(nquin);
        let (x, y, z) = Self::extract_semantic_coordinates(nquin);
        let t = Self::extract_temporal_state(nquin);
        let (alpha, mu, sigma) = Self::extract_spectral_payload(nquin);

        Tensor10D::new(q, v, w, x, y, z, t, alpha, mu, sigma)
    }

    /// Extract quantum context from NQuin metadata.
    fn extract_quantum_context(_nquin: &NQuin) -> f32 {
        0.0
    }

    /// Extract topological class from NQuin context.
    fn extract_topological_class(_nquin: &NQuin) -> f32 {
        0.0
    }

    /// Extract manifold index from NQuin context.
    fn extract_manifold_index(_nquin: &NQuin) -> f32 {
        0.0
    }

    /// Extract semantic coordinates from NQuin object field.
    fn extract_semantic_coordinates(nquin: &NQuin) -> (f32, f32, f32) {
        super::bake_pipeline::semantic_xyz_from_nquin(nquin)
    }

    /// Extract temporal state from NQuin metadata Lamport clock.
    fn extract_temporal_state(nquin: &NQuin) -> f32 {
        let clock = nquin.metadata >> 32;
        clock as f32
    }

    /// Extract spectral payload from NQuin metadata.
    fn extract_spectral_payload(nquin: &NQuin) -> (f32, f32, f32) {
        let payload = nquin.metadata & 0xFFFF_FFFF;
        let alpha = (payload & 0xFF) as f32 / 255.0;
        let mu = ((payload >> 8) & 0xFF) as f32 / 255.0;
        let sigma = ((payload >> 16) & 0xFF) as f32 / 255.0;
        (alpha, mu, sigma)
    }
}

/// Volume-level tensor configuration.
#[repr(C)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TensorVolumeConfig {
    /// Enable 10D tensor operations.
    pub tensor_enabled: bool,
    /// Default manifold index for new NQuins.
    pub default_manifold: f32,
    /// Default topological class for new NQuins.
    pub default_topology: f32,
    /// Tensor version for this volume.
    pub tensor_version: u32,
}

impl Default for TensorVolumeConfig {
    fn default() -> Self {
        Self {
            tensor_enabled: true,
            default_manifold: 0.0,
            default_topology: 0.0,
            tensor_version: 1,
        }
    }
}

/// Zero-heap tensor volume backed by caller-owned storage.
pub struct Q42TensorVolume<'a> {
    nquins: &'a mut [NQuin],
    tensor_metadata: &'a mut [TensorMetadata],
    len: usize,
    volume_config: TensorVolumeConfig,
}

/// Immutable zero-heap tensor query view.
#[derive(Debug)]
pub struct Q42TensorView<'a> {
    nquins: &'a [NQuin],
    tensor_metadata: &'a [TensorMetadata],
    volume_config: &'a TensorVolumeConfig,
}

impl<'a> Q42TensorVolume<'a> {
    /// Create a new tensor volume over caller-supplied storage.
    pub fn new(
        nquins: &'a mut [NQuin],
        tensor_metadata: &'a mut [TensorMetadata],
    ) -> Result<Self, TensorVolumeError> {
        Self::with_config(nquins, tensor_metadata, TensorVolumeConfig::default())
    }

    /// Create a new tensor volume with specific configuration.
    pub fn with_config(
        nquins: &'a mut [NQuin],
        tensor_metadata: &'a mut [TensorMetadata],
        config: TensorVolumeConfig,
    ) -> Result<Self, TensorVolumeError> {
        if nquins.len() != tensor_metadata.len() {
            return Err(TensorVolumeError::MismatchedStorage);
        }

        Ok(Self {
            nquins,
            tensor_metadata,
            len: 0,
            volume_config: config,
        })
    }

    /// Borrow this volume as a zero-heap query view.
    pub fn as_view(&self) -> Q42TensorView<'_> {
        Q42TensorView {
            nquins: &self.nquins[..self.len],
            tensor_metadata: &self.tensor_metadata[..self.len],
            volume_config: &self.volume_config,
        }
    }

    /// Add an NQuin to the volume with tensor coordinates.
    pub fn add_nquin_with_tensor(
        &mut self,
        nquin: NQuin,
        tensor: Tensor10D,
    ) -> Result<usize, TensorVolumeError> {
        if self.len >= self.nquins.len() {
            return Err(TensorVolumeError::StorageCapacityExceeded);
        }

        self.nquins[self.len] = nquin;
        self.tensor_metadata[self.len] =
            TensorMetadata::from_nquin_and_tensor(&self.nquins[self.len], tensor);
        self.len += 1;
        Ok(self.len)
    }

    /// Add an NQuin to the volume without tensor coordinates.
    pub fn add_nquin(&mut self, nquin: NQuin) -> Result<usize, TensorVolumeError> {
        if self.len >= self.nquins.len() {
            return Err(TensorVolumeError::StorageCapacityExceeded);
        }

        self.nquins[self.len] = nquin;
        self.tensor_metadata[self.len] = TensorMetadata::from_nquin_only(&self.nquins[self.len]);
        self.len += 1;
        Ok(self.len)
    }

    /// Reset the used prefix and scrub old entries.
    pub fn clear(&mut self) {
        for index in 0..self.len {
            self.nquins[index] = NQuin::default();
            self.tensor_metadata[index] = TensorMetadata::default();
        }
        self.len = 0;
    }

    /// Get tensor metadata for an NQuin by index.
    pub fn get_tensor_metadata(&self, index: usize) -> Option<&TensorMetadata> {
        self.tensor_metadata.get(index).filter(|_| index < self.len)
    }

    pub fn config(&self) -> &TensorVolumeConfig {
        &self.volume_config
    }

    pub fn update_config(&mut self, config: TensorVolumeConfig) {
        self.volume_config = config;
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.nquins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn tensor_count(&self) -> usize {
        self.as_view().tensor_count()
    }

    pub fn get_tensorized_nquins_into<'b>(
        &'b self,
        out: &mut [TensorizedEntry<'b>],
    ) -> Result<usize, TensorVolumeError> {
        self.as_view().get_tensorized_nquins_into(out)
    }

    pub fn visit_tensorized_nquins<F>(&self, on_entry: F) -> Result<(), TensorVolumeError>
    where
        F: FnMut(TensorizedEntry<'_>) -> ControlFlow<()>,
    {
        self.as_view().visit_tensorized_nquins(on_entry)
    }

    pub fn tensor_search_into(
        &self,
        query: &Tensor10D,
        max_distance: f32,
        out: &mut [usize],
    ) -> Result<usize, TensorVolumeError> {
        self.as_view().tensor_search_into(query, max_distance, out)
    }

    pub fn visit_tensor_search<F>(
        &self,
        query: &Tensor10D,
        max_distance: f32,
        on_match: F,
    ) -> Result<(), TensorVolumeError>
    where
        F: FnMut(usize) -> ControlFlow<()>,
    {
        self.as_view()
            .visit_tensor_search(query, max_distance, on_match)
    }

    pub fn temporal_query_into(
        &self,
        target_t: f32,
        tolerance: f32,
        out: &mut [usize],
    ) -> Result<usize, TensorVolumeError> {
        self.as_view().temporal_query_into(target_t, tolerance, out)
    }

    pub fn visit_temporal_query<F>(
        &self,
        target_t: f32,
        tolerance: f32,
        on_match: F,
    ) -> Result<(), TensorVolumeError>
    where
        F: FnMut(usize) -> ControlFlow<()>,
    {
        self.as_view()
            .visit_temporal_query(target_t, tolerance, on_match)
    }

    pub fn manifold_query_into(
        &self,
        target_w: f32,
        max_distance: f32,
        out: &mut [usize],
    ) -> Result<usize, TensorVolumeError> {
        self.as_view()
            .manifold_query_into(target_w, max_distance, out)
    }

    pub fn visit_manifold_query<F>(
        &self,
        target_w: f32,
        max_distance: f32,
        on_match: F,
    ) -> Result<(), TensorVolumeError>
    where
        F: FnMut(usize) -> ControlFlow<()>,
    {
        self.as_view()
            .visit_manifold_query(target_w, max_distance, on_match)
    }
}

impl<'a> Q42TensorView<'a> {
    pub fn new(
        nquins: &'a [NQuin],
        tensor_metadata: &'a [TensorMetadata],
        volume_config: &'a TensorVolumeConfig,
    ) -> Result<Self, TensorVolumeError> {
        if nquins.len() != tensor_metadata.len() {
            return Err(TensorVolumeError::MismatchedStorage);
        }

        Ok(Self {
            nquins,
            tensor_metadata,
            volume_config,
        })
    }

    pub fn config(&self) -> &'a TensorVolumeConfig {
        self.volume_config
    }

    pub fn len(&self) -> usize {
        self.nquins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nquins.is_empty()
    }

    pub fn tensor_count(&self) -> usize {
        self.tensor_metadata.iter().filter(|m| m.has_tensor).count()
    }

    pub fn get_tensorized_nquins_into(
        &self,
        out: &mut [TensorizedEntry<'a>],
    ) -> Result<usize, TensorVolumeError> {
        let mut written = 0;

        self.visit_tensorized_nquins(|entry| {
            if written >= out.len() {
                return ControlFlow::Break(());
            }

            out[written] = entry;
            written += 1;
            ControlFlow::Continue(())
        })?;

        if written < self.tensor_count() {
            return Err(TensorVolumeError::OutputBufferFull);
        }

        Ok(written)
    }

    pub fn visit_tensorized_nquins<F>(&self, mut on_entry: F) -> Result<(), TensorVolumeError>
    where
        F: FnMut(TensorizedEntry<'a>) -> ControlFlow<()>,
    {
        for (nquin, metadata) in self.nquins.iter().zip(self.tensor_metadata.iter()) {
            if !metadata.has_tensor {
                continue;
            }

            if let ControlFlow::Break(()) = on_entry(TensorizedEntry { nquin, metadata }) {
                break;
            }
        }

        Ok(())
    }

    pub fn tensor_search_into(
        &self,
        query: &Tensor10D,
        max_distance: f32,
        out: &mut [usize],
    ) -> Result<usize, TensorVolumeError> {
        self.collect_matching_indices_into(out, |metadata| {
            metadata.has_tensor && query.full_distance(&metadata.tensor) <= max_distance
        })
    }

    pub fn visit_tensor_search<F>(
        &self,
        query: &Tensor10D,
        max_distance: f32,
        mut on_match: F,
    ) -> Result<(), TensorVolumeError>
    where
        F: FnMut(usize) -> ControlFlow<()>,
    {
        for (index, metadata) in self.tensor_metadata.iter().enumerate() {
            if metadata.has_tensor && query.full_distance(&metadata.tensor) <= max_distance {
                if let ControlFlow::Break(()) = on_match(index) {
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn temporal_query_into(
        &self,
        target_t: f32,
        tolerance: f32,
        out: &mut [usize],
    ) -> Result<usize, TensorVolumeError> {
        self.collect_matching_indices_into(out, |metadata| {
            metadata.has_tensor && (metadata.tensor.t - target_t).abs() <= tolerance
        })
    }

    pub fn visit_temporal_query<F>(
        &self,
        target_t: f32,
        tolerance: f32,
        mut on_match: F,
    ) -> Result<(), TensorVolumeError>
    where
        F: FnMut(usize) -> ControlFlow<()>,
    {
        for (index, metadata) in self.tensor_metadata.iter().enumerate() {
            if metadata.has_tensor && (metadata.tensor.t - target_t).abs() <= tolerance {
                if let ControlFlow::Break(()) = on_match(index) {
                    break;
                }
            }
        }

        Ok(())
    }

    pub fn manifold_query_into(
        &self,
        target_w: f32,
        max_distance: f32,
        out: &mut [usize],
    ) -> Result<usize, TensorVolumeError> {
        self.collect_matching_indices_into(out, |metadata| {
            manifold_matches(metadata, target_w, max_distance)
        })
    }

    pub fn visit_manifold_query<F>(
        &self,
        target_w: f32,
        max_distance: f32,
        mut on_match: F,
    ) -> Result<(), TensorVolumeError>
    where
        F: FnMut(usize) -> ControlFlow<()>,
    {
        for (index, metadata) in self.tensor_metadata.iter().enumerate() {
            if manifold_matches(metadata, target_w, max_distance) {
                if let ControlFlow::Break(()) = on_match(index) {
                    break;
                }
            }
        }

        Ok(())
    }

    fn collect_matching_indices_into<F>(
        &self,
        out: &mut [usize],
        mut predicate: F,
    ) -> Result<usize, TensorVolumeError>
    where
        F: FnMut(&TensorMetadata) -> bool,
    {
        let mut written = 0;

        for (index, metadata) in self.tensor_metadata.iter().enumerate() {
            if !predicate(metadata) {
                continue;
            }

            if written >= out.len() {
                return Err(TensorVolumeError::OutputBufferFull);
            }

            out[written] = index;
            written += 1;
        }

        Ok(written)
    }
}

fn manifold_matches(metadata: &TensorMetadata, target_w: f32, max_distance: f32) -> bool {
    if !metadata.has_tensor {
        return false;
    }

    let w_diff = (metadata.tensor.w - target_w).abs();
    if w_diff > 0.1 {
        return false;
    }

    let query = Tensor10D::new(
        0.0,
        metadata.tensor.v,
        target_w,
        metadata.tensor.x,
        metadata.tensor.y,
        metadata.tensor.z,
        metadata.tensor.t,
        metadata.tensor.alpha,
        metadata.tensor.mu,
        metadata.tensor.sigma,
    );

    query.spatial_distance(&metadata.tensor) <= max_distance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tensor_volume_creation() {
        let mut nquins = [NQuin::default(); 4];
        let mut metadata = [TensorMetadata::default(); 4];
        let volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();
        assert!(volume.is_empty());
        assert_eq!(volume.len(), 0);
        assert_eq!(volume.capacity(), 4);
    }

    #[test]
    fn test_add_nquin_with_tensor() {
        let mut nquins = [NQuin::default(); 2];
        let mut metadata = [TensorMetadata::default(); 2];
        let mut volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();

        let nquin = create_test_nquin(1);
        let tensor = Tensor10D::new(0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 0.8, 0.5, 0.3);
        volume.add_nquin_with_tensor(nquin, tensor).unwrap();

        assert_eq!(volume.len(), 1);
        assert_eq!(volume.tensor_count(), 1);
        assert!(volume.get_tensor_metadata(0).unwrap().has_tensor);
    }

    #[test]
    fn test_add_nquin_without_tensor() {
        let mut nquins = [NQuin::default(); 2];
        let mut metadata = [TensorMetadata::default(); 2];
        let mut volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();

        volume.add_nquin(create_test_nquin(2)).unwrap();

        assert_eq!(volume.len(), 1);
        assert_eq!(volume.tensor_count(), 0);
        assert!(!volume.get_tensor_metadata(0).unwrap().has_tensor);
    }

    #[test]
    fn test_storage_capacity_is_bounded() {
        let mut nquins = [NQuin::default(); 1];
        let mut metadata = [TensorMetadata::default(); 1];
        let mut volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();

        volume.add_nquin(create_test_nquin(1)).unwrap();
        let result = volume.add_nquin(create_test_nquin(2));

        assert_eq!(result, Err(TensorVolumeError::StorageCapacityExceeded));
    }

    #[test]
    fn test_tensor_search_into() {
        let mut nquins = [NQuin::default(); 8];
        let mut metadata = [TensorMetadata::default(); 8];
        let mut volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();
        seed_linear_tensor_points(&mut volume);

        let query = Tensor10D::new(0.0, 0.0, 0.0, 2.0, 2.0, 0.0, 2.0, 1.0, 0.0, 0.0);
        let mut out = [usize::MAX; 2];
        let written = volume.tensor_search_into(&query, 0.5, &mut out).unwrap();

        assert_eq!(written, 1);
        assert_eq!(out[0], 2);
    }

    #[test]
    fn test_tensor_search_into_reports_overflow() {
        let mut nquins = [NQuin::default(); 4];
        let mut metadata = [TensorMetadata::default(); 4];
        let mut volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();

        for i in 0..3 {
            let tensor = Tensor10D::new(0.0, 0.0, 0.0, i as f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
            volume
                .add_nquin_with_tensor(create_test_nquin(i), tensor)
                .unwrap();
        }

        let query = Tensor10D::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let mut out = [usize::MAX; 1];
        let result = volume.tensor_search_into(&query, 1.5, &mut out);

        assert_eq!(result, Err(TensorVolumeError::OutputBufferFull));
    }

    #[test]
    fn test_temporal_query_into() {
        let mut nquins = [NQuin::default(); 8];
        let mut metadata = [TensorMetadata::default(); 8];
        let mut volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();

        for i in 0..5 {
            let tensor = Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, i as f32, 1.0, 0.0, 0.0);
            volume
                .add_nquin_with_tensor(create_test_nquin(i), tensor)
                .unwrap();
        }

        let mut out = [usize::MAX; 1];
        let written = volume.temporal_query_into(2.0, 0.1, &mut out).unwrap();

        assert_eq!(written, 1);
        assert_eq!(out[0], 2);
    }

    #[test]
    fn test_manifold_query_into_reports_overflow() {
        let mut nquins = [NQuin::default(); 8];
        let mut metadata = [TensorMetadata::default(); 8];
        let mut volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();

        for i in 0..5 {
            let w = (i % 3) as f32;
            let tensor = Tensor10D::new(0.0, 0.0, w, i as f32, i as f32, 0.0, 0.0, 1.0, 0.0, 0.0);
            volume
                .add_nquin_with_tensor(create_test_nquin(i), tensor)
                .unwrap();
        }

        let mut out = [usize::MAX; 1];
        let result = volume.manifold_query_into(1.0, 5.0, &mut out);

        assert_eq!(result, Err(TensorVolumeError::OutputBufferFull));
    }

    #[test]
    fn test_get_tensorized_nquins_into() {
        let mut nquins = [NQuin::default(); 4];
        let mut metadata = [TensorMetadata::default(); 4];
        let mut volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();
        let placeholder_nquin = NQuin::default();
        let placeholder_metadata = TensorMetadata::default();

        volume.add_nquin(create_test_nquin(0)).unwrap();
        volume
            .add_nquin_with_tensor(
                create_test_nquin(1),
                Tensor10D::new(0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, 0.0),
            )
            .unwrap();

        let mut out = [TensorizedEntry {
            nquin: &placeholder_nquin,
            metadata: &placeholder_metadata,
        }; 1];
        let written = volume.get_tensorized_nquins_into(&mut out).unwrap();

        assert_eq!(written, 1);
        assert_eq!(out[0].nquin.subject, 1);
        assert!(out[0].metadata.has_tensor);
    }

    #[test]
    fn test_visit_tensor_search_stops_when_callback_breaks() {
        let mut nquins = [NQuin::default(); 8];
        let mut metadata = [TensorMetadata::default(); 8];
        let mut volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();

        for i in 0..4 {
            let tensor = Tensor10D::new(0.0, 0.0, 0.0, i as f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
            volume
                .add_nquin_with_tensor(create_test_nquin(i), tensor)
                .unwrap();
        }

        let query = Tensor10D::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0);
        let mut first_hit = usize::MAX;
        let mut count = 0usize;
        volume
            .visit_tensor_search(&query, 3.0, |index| {
                first_hit = index;
                count += 1;
                ControlFlow::Break(())
            })
            .unwrap();

        assert_eq!(count, 1);
        assert_eq!(first_hit, 0);
    }

    #[test]
    fn test_clear_scrubs_used_prefix() {
        let mut nquins = [NQuin::default(); 2];
        let mut metadata = [TensorMetadata::default(); 2];
        let mut volume = Q42TensorVolume::new(&mut nquins, &mut metadata).unwrap();

        volume
            .add_nquin_with_tensor(
                create_test_nquin(7),
                Tensor10D::new(0.0, 0.0, 0.0, 7.0, 7.0, 0.0, 7.0, 1.0, 0.0, 0.0),
            )
            .unwrap();
        volume.clear();

        assert_eq!(volume.len(), 0);
        assert_eq!(volume.nquins[0], NQuin::default());
        assert_eq!(volume.tensor_metadata[0], TensorMetadata::default());
    }

    #[test]
    fn test_q42_tensor_view_rejects_mismatched_storage() {
        let nquins = [create_test_nquin(1)];
        let metadata = [TensorMetadata::default(), TensorMetadata::default()];
        let config = TensorVolumeConfig::default();

        let result = Q42TensorView::new(&nquins, &metadata, &config);

        assert!(matches!(result, Err(TensorVolumeError::MismatchedStorage)));
    }

    fn seed_linear_tensor_points(volume: &mut Q42TensorVolume<'_>) {
        for i in 0..5 {
            let tensor = Tensor10D::new(
                0.0, 0.0, 0.0, i as f32, i as f32, 0.0, i as f32, 1.0, 0.0, 0.0,
            );
            volume
                .add_nquin_with_tensor(create_test_nquin(i), tensor)
                .unwrap();
        }
    }

    fn create_test_nquin(id: u64) -> NQuin {
        NQuin {
            subject: id,
            predicate: id,
            object: id,
            context: id,
            metadata: (id << 32) | id,
            parity: 0,
        }
    }
}
