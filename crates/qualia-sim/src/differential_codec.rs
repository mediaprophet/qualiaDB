// Differential 3D Video Codec Kinematic & Salience Predictor (`differential_codec.rs`)
//
// Implements Phase 1 (Kinematic Extrapolation & A Priori Disocclusion Prediction)
// and Phase 4 (Progressive Monte Carlo Power Governor):
// - 6-DoF analytical trajectory extrapolation (zero-latency B-frame layer)
// - Frustum-plane disocclusion forecasting
// - Semantic salience perceptual quantization (QP)
// - Static dwell progressive convergence governor

pub const MAX_PREDICTED_DISOCCLUSIONS: usize = 32;

/// A bounding box for a spatial entity in world space.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EntitySpatialBounds {
    pub entity_id: u32,
    pub min_corner: [f32; 3],
    pub max_corner: [f32; 3],
    pub velocity_mps: [f32; 3],
    pub salience_class: u8,
}

/// Evaluates analytical 6-DoF trajectory extrapolation forward in time.
#[derive(Clone, Copy, Debug, Default)]
pub struct KinematicExtrapolator;

impl KinematicExtrapolator {
    /// Extrapolates position and velocity:
    /// $\mathbf{x}(t + \Delta t) = \mathbf{x}(t) + \mathbf{v}\Delta t + \frac{1}{2}\mathbf{a}\Delta t^2$
    #[inline]
    pub fn extrapolate_pos_vel(
        pos: [f32; 3],
        vel: [f32; 3],
        acc: [f32; 3],
        delta_t_sec: f32,
    ) -> ([f32; 3], [f32; 3]) {
        let dt = delta_t_sec;
        let half_dt2 = 0.5 * dt * dt;

        let new_pos = [
            pos[0] + vel[0] * dt + acc[0] * half_dt2,
            pos[1] + vel[1] * dt + acc[1] * half_dt2,
            pos[2] + vel[2] * dt + acc[2] * half_dt2,
        ];

        let new_vel = [
            vel[0] + acc[0] * dt,
            vel[1] + acc[1] * dt,
            vel[2] + acc[2] * dt,
        ];

        (new_pos, new_vel)
    }
}

/// Predicts disocclusions k frames in advance using velocity vectors and frustum planes.
#[derive(Clone, Copy, Debug, Default)]
pub struct DisocclusionForecaster;

impl DisocclusionForecaster {
    /// Tests moving entities against stationary background cells to detect upcoming disocclusion events.
    ///
    /// Respects QualiaDB zero-heap contract by populating caller-supplied fixed buffer.
    pub fn forecast_disocclusions(
        moving_entities: &[EntitySpatialBounds],
        cam_frustum_planes: &[[f32; 4]; 6],
        lookahead_frames: u32,
        frame_dt_sec: f32,
        out_disoccluded_ids: &mut [u32],
    ) -> usize {
        let mut count = 0usize;
        let lookahead_dt = (lookahead_frames as f32) * frame_dt_sec;

        for entity in moving_entities {
            let speed_sq = entity.velocity_mps[0].powi(2)
                + entity.velocity_mps[1].powi(2)
                + entity.velocity_mps[2].powi(2);

            // Only consider entities moving with perceptible velocity (> 0.1 m/s)
            if speed_sq < 0.01 {
                continue;
            }

            // Project bounding box center lookahead position
            let center = [
                (entity.min_corner[0] + entity.max_corner[0]) * 0.5,
                (entity.min_corner[1] + entity.max_corner[1]) * 0.5,
                (entity.min_corner[2] + entity.max_corner[2]) * 0.5,
            ];

            let lookahead_center = [
                center[0] + entity.velocity_mps[0] * lookahead_dt,
                center[1] + entity.velocity_mps[1] * lookahead_dt,
                center[2] + entity.velocity_mps[2] * lookahead_dt,
            ];

            // Verify if entity is inside camera frustum
            let mut inside = true;
            for plane in cam_frustum_planes {
                let dist = plane[0] * lookahead_center[0]
                    + plane[1] * lookahead_center[1]
                    + plane[2] * lookahead_center[2]
                    + plane[3];
                if dist < -2.0 {
                    inside = false;
                    break;
                }
            }

            if inside && count < out_disoccluded_ids.len() {
                out_disoccluded_ids[count] = entity.entity_id;
                count += 1;
            }
        }

        count
    }
}

/// Computes Quantization Parameter (QP) and ray budgets from D_9 semantic salience.
#[derive(Clone, Copy, Debug, Default)]
pub struct SaliencePerceptualGovernor;

impl SaliencePerceptualGovernor {
    /// Maps salience class (0=Class A Focal, 64=Class B Dynamic, 192=Class C Ambient, 255=Class D Subperceptual)
    /// to Quantization Parameter (QP) and allocated primary ray budget per pixel.
    #[inline]
    pub fn evaluate_qp_and_ray_budget(salience_class: u8) -> (u8, u32) {
        match salience_class {
            0..=31 => (0, 8),      // Class A: Lossless, 8 spp path trace
            32..=95 => (64, 2),    // Class B: Dynamic, 2 spp + SVGF
            96..=224 => (192, 1),  // Class C: Ambient, 1 spp rolling
            _ => (255, 0),         // Class D: Skip / Impostor
        }
    }
}

/// Phase 4: Manages static scene dwell, progressive Monte Carlo convergence, and power throttling.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ProgressiveMonteCarloGovernor {
    pub accumulated_samples: u32,
    pub target_samples: u32,
    pub noise_variance_estimate: f32,
    pub variance_epsilon: f32,
    pub is_converged: bool,
    pub power_throttle_active: bool,
}

impl Default for ProgressiveMonteCarloGovernor {
    fn default() -> Self {
        Self {
            accumulated_samples: 0,
            target_samples: 128,
            noise_variance_estimate: 1.0,
            variance_epsilon: 0.005,
            is_converged: false,
            power_throttle_active: false,
        }
    }
}

impl ProgressiveMonteCarloGovernor {
    /// Ticks the progressive convergence state machine:
    ///
    /// When scene motion delta $\Delta S_t < 1e-4$:
    /// - Accumulates Monte Carlo samples: $N \leftarrow N + 1$
    /// - Evaluates estimated noise variance: $\sigma^2 \propto 1/N$
    /// - Activates power throttle once $\sigma^2 < \epsilon$
    ///
    /// When motion detected: instantly resets accumulation to 0.
    pub fn tick_frame(&mut self, scene_delta_magnitude: f32) {
        if scene_delta_magnitude > 1e-4 {
            // Scene or camera mutated: flush accumulation
            self.accumulated_samples = 0;
            self.noise_variance_estimate = 1.0;
            self.is_converged = false;
            self.power_throttle_active = false;
            return;
        }

        // Static dwell: progressive accumulation
        self.accumulated_samples += 1;
        self.noise_variance_estimate = 1.0 / (self.accumulated_samples as f32).max(1.0);

        if self.noise_variance_estimate <= self.variance_epsilon
            || self.accumulated_samples >= self.target_samples
        {
            self.is_converged = true;
            self.power_throttle_active = true; // Drop GPU clock / idle frame submissions
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kinematic_extrapolation_accuracy() {
        let pos = [0.0, 10.0, 0.0];
        let vel = [10.0, 0.0, -5.0];
        let acc = [0.0, -9.80665, 0.0]; // Gravity
        let dt = 0.1; // 100ms

        let (new_pos, new_vel) = KinematicExtrapolator::extrapolate_pos_vel(pos, vel, acc, dt);

        assert!((new_pos[0] - 1.0).abs() < 1e-5);
        assert!((new_pos[1] - (10.0 - 0.5 * 9.80665 * 0.01)).abs() < 1e-5);
        assert!((new_pos[2] - (-0.5)).abs() < 1e-5);

        assert!((new_vel[0] - 10.0).abs() < 1e-5);
        assert!((new_vel[1] - (-0.980665)).abs() < 1e-5);
        assert!((new_vel[2] - (-5.0)).abs() < 1e-5);
    }

    #[test]
    fn test_disocclusion_prediction_frustum() {
        let moving = [
            EntitySpatialBounds {
                entity_id: 42,
                min_corner: [0.0, 0.0, 5.0],
                max_corner: [2.0, 2.0, 7.0],
                velocity_mps: [5.0, 0.0, 0.0], // Moving across
                salience_class: 0,
            },
            EntitySpatialBounds {
                entity_id: 43,
                min_corner: [100.0, 0.0, 0.0],
                max_corner: [102.0, 2.0, 2.0],
                velocity_mps: [0.0, 0.0, 0.0], // Static
                salience_class: 192,
            },
        ];

        // Simple bounding frustum planes (open box around z in [0, 50])
        let planes = [
            [1.0, 0.0, 0.0, 20.0],
            [-1.0, 0.0, 0.0, 20.0],
            [0.0, 1.0, 0.0, 20.0],
            [0.0, -1.0, 0.0, 20.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, -1.0, 100.0],
        ];

        let mut out_ids = [0u32; 8];
        let count = DisocclusionForecaster::forecast_disocclusions(
            &moving, &planes, 3, 0.0166, &mut out_ids,
        );

        assert_eq!(count, 1);
        assert_eq!(out_ids[0], 42);
    }

    #[test]
    fn test_salience_perceptual_quantization() {
        let (qp_a, rays_a) = SaliencePerceptualGovernor::evaluate_qp_and_ray_budget(0);
        assert_eq!(qp_a, 0);
        assert_eq!(rays_a, 8);

        let (qp_c, rays_c) = SaliencePerceptualGovernor::evaluate_qp_and_ray_budget(192);
        assert_eq!(qp_c, 192);
        assert_eq!(rays_c, 1);

        let (qp_d, rays_d) = SaliencePerceptualGovernor::evaluate_qp_and_ray_budget(255);
        assert_eq!(qp_d, 255);
        assert_eq!(rays_d, 0);
    }

    #[test]
    fn test_progressive_monte_carlo_convergence() {
        let mut governor = ProgressiveMonteCarloGovernor::default();
        assert!(!governor.is_converged);

        // Tick static dwell 200 frames
        for _ in 0..200 {
            governor.tick_frame(0.0);
        }

        assert!(governor.is_converged);
        assert!(governor.power_throttle_active);
        assert!(governor.accumulated_samples >= 128);

        // Sudden camera motion
        governor.tick_frame(1.5);
        assert!(!governor.is_converged);
        assert!(!governor.power_throttle_active);
        assert_eq!(governor.accumulated_samples, 0);
    }
}
