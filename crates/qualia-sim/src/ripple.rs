//! 2D Interactive Ripple Buffer & Dynamic Wave Equation Solver (`ripple.rs`)
//!
//! Provides zero-heap real-time 2D water perturbation simulation:
//! - Finite difference wave equation: $h^{n+1} = 2h^n - h^{n-1} + C^2 \nabla^2 h^n$
//! - Laplacian dampening for viscosity and surface tension loss
//! - Impulse injection for character footfalls, vehicle wakes, and raindrops
//! - Bilinear height and surface normal sampling for rendering and physics coupling
//!
//! Adheres strictly to `AGENTS.md` §0 (Zero heap in hot paths, no Vec/String/Box).

pub const RIPPLE_GRID_DIM: usize = 64;
pub const RIPPLE_TOTAL_CELLS: usize = RIPPLE_GRID_DIM * RIPPLE_GRID_DIM;

/// A zero-heap 2D discrete wave heightfield solver.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct InteractiveRippleBuffer {
    /// Previous time-step height buffer $h^{n-1}$
    pub h_prev: [f32; RIPPLE_TOTAL_CELLS],
    /// Current time-step height buffer $h^n$
    pub h_curr: [f32; RIPPLE_TOTAL_CELLS],
    /// Next time-step height buffer $h^{n+1}$
    pub h_next: [f32; RIPPLE_TOTAL_CELLS],
    /// World-space center X coordinate
    pub center_x: f32,
    /// World-space center Z coordinate
    pub center_z: f32,
    /// Physical width of the grid in meters
    pub world_size_m: f32,
    /// Wave propagation speed $c$ in m/s (typically 1.5 - 3.5 m/s for shallow water ripples)
    pub wave_speed: f32,
    /// Damping factor per step $\gamma \in (0.0, 1.0)$ (e.g. 0.985)
    pub damping: f32,
}

impl Default for InteractiveRippleBuffer {
    fn default() -> Self {
        Self::new(0.0, 0.0, 16.0, 2.5, 0.985)
    }
}

impl InteractiveRippleBuffer {
    /// Creates a new interactive ripple grid centered at `(center_x, center_z)` in world space.
    pub const fn new(
        center_x: f32,
        center_z: f32,
        world_size_m: f32,
        wave_speed: f32,
        damping: f32,
    ) -> Self {
        Self {
            h_prev: [0.0; RIPPLE_TOTAL_CELLS],
            h_curr: [0.0; RIPPLE_TOTAL_CELLS],
            h_next: [0.0; RIPPLE_TOTAL_CELLS],
            center_x,
            center_z,
            world_size_m,
            wave_speed,
            damping,
        }
    }

    /// Resets all height buffers to zero (calm water).
    pub fn clear(&mut self) {
        self.h_prev = [0.0; RIPPLE_TOTAL_CELLS];
        self.h_curr = [0.0; RIPPLE_TOTAL_CELLS];
        self.h_next = [0.0; RIPPLE_TOTAL_CELLS];
    }

    /// Spatial resolution $\Delta x$ in meters.
    #[inline(always)]
    pub fn cell_size(&self) -> f32 {
        self.world_size_m / (RIPPLE_GRID_DIM as f32)
    }

    /// Converts world coordinates $(wx, wz)$ to continuous grid indices $(gx, gz) \in [0, \text{DIM}-1]$.
    #[inline]
    pub fn world_to_grid(&self, wx: f32, wz: f32) -> (f32, f32) {
        let half = self.world_size_m * 0.5;
        let rx = (wx - (self.center_x - half)) / self.world_size_m;
        let rz = (wz - (self.center_z - half)) / self.world_size_m;
        let dim = (RIPPLE_GRID_DIM - 1) as f32;
        (rx * dim, rz * dim)
    }

    /// Injects a smooth radial Gaussian impulse into the wave heightfield.
    pub fn inject_impulse(&mut self, wx: f32, wz: f32, amplitude: f32, radius_m: f32) {
        let (gx, gz) = self.world_to_grid(wx, wz);
        let cell_m = self.cell_size();
        let grid_radius = (radius_m / cell_m).max(1.0);
        let r_sq = grid_radius * grid_radius;

        let min_x = ((gx - grid_radius).floor() as isize).max(1).min((RIPPLE_GRID_DIM - 2) as isize) as usize;
        let max_x = ((gx + grid_radius).ceil() as isize).max(1).min((RIPPLE_GRID_DIM - 2) as isize) as usize;
        let min_z = ((gz - grid_radius).floor() as isize).max(1).min((RIPPLE_GRID_DIM - 2) as isize) as usize;
        let max_z = ((gz + grid_radius).ceil() as isize).max(1).min((RIPPLE_GRID_DIM - 2) as isize) as usize;

        for z in min_z..=max_z {
            let dz = (z as f32) - gz;
            let dz_sq = dz * dz;
            let row_offset = z * RIPPLE_GRID_DIM;

            for x in min_x..=max_x {
                let dx = (x as f32) - gx;
                let dist_sq = dx * dx + dz_sq;

                if dist_sq <= r_sq {
                    let falloff = (-dist_sq / (2.0 * (grid_radius * 0.5).powi(2))).exp();
                    let idx = row_offset + x;
                    self.h_curr[idx] += amplitude * falloff;
                }
            }
        }
    }

    /// Steps the 2D wave equation forward by $\Delta t$ seconds.
    ///
    /// Uses Courant-Friedrichs-Lewy (CFL) stable finite difference:
    /// $h^{n+1}_{i,j} = \left[ 2h^n_{i,j} - h^{n-1}_{i,j} + \alpha (h^n_{i+1,j} + h^n_{i-1,j} + h^n_{i,j+1} + h^n_{i,j-1} - 4h^n_{i,j}) \right] \cdot \gamma$
    /// where $\alpha = (c \cdot \Delta t / \Delta x)^2 \le 0.5$.
    pub fn step(&mut self, dt: f32) {
        let dx = self.cell_size();
        let courant = (self.wave_speed * dt) / dx;
        // CFL condition: alpha <= 0.5 for numerical stability in 2D
        let alpha = (courant * courant).min(0.48);
        let damping = self.damping;

        for z in 1..(RIPPLE_GRID_DIM - 1) {
            let row = z * RIPPLE_GRID_DIM;
            let row_up = (z - 1) * RIPPLE_GRID_DIM;
            let row_down = (z + 1) * RIPPLE_GRID_DIM;

            for x in 1..(RIPPLE_GRID_DIM - 1) {
                let idx = row + x;

                let laplacian = self.h_curr[row + (x - 1)]
                    + self.h_curr[row + (x + 1)]
                    + self.h_curr[row_up + x]
                    + self.h_curr[row_down + x]
                    - 4.0 * self.h_curr[idx];

                let h_new = (2.0 * self.h_curr[idx] - self.h_prev[idx] + alpha * laplacian) * damping;
                self.h_next[idx] = h_new;
            }
        }

        // Absorbing Dirichlet zero boundary on outer edge
        for i in 0..RIPPLE_GRID_DIM {
            self.h_next[i] = 0.0; // Top
            self.h_next[(RIPPLE_GRID_DIM - 1) * RIPPLE_GRID_DIM + i] = 0.0; // Bottom
            self.h_next[i * RIPPLE_GRID_DIM] = 0.0; // Left
            self.h_next[i * RIPPLE_GRID_DIM + (RIPPLE_GRID_DIM - 1)] = 0.0; // Right
        }

        // Ping-pong buffer swap
        self.h_prev = self.h_curr;
        self.h_curr = self.h_next;
    }

    /// Bilinear sampling of the water surface height at arbitrary world coordinates $(wx, wz)$.
    pub fn sample_height(&self, wx: f32, wz: f32) -> f32 {
        let (gx, gz) = self.world_to_grid(wx, wz);
        let dim = (RIPPLE_GRID_DIM - 1) as f32;

        if gx < 0.0 || gx >= dim || gz < 0.0 || gz >= dim {
            return 0.0;
        }

        let x0 = gx.floor() as usize;
        let z0 = gz.floor() as usize;
        let x1 = (x0 + 1).min(RIPPLE_GRID_DIM - 1);
        let z1 = (z0 + 1).min(RIPPLE_GRID_DIM - 1);

        let fx = gx - (x0 as f32);
        let fz = gz - (z0 as f32);

        let h00 = self.h_curr[z0 * RIPPLE_GRID_DIM + x0];
        let h10 = self.h_curr[z0 * RIPPLE_GRID_DIM + x1];
        let h01 = self.h_curr[z1 * RIPPLE_GRID_DIM + x0];
        let h11 = self.h_curr[z1 * RIPPLE_GRID_DIM + x1];

        let h0 = h00 * (1.0 - fx) + h10 * fx;
        let h1 = h01 * (1.0 - fx) + h11 * fx;

        h0 * (1.0 - fz) + h1 * fz
    }

    /// Evaluates the normalized surface normal vector $[nx, ny, nz]$ at arbitrary world coordinates.
    pub fn sample_normal(&self, wx: f32, wz: f32) -> [f32; 3] {
        let dx = self.cell_size();
        let eps = dx * 0.5;

        let h_left = self.sample_height(wx - eps, wz);
        let h_right = self.sample_height(wx + eps, wz);
        let h_back = self.sample_height(wx, wz - eps);
        let h_front = self.sample_height(wx, wz + eps);

        let dh_dx = (h_right - h_left) / (2.0 * eps);
        let dh_dz = (h_front - h_back) / (2.0 * eps);

        let nx = -dh_dx;
        let ny = 1.0;
        let nz = -dh_dz;
        let len = (nx * nx + ny * ny + nz * nz).sqrt();

        if len > 1e-6 {
            [nx / len, ny / len, nz / len]
        } else {
            [0.0, 1.0, 0.0]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ripple_buffer_initialization() {
        let ripple = InteractiveRippleBuffer::default();
        assert_eq!(ripple.sample_height(0.0, 0.0), 0.0);
        let n = ripple.sample_normal(0.0, 0.0);
        assert!((n[0] - 0.0).abs() < 1e-4);
        assert!((n[1] - 1.0).abs() < 1e-4);
        assert!((n[2] - 0.0).abs() < 1e-4);
    }

    #[test]
    fn test_impulse_and_wave_propagation() {
        let mut ripple = InteractiveRippleBuffer::new(0.0, 0.0, 16.0, 2.0, 0.99);
        ripple.inject_impulse(0.0, 0.0, 1.0, 1.0);

        let initial_h = ripple.sample_height(0.0, 0.0);
        assert!(initial_h > 0.5, "Impulse should be registered at center");

        // Step forward in time
        let dt = 1.0 / 64.0;
        for _ in 0..10 {
            ripple.step(dt);
        }

        // Energy should disperse radially outward
        let ring_h = ripple.sample_height(0.5, 0.0);
        assert!(ring_h != 0.0, "Wave should propagate outward from center");
    }

    #[test]
    fn test_ripple_damping_conservation() {
        let mut ripple = InteractiveRippleBuffer::new(0.0, 0.0, 16.0, 2.0, 0.90);
        ripple.inject_impulse(0.0, 0.0, 0.5, 0.8);

        // Run for 100 steps with heavy damping
        let dt = 1.0 / 64.0;
        for _ in 0..100 {
            ripple.step(dt);
        }

        let damped_h = ripple.sample_height(0.0, 0.0);
        assert!(damped_h.abs() < 0.05, "Wave must decay with damping");
    }
}
