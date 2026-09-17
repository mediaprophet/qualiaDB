use super::*;
use crate::modalities::manifold::ManifoldCoordinate10D;

/// Speed of light in vacuum (m/s).
const C_LIGHT: f64 = 299_792_458.0;

/// Minimum distance to avoid singularity in 1/r falloff.
const MIN_DIST: f64 = 1e-12;

/// One EMF source: position, amplitude, frequency, phase.
#[derive(Clone, Copy, Debug)]
pub struct EmfSource {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub amplitude: f64,
    pub frequency: f64,
    pub phase: f64,
}

impl EmfSource {
    /// Parse a flat `&[f64]` array into a slice of sources.
    /// Each source occupies 6 consecutive values: [x, y, z, A, f, φ].
    pub fn parse_flat(flat: &[f64]) -> Result<Vec<Self>, PhysicsError> {
        if flat.len() % 6 != 0 {
            return Err(PhysicsError::InvalidConfiguration(format!(
                "sources flat array length {} is not a multiple of 6",
                flat.len()
            )));
        }
        let n = flat.len() / 6;
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            let base = i * 6;
            out.push(Self {
                x: flat[base],
                y: flat[base + 1],
                z: flat[base + 2],
                amplitude: flat[base + 3],
                frequency: flat[base + 4],
                phase: flat[base + 5],
            });
        }
        Ok(out)
    }

    /// Distance from this source to an observation point.
    #[inline]
    pub fn distance_to(&self, x: f64, y: f64, z: f64) -> f64 {
        let dx = x - self.x;
        let dy = y - self.y;
        let dz = z - self.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    /// Wave number k = 2π·f / c.
    #[inline]
    pub fn wave_number(&self, c: f64) -> f64 {
        2.0 * std::f64::consts::PI * self.frequency / c
    }

    /// Instantaneous field contribution at observation point (x,y,z) at time t.
    /// E_i(t) = (A_i / r_i) · sin(2π·f_i·t − k_i·r_i + φ_i)
    #[inline]
    pub fn field_at(&self, x: f64, y: f64, z: f64, t: f64, c: f64) -> f64 {
        let r = self.distance_to(x, y, z).max(MIN_DIST);
        let k = self.wave_number(c);
        let omega = 2.0 * std::f64::consts::PI * self.frequency;
        (self.amplitude / r) * (omega * t - k * r + self.phase).sin()
    }
}

impl PhysicsSimulationLibrary {
    /// EMF interference — superposition of N sources at a 3D observation point.
    ///
    /// Each source contributes `A_i / r_i · sin(2π·f_i·t − k_i·r_i + φ_i)`.
    /// Same-frequency sources produce standing interference patterns;
    /// different frequencies produce beat frequencies.
    ///
    /// **Classical simulation** — no QPU required.
    pub fn run_emf_interference(
        &self,
        sources: &[EmfSource],
        x: f64,
        y: f64,
        z: f64,
        t: f64,
        c: f64,
    ) -> Result<EmfInterferenceResult, PhysicsError> {
        if sources.is_empty() {
            return Err(PhysicsError::InvalidConfiguration(
                "emf_interference needs at least one source".to_string(),
            ));
        }
        let c = if c > 0.0 { c } else { C_LIGHT };

        let instant: f64 = sources.iter().map(|s| s.field_at(x, y, z, t, c)).sum();

        // Find the lowest frequency to determine the sampling period.
        let f_min = sources
            .iter()
            .map(|s| s.frequency.abs())
            .filter(|f| *f > 0.0)
            .min_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(1.0);
        let period = 1.0 / f_min;
        let n_samples = 256;
        let mut peak = 0.0f64;
        for i in 0..n_samples {
            let ti = t + (i as f64 / n_samples as f64) * period;
            let val: f64 = sources.iter().map(|s| s.field_at(x, y, z, ti, c)).sum();
            peak = peak.max(val.abs());
        }

        // Effective phase: for single-frequency, compute analytically.
        // For multi-frequency, use the phase of the dominant (highest amplitude) source.
        let (phase, freq_eff) = if sources.len() == 1
            || sources.iter().all(|s| {
                (s.frequency - sources[0].frequency).abs()
                    < 1e-10 * sources[0].frequency.abs().max(1e-10)
            }) {
            // Single frequency: combine analytically.
            // Σ A_i/r_i · sin(ω·t − k_i·r_i + φ_i) = R·sin(ω·t + Φ)
            // where R = |Σ (A_i/r_i)·e^{i(φ_i − k_i·r_i)}| and Φ = arg(Σ ...)
            let mut re = 0.0f64;
            let mut im = 0.0f64;
            for s in sources {
                let r = s.distance_to(x, y, z).max(MIN_DIST);
                let k = s.wave_number(c);
                let phi = s.phase - k * r;
                let amp = s.amplitude / r;
                re += amp * phi.cos();
                im += amp * phi.sin();
            }
            let combined_phase = im.atan2(re);
            (combined_phase, sources[0].frequency)
        } else {
            // Multi-frequency: dominant source phase, beat frequency.
            let dominant = sources
                .iter()
                .max_by(|a, b| {
                    let ra = a.distance_to(x, y, z).max(MIN_DIST);
                    let rb = b.distance_to(x, y, z).max(MIN_DIST);
                    (a.amplitude / ra)
                        .partial_cmp(&(b.amplitude / rb))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap();
            let r_dom = dominant.distance_to(x, y, z).max(MIN_DIST);
            let k_dom = dominant.wave_number(c);
            let phase_dom = dominant.phase - k_dom * r_dom;

            // Beat frequency = |f1 - f2| for two sources, or min difference for N.
            let beat_freq = if sources.len() == 2 {
                (sources[0].frequency - sources[1].frequency).abs()
            } else {
                let mut freqs: Vec<f64> = sources.iter().map(|s| s.frequency).collect();
                freqs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let mut min_diff = f64::MAX;
                for i in 1..freqs.len() {
                    let d = (freqs[i] - freqs[i - 1]).abs();
                    if d > 0.0 && d < min_diff {
                        min_diff = d;
                    }
                }
                if min_diff < f64::MAX {
                    min_diff
                } else {
                    0.0
                }
            };
            (phase_dom, beat_freq)
        };

        Ok(EmfInterferenceResult {
            instantaneous_value: instant,
            amplitude: peak,
            phase,
            frequency_effective: freq_eff,
            num_sources: sources.len(),
        })
    }

    /// EMF attenuation — inverse-square law + atmospheric absorption.
    ///
    /// P_received = P_source / (4π·r²) · exp(−α·r)
    ///
    /// **Classical simulation** — no QPU required.
    pub fn run_emf_attenuation(
        &self,
        source_power: f64,
        frequency: f64,
        distance: f64,
        absorption_coeff: f64,
    ) -> Result<EmfAttenuationResult, PhysicsError> {
        if source_power <= 0.0 {
            return Err(PhysicsError::InvalidConfiguration(
                "source_power must be positive".to_string(),
            ));
        }
        if distance < 0.0 {
            return Err(PhysicsError::InvalidConfiguration(
                "distance must be non-negative".to_string(),
            ));
        }
        let r = distance.max(MIN_DIST);
        let alpha = absorption_coeff.max(0.0);

        // Free-space (inverse-square): P_fs = P / (4π·r²)
        let p_free_space = source_power / (4.0 * std::f64::consts::PI * r * r);
        // Atmospheric absorption: P_rx = P_fs · exp(−α·r)
        let p_received = p_free_space * (-alpha * r).exp();

        // dB calculations
        let free_space_loss_db = if p_free_space > 0.0 {
            10.0 * (source_power / p_free_space).log10()
        } else {
            f64::INFINITY
        };
        let absorption_loss_db = if p_received > 0.0 && p_free_space > 0.0 {
            10.0 * (p_free_space / p_received).log10()
        } else {
            f64::INFINITY
        };
        let total_attenuation_db = if p_received > 0.0 {
            10.0 * (source_power / p_received).log10()
        } else {
            f64::INFINITY
        };

        Ok(EmfAttenuationResult {
            received_power: p_received,
            attenuation_db: total_attenuation_db,
            free_space_loss_db,
            absorption_loss_db,
            distance,
            frequency,
        })
    }

    /// Relativistic Doppler shift.
    ///
    /// f_observed = f_source · √((1+β)/(1−β))  (approaching, v > 0)
    /// f_observed = f_source · √((1−β)/(1+β))  (receding, v < 0)
    ///
    /// **Classical simulation** — no QPU required.
    pub fn run_doppler_shift(
        &self,
        source_frequency: f64,
        relative_velocity: f64,
        c: f64,
    ) -> Result<DopplerShiftResult, PhysicsError> {
        if source_frequency <= 0.0 {
            return Err(PhysicsError::InvalidConfiguration(
                "source_frequency must be positive".to_string(),
            ));
        }
        let c = if c > 0.0 { c } else { C_LIGHT };
        let beta = relative_velocity / c;
        if beta.abs() >= 1.0 {
            return Err(PhysicsError::InvalidConfiguration(format!(
                "relative velocity |v|={abs} >= c={c}; beta={beta}",
                abs = relative_velocity.abs(),
            )));
        }

        // Unified formula: sign of beta determines approaching/receding.
        // f_obs = f_src · √((1+β)/(1−β))
        let ratio = ((1.0 + beta) / (1.0 - beta)).sqrt();
        let observed = source_frequency * ratio;

        Ok(DopplerShiftResult {
            observed_frequency: observed,
            shift_ratio: ratio,
            relative_velocity,
            beta,
        })
    }

    /// EMF 4D field grid (x×y×z×t) with 10D manifold tags.
    ///
    /// Computes the interference field over a 3D spatial grid at multiple time
    /// slices. Each cell is tagged with a `ManifoldCoordinate10D` derived from
    /// the physics at that point.
    ///
    /// **Classical simulation** — no QPU required.
    pub fn run_emf_field_grid_3d(
        &self,
        sources: &[EmfSource],
        bounds: [f64; 6],
        nx: usize,
        ny: usize,
        nz: usize,
        nt: usize,
        t_start: f64,
        t_end: f64,
        c: f64,
    ) -> Result<EmfFieldGrid3DResult, PhysicsError> {
        if sources.is_empty() {
            return Err(PhysicsError::InvalidConfiguration(
                "emf_field_grid_3d needs at least one source".to_string(),
            ));
        }
        if nx == 0 || ny == 0 || nz == 0 || nt == 0 {
            return Err(PhysicsError::InvalidConfiguration(
                "grid dimensions must be non-zero".to_string(),
            ));
        }
        let c = if c > 0.0 { c } else { C_LIGHT };

        let [x_min, x_max, y_min, y_max, z_min, z_max] = bounds;
        if !(x_max > x_min && y_max > y_min && z_max > z_min) {
            return Err(PhysicsError::InvalidConfiguration(
                "bounds must satisfy x_max > x_min, y_max > y_min, z_max > z_min".to_string(),
            ));
        }

        let total = nx * ny * nz * nt;
        let mut amplitudes = vec![0.0f64; total];
        let mut phases = vec![0.0f64; total];
        let mut frequencies = vec![0.0f64; total];
        let mut manifold_coords = vec![ManifoldCoordinate10D::default(); total];
        let mut times = Vec::with_capacity(nt);

        let dx = (x_max - x_min) / nx.max(1) as f64;
        let dy = (y_max - y_min) / ny.max(1) as f64;
        let dz = (z_max - z_min) / nz.max(1) as f64;
        let dt = if nt > 1 {
            (t_end - t_start) / (nt - 1) as f64
        } else {
            0.0
        };

        // Reference values for manifold normalization.
        let max_amp = sources
            .iter()
            .map(|s| s.amplitude)
            .fold(0.0f64, f64::max)
            .max(1e-12);
        let max_freq = sources
            .iter()
            .map(|s| s.frequency.abs())
            .fold(0.0f64, f64::max)
            .max(1e-12);
        let t_span = (t_end - t_start).abs().max(1e-12);

        for it in 0..nt {
            let t = t_start + it as f64 * dt;
            times.push(t);
            for iz in 0..nz {
                let z = z_min + (iz as f64 + 0.5) * dz;
                for iy in 0..ny {
                    let y = y_min + (iy as f64 + 0.5) * dy;
                    for ix in 0..nx {
                        let x = x_min + (ix as f64 + 0.5) * dx;
                        let idx = ((it * nz + iz) * ny + iy) * nx + ix;

                        let result = self.run_emf_interference(sources, x, y, z, t, c)?;

                        amplitudes[idx] = result.amplitude;
                        phases[idx] = result.phase;
                        frequencies[idx] = result.frequency_effective;

                        // Manifold coordinate mapping.
                        let normalized_amp = (result.amplitude / max_amp) as f32;
                        let normalized_freq = (result.frequency_effective / max_freq) as f32;
                        let normalized_phase = (result.phase / std::f64::consts::TAU) as f32;
                        let normalized_time = ((t - t_start) / t_span) as f32;

                        // Distance from grid center for curvature estimate.
                        let cx = (x_min + x_max) * 0.5;
                        let cy = (y_min + y_max) * 0.5;
                        let cz = (z_min + z_max) * 0.5;
                        let dist_center =
                            ((x - cx).powi(2) + (y - cy).powi(2) + (z - cz).powi(2)).sqrt();
                        let max_dist = ((x_max - x_min).powi(2)
                            + (y_max - y_min).powi(2)
                            + (z_max - z_min).powi(2))
                        .sqrt();
                        let normalized_dist = (dist_center / max_dist.max(1e-12)) as f32;

                        manifold_coords[idx] = ManifoldCoordinate10D {
                            scale: normalized_amp,
                            attention_depth: normalized_dist,
                            epistemic_weight: 1.0,
                            topological_spin: normalized_phase,
                            temporal_decay: normalized_time,
                            entropy_bias: (result.instantaneous_value.abs() / max_amp) as f32,
                            spatial_phase: normalized_phase,
                            recurrence_frequency: normalized_freq,
                            density_threshold: (1.0 / (1.0 + result.amplitude)).min(1.0) as f32,
                            manifold_curvature: 0.0,
                        };
                    }
                }
            }
        }

        Ok(EmfFieldGrid3DResult {
            nx,
            ny,
            nz,
            nt,
            bounds,
            times,
            amplitudes,
            phases,
            frequencies,
            manifold_coords,
            num_sources: sources.len(),
        })
    }

    /// Depth-aware sampling of the EMF field for render integration.
    ///
    /// Samples the physics field at specified depths along a ray from the
    /// camera/observer, applying perspective scaling, display attenuation,
    /// and LOD selection. Bridges physics to `vibeAnimation` output (Phase D).
    ///
    /// **Classical simulation** — no QPU required.
    pub fn run_emf_sample_at_depth(
        &self,
        sources: &[EmfSource],
        camera: [f64; 3],
        direction: [f64; 3],
        depths: &[f64],
        t: f64,
        c: f64,
    ) -> Result<EmfSampleAtDepthResult, PhysicsError> {
        if sources.is_empty() {
            return Err(PhysicsError::InvalidConfiguration(
                "emf_sample_at_depth needs at least one source".to_string(),
            ));
        }
        if depths.is_empty() {
            return Err(PhysicsError::InvalidConfiguration(
                "depths must be non-empty".to_string(),
            ));
        }
        let c = if c > 0.0 { c } else { C_LIGHT };

        // Normalize direction.
        let [dx, dy, dz] = direction;
        let dlen = (dx * dx + dy * dy + dz * dz).sqrt();
        if dlen < MIN_DIST {
            return Err(PhysicsError::InvalidConfiguration(
                "direction must be non-zero".to_string(),
            ));
        }
        let (dx, dy, dz) = (dx / dlen, dy / dlen, dz / dlen);
        let [cx, cy, cz] = camera;

        let max_amp = sources
            .iter()
            .map(|s| s.amplitude)
            .fold(0.0f64, f64::max)
            .max(1e-12);
        let max_freq = sources
            .iter()
            .map(|s| s.frequency.abs())
            .fold(0.0f64, f64::max)
            .max(1e-12);

        let mut samples = Vec::with_capacity(depths.len());
        for &depth in depths {
            let x = cx + depth * dx;
            let y = cy + depth * dy;
            let z = cz + depth * dz;

            let result = self.run_emf_interference(sources, x, y, z, t, c)?;

            // Perspective scaling: objects further away appear smaller.
            let perspective_scale = 1.0 / depth.max(1e-3);

            // Display attenuation: exponential falloff with depth.
            let display_attenuation = (-depth * 0.1).exp();

            // LOD selection: higher depth → lower detail.
            let lod_level = if depth < 10.0 {
                0
            } else if depth < 100.0 {
                1
            } else if depth < 1000.0 {
                2
            } else {
                3
            };

            let normalized_amp = (result.amplitude / max_amp) as f32;
            let normalized_freq = (result.frequency_effective / max_freq) as f32;
            let normalized_phase = (result.phase / std::f64::consts::TAU) as f32;
            let normalized_depth =
                (depth / depths.iter().cloned().fold(0.0f64, f64::max).max(1e-12)) as f32;

            let manifold_coord = ManifoldCoordinate10D {
                scale: normalized_amp,
                attention_depth: normalized_depth,
                epistemic_weight: 1.0,
                topological_spin: normalized_phase,
                temporal_decay: (t.rem_euclid(1.0)) as f32,
                entropy_bias: (result.instantaneous_value.abs() / max_amp) as f32,
                spatial_phase: normalized_phase,
                recurrence_frequency: normalized_freq,
                density_threshold: (1.0 / (1.0 + depth)).min(1.0) as f32,
                manifold_curvature: 0.0,
            };

            samples.push(DepthSample {
                depth,
                amplitude: result.amplitude,
                phase: result.phase,
                frequency: result.frequency_effective,
                perspective_scale,
                display_attenuation,
                lod_level,
                manifold_coord,
            });
        }

        Ok(EmfSampleAtDepthResult {
            num_depths: samples.len(),
            samples,
            time: t,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lib() -> PhysicsSimulationLibrary {
        let lib = PhysicsSimulationLibrary::new();
        // We don't call initialize() — the EMF methods are pure math and don't
        // need the simulation engine. But we create the struct for completeness.
        lib
    }

    #[test]
    fn two_source_constructive_interference() {
        // Two sources at same position, same frequency, same phase → constructive.
        let sources = vec![
            EmfSource {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                amplitude: 1.0,
                frequency: 1.0,
                phase: 0.0,
            },
            EmfSource {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                amplitude: 1.0,
                frequency: 1.0,
                phase: 0.0,
            },
        ];
        let r = lib()
            .run_emf_interference(&sources, 1.0, 0.0, 0.0, 0.0, 1.0)
            .unwrap();
        // At t=0, both contribute sin(−k·r) = sin(−2π·1/1) = sin(−2π) = 0.
        // But the peak amplitude should be ~2/r = 2.0 (two sources at same phase).
        assert!(
            r.amplitude > 1.5,
            "constructive interference amplitude should be ~2.0, got {}",
            r.amplitude
        );
    }

    #[test]
    fn two_source_destructive_interference() {
        // Two sources at same position, same frequency, opposite phase → destructive.
        let sources = vec![
            EmfSource {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                amplitude: 1.0,
                frequency: 1.0,
                phase: 0.0,
            },
            EmfSource {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                amplitude: 1.0,
                frequency: 1.0,
                phase: std::f64::consts::PI,
            },
        ];
        let r = lib()
            .run_emf_interference(&sources, 1.0, 0.0, 0.0, 0.0, 1.0)
            .unwrap();
        // Opposite phases cancel → amplitude near 0.
        assert!(
            r.amplitude < 0.1,
            "destructive interference amplitude should be ~0, got {}",
            r.amplitude
        );
    }

    #[test]
    fn inverse_square_quarter_amplitude() {
        // At 2× distance, received power = 1/(4π·(2r)²) = ¼ of 1/(4π·r²).
        let lib = lib();
        let r1 = lib.run_emf_attenuation(100.0, 1e9, 1.0, 0.0).unwrap();
        let r2 = lib.run_emf_attenuation(100.0, 1e9, 2.0, 0.0).unwrap();
        let ratio = r2.received_power / r1.received_power;
        assert!(
            (ratio - 0.25).abs() < 1e-10,
            "inverse-square: power at 2× distance should be ¼, got {}",
            ratio
        );
    }

    #[test]
    fn doppler_approaching_increases_frequency() {
        // Source approaching at 0.1c → f_obs = f_src · √(1.1/0.9)
        let r = lib()
            .run_doppler_shift(1e9, 0.1 * C_LIGHT, C_LIGHT)
            .unwrap();
        let expected = 1e9_f64 * (1.1_f64 / 0.9_f64).sqrt();
        assert!(
            (r.observed_frequency - expected).abs() / expected < 1e-10,
            "Doppler approaching: expected {}, got {}",
            expected,
            r.observed_frequency
        );
        assert!(r.shift_ratio > 1.0, "approaching should increase frequency");
    }

    #[test]
    fn doppler_receding_decreases_frequency() {
        // Source receding at 0.1c → f_obs = f_src · √(0.9/1.1)
        let r = lib()
            .run_doppler_shift(1e9, -0.1 * C_LIGHT, C_LIGHT)
            .unwrap();
        let expected = 1e9_f64 * (0.9_f64 / 1.1_f64).sqrt();
        assert!(
            (r.observed_frequency - expected).abs() / expected < 1e-10,
            "Doppler receding: expected {}, got {}",
            expected,
            r.observed_frequency
        );
        assert!(r.shift_ratio < 1.0, "receding should decrease frequency");
    }

    #[test]
    fn field_grid_finite_and_monotonic() {
        // Source at corner (-5,-5,-5) so cell (0,0,0) is close and cell (3,3,3) is far.
        let sources = vec![EmfSource {
            x: -5.0,
            y: -5.0,
            z: -5.0,
            amplitude: 1.0,
            frequency: 1.0,
            phase: 0.0,
        }];
        let r = lib()
            .run_emf_field_grid_3d(
                &sources,
                [-5.0, 5.0, -5.0, 5.0, -5.0, 5.0],
                4,
                4,
                4,
                2,
                0.0,
                1.0,
                1.0,
            )
            .unwrap();
        assert_eq!(r.amplitudes.len(), 4 * 4 * 4 * 2);
        assert!(
            r.amplitudes.iter().all(|a| a.is_finite()),
            "all amplitudes must be finite"
        );
        assert!(
            r.phases.iter().all(|p| p.is_finite()),
            "all phases must be finite"
        );
        // Cell (0,0,0) is at (-3.75,-3.75,-3.75) — close to source at (-5,-5,-5).
        let close_idx = 0 * (4 * 4 * 4) + 0 * (4 * 4) + 0 * 4 + 0;
        // Cell (3,3,3) is at (3.75,3.75,3.75) — far from source.
        let far_idx = 0 * (4 * 4 * 4) + 3 * (4 * 4) + 3 * 4 + 3;
        assert!(
            r.amplitudes[close_idx] > r.amplitudes[far_idx],
            "amplitude should decrease with distance: close={}, far={}",
            r.amplitudes[close_idx],
            r.amplitudes[far_idx]
        );
    }

    #[test]
    fn sample_at_depth_perspective_scaling() {
        let sources = vec![EmfSource {
            x: 0.0,
            y: 0.0,
            z: 10.0,
            amplitude: 1.0,
            frequency: 1.0,
            phase: 0.0,
        }];
        let r = lib()
            .run_emf_sample_at_depth(
                &sources,
                [0.0, 0.0, 0.0],
                [0.0, 0.0, 1.0],
                &[1.0, 10.0, 100.0],
                0.0,
                1.0,
            )
            .unwrap();
        assert_eq!(r.samples.len(), 3);
        // Perspective scale should decrease with depth.
        assert!(r.samples[0].perspective_scale > r.samples[1].perspective_scale);
        assert!(r.samples[1].perspective_scale > r.samples[2].perspective_scale);
        // Display attenuation should decrease with depth.
        assert!(r.samples[0].display_attenuation > r.samples[2].display_attenuation);
        // LOD should increase with depth.
        assert!(r.samples[2].lod_level >= r.samples[0].lod_level);
        // All manifold coords should have finite values.
        for s in &r.samples {
            let arr = s.manifold_coord.as_f32_array();
            assert!(
                arr.iter().all(|v| v.is_finite()),
                "manifold coords must be finite"
            );
        }
    }

    #[test]
    fn beat_frequency_two_sources() {
        // Two sources at 100 Hz and 105 Hz → beat frequency = 5 Hz.
        let sources = vec![
            EmfSource {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                amplitude: 1.0,
                frequency: 100.0,
                phase: 0.0,
            },
            EmfSource {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                amplitude: 1.0,
                frequency: 105.0,
                phase: 0.0,
            },
        ];
        let r = lib()
            .run_emf_interference(&sources, 1.0, 0.0, 0.0, 0.0, 340.0)
            .unwrap();
        assert!(
            (r.frequency_effective - 5.0).abs() < 1e-6,
            "beat frequency should be 5 Hz, got {}",
            r.frequency_effective
        );
    }

    #[test]
    fn empty_sources_errors() {
        let r = lib().run_emf_interference(&[], 0.0, 0.0, 0.0, 0.0, 1.0);
        assert!(r.is_err());
    }

    #[test]
    fn doppler_superluminal_errors() {
        let r = lib().run_doppler_shift(1.0, 2.0 * C_LIGHT, C_LIGHT);
        assert!(r.is_err());
    }
}
