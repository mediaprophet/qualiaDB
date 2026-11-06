//! Zero-Heap 3D Spatial Audio & Doppler Engine (`spatial.rs`)
//!
//! Provides deterministic 3D spatial sound processing delegated to [`etalon_signal::spatial`]:
//! - Distance attenuation models (Inverse, Linear, Exponential)
//! - Directional emission cones (inner angle, outer angle, outer attenuation)
//! - Doppler frequency pitch shifting based on relative emitter-listener velocities
//! - Equal-power stereo binaural panning and interaural level/time differences
//! - Caller-buffered zero-heap mixing
//!
//! Designed according to `AGENTS.md` §0 (Zero heap in hot paths).

pub use etalon_signal::spatial::{
    mix_spatial_voice_into, mix_spatial_voice_occluded_into, DistanceModel, SpatialEmitter,
    SpatialListener, SpatialOcclusionBridge, SpatialOcclusionFilterState, SpatialVoiceParams,
    MAX_SPATIAL_VOICES, SPEED_OF_SOUND_AIR,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_attenuation() {
        let model = DistanceModel::Inverse {
            ref_distance: 1.0,
            max_distance: 50.0,
            rolloff_factor: 1.0,
        };

        let g1 = model.calculate_gain(1.0);
        let g2 = model.calculate_gain(2.0);
        let g10 = model.calculate_gain(10.0);

        assert!((g1 - 1.0).abs() < 1e-5);
        assert!((g2 - 0.5).abs() < 1e-4);
        assert!(g10 < g2);
    }

    #[test]
    fn test_doppler_shift() {
        let mut emitter = SpatialEmitter::default();
        emitter.position = [0.0, 0.0, 100.0];
        // Moving towards listener at 34.3 m/s (~10% speed of sound)
        emitter.velocity = [0.0, 0.0, -34.32];

        let listener = SpatialListener::default();
        let params = emitter.evaluate_spatial(&listener);

        // When approaching, pitch multiplier must be > 1.0 (approx 1.11)
        assert!(params.pitch_multiplier > 1.05);
    }

    #[test]
    fn test_stereo_panning() {
        let mut emitter = SpatialEmitter::default();
        // Place emitter directly to the right
        emitter.position = [10.0, 0.0, 0.0];

        let listener = SpatialListener::default();
        let params = emitter.evaluate_spatial(&listener);

        // Right channel must be significantly louder than left
        assert!(params.gain_right > params.gain_left);
    }

    #[test]
    fn test_pga_motor_listener_and_occluded_mixer_bridge() {
        // Pure translation motor by [0.0, 2.0, -5.0] in PGA blade4 encoding
        let r = [1.0, 0.0, 0.0, 0.0];
        let d = [0.0, 2.5, -1.0, 0.0]; // blade4: [w, -z, -y, -x] for half-translation [0, 1, -2.5]
        let listener = SpatialListener::from_pga_motor_blades(r, d, [0.0, 0.0, 0.0]);
        assert!((listener.position[1] - 2.0).abs() < 1e-4);
        assert!((listener.position[2] + 5.0).abs() < 1e-4);

        let mut emitter = SpatialEmitter::default();
        emitter.position = [4.0, 2.0, -5.0];
        let occlusion = SpatialOcclusionBridge {
            direct_gain: 0.4,
            lowpass_cutoff_hz: 900.0,
            diffraction_phase_rad: 0.25,
            reverb_send: 0.35,
        };
        let params = emitter.evaluate_spatial_with_occlusion(&listener, &occlusion);
        assert!(params.gain_right > params.gain_left);

        let mono = [0.8_f32; 32];
        let mut st = SpatialOcclusionFilterState::default();
        let mut direct = [[0.0_f32; 2]; 32];
        let mut reverb = [[0.0_f32; 2]; 32];
        mix_spatial_voice_occluded_into(
            &mono,
            &params,
            &occlusion,
            48_000.0,
            &mut st,
            &mut direct,
            &mut reverb,
        );
        assert!(direct[31][1] > direct[31][0]);
        assert!(reverb[31][1] > 0.0);
    }
}
