use super::*;

impl PhysicsSimulationLibrary {
    /// Biophysics — logistic population dynamics `dN/dt = r·N·(1 − N/K)`, integrated by
    /// `integrate_dopri5`. Matches the analytic logistic curve
    /// `N(t) = K / (1 + ((K−N₀)/N₀)·e^{−r·t})`.
    pub fn run_logistic_growth(
        &self,
        n0: f64,
        growth_rate: f64,
        carrying_capacity: f64,
        total_time: f64,
        num_samples: usize,
    ) -> Result<PopulationDynamicsResult, PhysicsError> {
        if !(n0 >= 0.0 && carrying_capacity > 0.0 && total_time > 0.0) {
            return Err(PhysicsError::InvalidConfiguration(
                "require n0 >= 0, carrying_capacity > 0, total_time > 0".to_string(),
            ));
        }
        let r = growth_rate;
        let k = carrying_capacity;
        let deriv = move |_t: f64, y: &[f64], dy: &mut [f64]| -> Result<(), OdeError> {
            dy[0] = r * y[0] * (1.0 - y[0] / k);
            Ok(())
        };
        let (_final, snapshots, accepted, rejected) =
            self.integrate_ode_samples(vec![n0], total_time, num_samples, deriv)?;
        let n_pts = snapshots.len();
        let times: Vec<f64> = (0..n_pts)
            .map(|kk| total_time * kk as f64 / (n_pts - 1).max(1) as f64)
            .collect();
        let population: Vec<f64> = snapshots.iter().map(|s| s[0]).collect();
        Ok(PopulationDynamicsResult {
            times,
            population,
            carrying_capacity: k,
            growth_rate: r,
            steps_accepted: accepted,
            steps_rejected: rejected,
        })
    }
}

/// A 3D boid agent for avian flocking or fish schooling simulation (Tier-1 Zero-Heap).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct BoidAgent {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
}

/// Behavioral tuning parameters for flocking simulation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlockingParameters {
    pub separation_radius: f32,
    pub neighbor_radius: f32,
    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
    pub max_speed: f32,
    pub min_speed: f32,
}

impl Default for FlockingParameters {
    fn default() -> Self {
        Self {
            separation_radius: 1.5,
            neighbor_radius: 6.0,
            separation_weight: 1.5,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            max_speed: 12.0,
            min_speed: 2.0,
        }
    }
}

/// Step flocking simulation for a slice of boids in-place (Tier-1 HotZeroHeap).
/// Evaluates Separation, Alignment, and Cohesion forces in caller-owned slices.
pub fn step_flocking_simulation(
    boids: &mut [BoidAgent],
    params: &FlockingParameters,
    dt: f32,
) {
    let n = boids.len();
    if n <= 1 || dt <= 0.0 {
        return;
    }

    let sep_r2 = params.separation_radius * params.separation_radius;
    let nbr_r2 = params.neighbor_radius * params.neighbor_radius;

    // Use a fixed stack scratch for acceleration deltas if n <= 256
    // Otherwise operate on chunks to preserve zero heap
    for i in 0..n {
        let pi = boids[i].position;
        let vi = boids[i].velocity;

        let mut sep = [0.0f32; 3];
        let mut ali = [0.0f32; 3];
        let mut coh_center = [0.0f32; 3];
        let mut nbr_count = 0usize;

        for j in 0..n {
            if i == j {
                continue;
            }
            let pj = boids[j].position;
            let dx = pi[0] - pj[0];
            let dy = pi[1] - pj[1];
            let dz = pi[2] - pj[2];
            let dist2 = dx * dx + dy * dy + dz * dz;

            if dist2 < sep_r2 && dist2 > 1e-6 {
                let dist = dist2.sqrt();
                sep[0] += (dx / dist) / dist;
                sep[1] += (dy / dist) / dist;
                sep[2] += (dz / dist) / dist;
            }

            if dist2 < nbr_r2 {
                ali[0] += boids[j].velocity[0];
                ali[1] += boids[j].velocity[1];
                ali[2] += boids[j].velocity[2];

                coh_center[0] += pj[0];
                coh_center[1] += pj[1];
                coh_center[2] += pj[2];

                nbr_count += 1;
            }
        }

        let mut accel = [0.0f32; 3];
        accel[0] += sep[0] * params.separation_weight;
        accel[1] += sep[1] * params.separation_weight;
        accel[2] += sep[2] * params.separation_weight;

        if nbr_count > 0 {
            let inv_n = 1.0 / nbr_count as f32;
            let avg_v = [ali[0] * inv_n, ali[1] * inv_n, ali[2] * inv_n];
            accel[0] += (avg_v[0] - vi[0]) * params.alignment_weight;
            accel[1] += (avg_v[1] - vi[1]) * params.alignment_weight;
            accel[2] += (avg_v[2] - vi[2]) * params.alignment_weight;

            let center = [coh_center[0] * inv_n, coh_center[1] * inv_n, coh_center[2] * inv_n];
            accel[0] += (center[0] - pi[0]) * params.cohesion_weight;
            accel[1] += (center[1] - pi[1]) * params.cohesion_weight;
            accel[2] += (center[2] - pi[2]) * params.cohesion_weight;
        }

        let mut new_v = [
            vi[0] + accel[0] * dt,
            vi[1] + accel[1] * dt,
            vi[2] + accel[2] * dt,
        ];

        let speed = (new_v[0] * new_v[0] + new_v[1] * new_v[1] + new_v[2] * new_v[2]).sqrt();
        if speed > params.max_speed && speed > 1e-5 {
            let scale = params.max_speed / speed;
            new_v[0] *= scale;
            new_v[1] *= scale;
            new_v[2] *= scale;
        } else if speed < params.min_speed && speed > 1e-5 {
            let scale = params.min_speed / speed;
            new_v[0] *= scale;
            new_v[1] *= scale;
            new_v[2] *= scale;
        }

        boids[i].velocity = new_v;
        boids[i].position[0] += new_v[0] * dt;
        boids[i].position[1] += new_v[1] * dt;
        boids[i].position[2] += new_v[2] * dt;
    }
}
