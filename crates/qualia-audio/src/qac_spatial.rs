//! QAC Spatial Audio, Higher-Order Ambisonics (HOA) Bus, Wigner-D Rotation,
//! Binaural HRIR/RIR Rendering, ITU-R BS.1770-5 Loudness & Modal Resonance Adapters
//! (`qac_spatial.rs`).
//!
//! Bridges `etalon_signal::{hoa, bs1770, modal}` into `qualia-audio` with caller-buffered,
//! zero-heap scene buses and loudness/modal sessions.

pub use etalon_signal::bs1770::{
    calibrated_rms_db_spl, gate_block_energies_bs1770, measure_bs1770_integrated_loudness,
    measure_true_peak_4x_f32, new_k_weighting_cascade, Bs1770ChannelRole, Bs1770LoudnessReading,
    CalibratedSplReading, QualityStatus, TruePeakReading, AIR_REF_PRESSURE_PA,
    BS1770_ABSOLUTE_GATE_LUFS, BS1770_RELATIVE_GATE_OFFSET_LU, MAX_BS1770_CHANNELS,
    UNDERWATER_REF_PRESSURE_PA,
};
pub use etalon_signal::hoa::{
    acn_degree_order, acn_index, admit_and_derive_unit_quaternion_f64,
    convert_hoa_normalization_in_place_f64, convert_hoa_stream_normalization_in_place_f32,
    decode_hoa_frame_to_direction_f64, decode_hoa_stream_to_speakers_into,
    encode_mono_to_hoa_accum_into, evaluate_hoa_basis_f32, evaluate_hoa_basis_f64,
    hamilton_product_xyzw, hoa_decode_matrix_apply_into, max_re_degree_weight_f64,
    n3d_over_sn3d_factor_f64, quaternion_conjugate_xyzw, quaternion_ordered_norm_sq_f64,
    rotate_symmetric_tensor_active_f64, rotate_vector_active_f64,
    rotation_matrix_from_quaternion_f64, sn3d_over_n3d_factor_f64,
    spherical_to_unit_cartesian_f64, HoaDecodeShape, HoaNormalization, HoaOrder,
    HoaWignerRotation, MAX_HOA_CHANNELS, MAX_HOA_ORDER, QUATERNION_NORM_SQ_TOLERANCE,
};
pub use etalon_signal::modal::{
    apply_linear_fir_filter_into, convolve_hrir_with_additional_delay_into,
    convolve_stereo_hrir_with_additional_delays_into, estimate_pitch_yin_with_status,
    evaluate_modal_bank_at_time_f64, extract_mel_mfcc_frame_into, fir_dc_and_nyquist_gain_f64,
    impulse_first_moment_tap_f64, modal_t60_seconds, normal_incidence_fluid_interface_f64,
    place_cubic_lagrange_fractional_impulse, ray_arrival_delay_taps_f64,
    render_hoa_stream_binaural_into, FluidImpedanceInterface, IdealAcousticBoundary,
    ModalOscillatorState, ModalResonanceRecord, PitchReading, LN_1000,
    MODAL_RESONANCE_RECORD_BYTES, OCTAHEDRAL_VIRTUAL_SPEAKERS_XYZ,
};
pub use etalon_signal::SignalError;

/// Zero-heap Higher-Order Ambisonics (HOA) scene bus for `qualia-audio` sessions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct QacHoaSceneBus {
    pub order: HoaOrder,
    pub normalization: HoaNormalization,
    pub listener_rotation: HoaWignerRotation,
    pub reference_freq_hz: f32,
}

impl QacHoaSceneBus {
    /// Construct a new HOA scene bus with identity listener orientation.
    pub fn new(
        order: HoaOrder,
        normalization: HoaNormalization,
        reference_freq_hz: f32,
    ) -> Result<Self, SignalError> {
        if !reference_freq_hz.is_finite() || reference_freq_hz <= 0.0 {
            return Err(SignalError::InvalidParameter);
        }
        let listener_rotation =
            HoaWignerRotation::from_quaternion_xyzw([0.0, 0.0, 0.0, 1.0])?;
        Ok(Self {
            order,
            normalization,
            listener_rotation,
            reference_freq_hz,
        })
    }

    /// Update the active scene rotation from an admitted Hamilton quaternion `xyzw`.
    pub fn set_rotation_quaternion_xyzw(&mut self, xyzw: [f64; 4]) -> Result<(), SignalError> {
        self.listener_rotation = HoaWignerRotation::from_quaternion_xyzw(xyzw)?;
        Ok(())
    }

    /// Encode a mono source at `direction_xyz` into `hoa_bus_interleaved`, rotate by
    /// `self.listener_rotation` into `rotated_hoa_scratch`, and render stereo binaural output
    /// into `out_stereo`.
    pub fn encode_rotate_and_render_binaural_into(
        &self,
        mono_samples: &[f32],
        direction_xyz: [f32; 3],
        gain: f32,
        hoa_bus_interleaved: &mut [f32],
        rotated_hoa_scratch: &mut [f32],
        out_stereo: &mut [[f32; 2]],
    ) -> Result<usize, SignalError> {
        let ch = self.order.channel_count();
        let needed = mono_samples
            .len()
            .checked_mul(ch)
            .ok_or(SignalError::NumericRange)?;
        if hoa_bus_interleaved.len() < needed || rotated_hoa_scratch.len() < needed {
            return Err(SignalError::LengthMismatch);
        }
        hoa_bus_interleaved[..needed].fill(0.0);
        encode_mono_to_hoa_accum_into(
            mono_samples,
            self.order,
            self.normalization,
            direction_xyz,
            gain,
            &mut hoa_bus_interleaved[..needed],
        )?;
        self.listener_rotation.apply_stream_into_f32(
            self.order,
            &hoa_bus_interleaved[..needed],
            &mut rotated_hoa_scratch[..needed],
        )?;
        render_hoa_stream_binaural_into(
            self.order,
            self.normalization,
            &rotated_hoa_scratch[..needed],
            self.reference_freq_hz,
            out_stereo,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qac_hoa_scene_bus_rotates_and_renders_stereo() {
        let mut bus = QacHoaSceneBus::new(HoaOrder::Order1, HoaNormalization::Sn3d, 1000.0).unwrap();
        let s = core::f64::consts::FRAC_1_SQRT_2;
        // Rotate +90 deg around +Z so a +X source moves to +Y (left ear)
        bus.set_rotation_quaternion_xyzw([0.0, 0.0, s, s]).unwrap();

        let mono = [1.0f32; 16];
        let mut hoa_buf = [0.0f32; 64];
        let mut rot_buf = [0.0f32; 64];
        let mut stereo = [[0.0f32; 2]; 16];
        let frames = bus
            .encode_rotate_and_render_binaural_into(
                &mono,
                [1.0, 0.0, 0.0],
                1.0,
                &mut hoa_buf,
                &mut rot_buf,
                &mut stereo,
            )
            .unwrap();
        assert_eq!(frames, 16);
        assert!(stereo[0][0] > stereo[0][1]);
    }
}
