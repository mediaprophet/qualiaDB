//! GPU Uniform/Voxel Packer for EMF, Acoustic & Physical Field Cues (`BS-40`).
//!
//! Provides zero-heap GPU layout translation for physical field cues into 64-byte
//! flat `repr(C)` descriptors for WebGPU / Etalon storage and uniform buffers.
//! Enforces honest calibration flags: uncalibrated bands are either rejected or
//! tagged with `uncalibrated_flag = 1` for warning overlay rendering.
//!
//! Strictly zero heap in hot paths (Tier-1 contract, `AGENTS.md` §0-A).

use bytemuck::{Pod, Zeroable};
use crate::modalities::typed_cue_profile::{CueEvaluationError, PhysicalCueEmission};
#[cfg(test)]
use crate::modalities::typed_cue_profile::{CalibrationStatus, PhysicalCueDomain, PhysicalUnitKind};

/// 64-byte GPU storage/uniform buffer descriptor for physical cues.
///
/// Layout is aligned to 8 bytes and exactly 64 bytes in size for direct GPU buffer upload.
/// `uncalibrated_flag = 1` triggers HUD warning badges for uncalibrated overlay modes.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct EmfAcousticGpuDescriptor {
    pub source_entity_did: u64,
    pub source_organ_id: u64,
    pub band_min: f32,
    pub band_max: f32,
    pub magnitude: f32,
    pub polarization_deg: f32,
    pub domain_code: u32,
    pub unit_code: u32,
    pub calibration_code: u32,
    pub uncalibrated_flag: u32,
    pub _reserved: [u32; 4],
}

const _: () = {
    assert!(std::mem::size_of::<EmfAcousticGpuDescriptor>() == 64);
    assert!(std::mem::align_of::<EmfAcousticGpuDescriptor>() == 8);
};

impl Default for EmfAcousticGpuDescriptor {
    fn default() -> Self {
        Self {
            source_entity_did: 0,
            source_organ_id: 0,
            band_min: 0.0,
            band_max: 0.0,
            magnitude: 0.0,
            polarization_deg: -1.0,
            domain_code: 0,
            unit_code: 0,
            calibration_code: 0,
            uncalibrated_flag: 0,
            _reserved: [0; 4],
        }
    }
}

impl EmfAcousticGpuDescriptor {
    #[inline(always)]
    pub fn is_uncalibrated(&self) -> bool {
        self.uncalibrated_flag != 0
    }
}

/// Packs physical cue emissions into GPU descriptors with zero heap allocations.
///
/// # Mandatory BS-40 Negative Cases
/// - If `allow_uncalibrated_overlay == false` and any cue in `cues` is uncalibrated
///   (`UncalibratedTheoretical` or `OutOfSensorRange`), returns `Err(CueEvaluationError::UncalibratedBand)`.
/// - If `allow_uncalibrated_overlay == true`, uncalibrated cues are packed with `uncalibrated_flag = 1`.
/// - If `out_descriptors.len() < cues.len()`, returns `Err(CueEvaluationError::BufferOverflow)`.
pub fn pack_calibrated_cues_for_gpu_zero_heap(
    cues: &[PhysicalCueEmission],
    allow_uncalibrated_overlay: bool,
    out_descriptors: &mut [EmfAcousticGpuDescriptor],
) -> Result<usize, CueEvaluationError> {
    if out_descriptors.len() < cues.len() {
        return Err(CueEvaluationError::BufferOverflow);
    }

    for (i, cue) in cues.iter().enumerate() {
        let is_cal = cue.calibration.is_calibrated();
        let uncalibrated_flag = if !is_cal {
            if !allow_uncalibrated_overlay {
                return Err(CueEvaluationError::UncalibratedBand);
            }
            1u32
        } else {
            0u32
        };

        let pol = cue.polarization_deg.unwrap_or(-1.0);

        out_descriptors[i] = EmfAcousticGpuDescriptor {
            source_entity_did: cue.source_entity_did,
            source_organ_id: cue.source_organ_id,
            band_min: cue.band_min as f32,
            band_max: cue.band_max as f32,
            magnitude: cue.magnitude as f32,
            polarization_deg: pol,
            domain_code: cue.domain as u32,
            unit_code: cue.unit as u32,
            calibration_code: cue.calibration as u32,
            uncalibrated_flag,
            _reserved: [0; 4],
        };
    }

    Ok(cues.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    fn sample_calibrated_cues() -> [PhysicalCueEmission; 2] {
        [
            PhysicalCueEmission {
                source_entity_did: 0x1111_2222_3333_4444,
                source_organ_id: 10,
                domain: PhysicalCueDomain::ElectromagneticOptical,
                unit: PhysicalUnitKind::WattsPerSquareMeterIrradiance,
                band_min: 400.0,
                band_max: 700.0,
                magnitude: 120.0,
                polarization_deg: Some(45.0),
                calibration: CalibrationStatus::LaboratoryCalibrated,
            },
            PhysicalCueEmission {
                source_entity_did: 0x5555_6666_7777_8888,
                source_organ_id: 20,
                domain: PhysicalCueDomain::AcousticMechanical,
                unit: PhysicalUnitKind::DecibelSplAcoustic,
                band_min: 20.0,
                band_max: 20000.0,
                magnitude: 65.0,
                polarization_deg: None,
                calibration: CalibrationStatus::FieldCalibrated,
            },
        ]
    }

    #[test]
    fn test_descriptor_layout_and_size() {
        assert_eq!(std::mem::size_of::<EmfAcousticGpuDescriptor>(), 64);
        assert_eq!(std::mem::align_of::<EmfAcousticGpuDescriptor>(), 8);

        let desc = EmfAcousticGpuDescriptor::default();
        let bytes = bytemuck::bytes_of(&desc);
        assert_eq!(bytes.len(), 64);

        let recovered: &EmfAcousticGpuDescriptor = bytemuck::from_bytes(bytes);
        assert_eq!(recovered, &desc);
        assert!(!recovered.is_uncalibrated());
    }

    #[test]
    fn test_pack_calibrated_cues_success() {
        let cues = sample_calibrated_cues();
        let mut out = [EmfAcousticGpuDescriptor::default(); 2];

        let count = pack_calibrated_cues_for_gpu_zero_heap(&cues, false, &mut out).unwrap();
        assert_eq!(count, 2);

        assert_eq!(out[0].source_entity_did, 0x1111_2222_3333_4444);
        assert_eq!(out[0].domain_code, PhysicalCueDomain::ElectromagneticOptical as u32);
        assert_eq!(out[0].unit_code, PhysicalUnitKind::WattsPerSquareMeterIrradiance as u32);
        assert_eq!(out[0].magnitude, 120.0);
        assert_eq!(out[0].polarization_deg, 45.0);
        assert_eq!(out[0].uncalibrated_flag, 0);
        assert!(!out[0].is_uncalibrated());

        assert_eq!(out[1].source_entity_did, 0x5555_6666_7777_8888);
        assert_eq!(out[1].domain_code, PhysicalCueDomain::AcousticMechanical as u32);
        assert_eq!(out[1].unit_code, PhysicalUnitKind::DecibelSplAcoustic as u32);
        assert_eq!(out[1].magnitude, 65.0);
        assert_eq!(out[1].polarization_deg, -1.0);
        assert_eq!(out[1].uncalibrated_flag, 0);
        assert!(!out[1].is_uncalibrated());
    }

    #[test]
    fn test_pack_uncalibrated_disallowed_fails_closed() {
        let mut cues = sample_calibrated_cues();
        cues[0].calibration = CalibrationStatus::UncalibratedTheoretical;
        let mut out = [EmfAcousticGpuDescriptor::default(); 2];

        let res = pack_calibrated_cues_for_gpu_zero_heap(&cues, false, &mut out);
        assert_eq!(res, Err(CueEvaluationError::UncalibratedBand));

        cues[0].calibration = CalibrationStatus::OutOfSensorRange;
        let res2 = pack_calibrated_cues_for_gpu_zero_heap(&cues, false, &mut out);
        assert_eq!(res2, Err(CueEvaluationError::UncalibratedBand));
    }

    #[test]
    fn test_pack_uncalibrated_allowed_sets_flag() {
        let mut cues = sample_calibrated_cues();
        cues[0].calibration = CalibrationStatus::UncalibratedTheoretical;
        let mut out = [EmfAcousticGpuDescriptor::default(); 2];

        let count = pack_calibrated_cues_for_gpu_zero_heap(&cues, true, &mut out).unwrap();
        assert_eq!(count, 2);

        assert_eq!(out[0].uncalibrated_flag, 1);
        assert!(out[0].is_uncalibrated());
        assert_eq!(out[1].uncalibrated_flag, 0);
        assert!(!out[1].is_uncalibrated());
    }

    #[test]
    fn test_pack_buffer_overflow() {
        let cues = sample_calibrated_cues();
        let mut small_out = [EmfAcousticGpuDescriptor::default(); 1];

        let res = pack_calibrated_cues_for_gpu_zero_heap(&cues, false, &mut small_out);
        assert_eq!(res, Err(CueEvaluationError::BufferOverflow));
    }

    #[test]
    fn test_zero_alloc_gpu_packing() {
        let cues = sample_calibrated_cues();
        let mut out = [EmfAcousticGpuDescriptor::default(); 2];

        assert_zero_alloc("pack_calibrated_cues_for_gpu", || {
            let count = pack_calibrated_cues_for_gpu_zero_heap(&cues, false, &mut out).unwrap();
            assert_eq!(count, 2);
        });

        assert_zero_alloc("pack_uncalibrated_overlay_for_gpu", || {
            let mut uncal = cues;
            uncal[0].calibration = CalibrationStatus::UncalibratedTheoretical;
            let count = pack_calibrated_cues_for_gpu_zero_heap(&uncal, true, &mut out).unwrap();
            assert_eq!(count, 2);
        });
    }
}
