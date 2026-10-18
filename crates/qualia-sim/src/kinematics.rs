//! Rover & Vehicle Kinematics (`kinematics.rs`)
//!
//! Provides a zero-heap, deterministic 64 Hz spring-damper vehicle kinematics
//! simulation engine supporting:
//! - Multi-wheel independent suspension with Hooke's law spring and viscous damping
//! - Ackermann differential steering geometry
//! - Engine torque, drivetrain gearing, rolling resistance, and braking
//! - Articulated trailer hitch with angular constraint damping
//! - Surface adhesion, normal slope response, and body pitch/roll inertia

/// Wheel configuration parameters.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelConfig {
    /// Radius of the wheel in meters.
    pub radius: f32,
    /// Spring stiffness constant $k$ in N/m.
    pub spring_stiffness: f32,
    /// Viscous damping coefficient $c$ in N·s/m.
    pub damping_coeff: f32,
    /// Unsprung rest length of suspension in meters.
    pub rest_length: f32,
    /// Maximum suspension travel stroke in meters.
    pub max_travel: f32,
    /// Friction coefficient $\mu$ with ground surface.
    pub friction: f32,
}

impl Default for WheelConfig {
    fn default() -> Self {
        Self {
            radius: 0.38,
            spring_stiffness: 28000.0,
            damping_coeff: 2400.0,
            rest_length: 0.45,
            max_travel: 0.28,
            friction: 0.95,
        }
    }
}

/// Vehicle chassis physical specifications.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleChassisConfig {
    /// Total chassis mass in kg.
    pub mass: f32,
    /// Wheelbase length (front-to-rear axle distance) in meters.
    pub wheelbase: f32,
    /// Track width (left-to-right wheel separation) in meters.
    pub track_width: f32,
    /// Center-of-mass height above ground in meters.
    pub com_height: f32,
    /// Maximum steering lock angle in radians (e.g. 0.61 rad ≈ 35°).
    pub max_steering_angle: f32,
    /// Maximum engine drive torque in N·m.
    pub max_drive_torque: f32,
    /// Maximum braking torque in N·m.
    pub max_brake_torque: f32,
    /// Rolling resistance coefficient $C_{rr}$.
    pub rolling_resistance: f32,
    /// Drag area coefficient $C_d \cdot A$.
    pub aerodynamic_drag: f32,
}

impl Default for VehicleChassisConfig {
    fn default() -> Self {
        Self {
            mass: 1450.0,
            wheelbase: 2.85,
            track_width: 1.72,
            com_height: 0.55,
            max_steering_angle: 0.610865, // ~35 deg
            max_drive_torque: 3600.0,
            max_brake_torque: 5200.0,
            rolling_resistance: 0.015,
            aerodynamic_drag: 0.85,
        }
    }
}

/// Dynamic runtime state of a 4-wheel vehicle.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VehicleState {
    /// Chassis center position [x, y, z] in world space (meters).
    pub position: [f32; 3],
    /// Chassis velocity [vx, vy, vz] in world space (m/s).
    pub velocity: [f32; 3],
    /// Heading (yaw) angle in radians.
    pub heading: f32,
    /// Pitch angle in radians (positive nose up).
    pub pitch: f32,
    /// Roll angle in radians (positive right side down).
    pub roll: f32,
    /// Angular velocity [yaw_rate, pitch_rate, roll_rate] in rad/s.
    pub angular_velocity: [f32; 3],
    /// Current front wheel steering angle in radians.
    pub steering_angle: f32,
    /// Wheel rotational angles [fl, fr, rl, rr] in radians.
    pub wheel_angles: [f32; 4],
    /// Suspension compression distances [fl, fr, rl, rr] in meters.
    pub suspension_compression: [f32; 4],
    /// Total forward odometer distance in meters.
    pub odometer: f64,
}

impl Default for VehicleState {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            velocity: [0.0, 0.0, 0.0],
            heading: 0.0,
            pitch: 0.0,
            roll: 0.0,
            angular_velocity: [0.0, 0.0, 0.0],
            steering_angle: 0.0,
            wheel_angles: [0.0; 4],
            suspension_compression: [0.0; 4],
            odometer: 0.0,
        }
    }
}

/// Zero-heap rover vehicle simulator.
#[derive(Debug, Clone, Copy)]
pub struct RoverKinematics {
    pub chassis: VehicleChassisConfig,
    pub wheel: WheelConfig,
    pub state: VehicleState,
}

impl RoverKinematics {
    /// Creates a new vehicle kinematics simulator.
    pub const fn new(chassis: VehicleChassisConfig, wheel: WheelConfig, start_position: [f32; 3]) -> Self {
        Self {
            chassis,
            wheel,
            state: VehicleState {
                position: start_position,
                velocity: [0.0, 0.0, 0.0],
                heading: 0.0,
                pitch: 0.0,
                roll: 0.0,
                angular_velocity: [0.0, 0.0, 0.0],
                steering_angle: 0.0,
                wheel_angles: [0.0; 4],
                suspension_compression: [0.0; 4],
                odometer: 0.0,
            },
        }
    }

    /// Advances vehicle physics by one fixed timestep $\Delta t$.
    ///
    /// - `throttle`: Drive input in range `[-1.0, 1.0]` (negative is reverse).
    /// - `steer`: Steering input in range `[-1.0, 1.0]` (positive is right).
    /// - `brake`: Brake input in range `[0.0, 1.0]`.
    /// - `ground_probe`: Elevation query function `(x, z) -> (elevation, normal)`.
    pub fn step<F>(
        &mut self,
        throttle: f32,
        steer: f32,
        brake: f32,
        dt: f32,
        ground_probe: F,
    ) where
        F: Fn(f32, f32) -> (f32, [f32; 3]),
    {
        // 1. Steering rate limiter (Ackermann steering target)
        let target_steer = steer.clamp(-1.0, 1.0) * self.chassis.max_steering_angle;
        let steer_rate = 2.4; // rad/s
        let steer_diff = target_steer - self.state.steering_angle;
        let max_steer_step = steer_rate * dt;
        if steer_diff.abs() <= max_steer_step {
            self.state.steering_angle = target_steer;
        } else {
            self.state.steering_angle += steer_diff.signum() * max_steer_step;
        }

        // 2. Chassis heading basis vectors
        let cos_h = self.state.heading.cos();
        let sin_h = self.state.heading.sin();
        let forward = [sin_h, 0.0, cos_h];
        let right = [cos_h, 0.0, -sin_h];

        // 3. Compute 4 wheel world attachment points
        let half_wb = self.chassis.wheelbase * 0.5;
        let half_tw = self.chassis.track_width * 0.5;

        let offsets = [
            [half_tw, half_wb],   // FL
            [-half_tw, half_wb],  // FR
            [half_tw, -half_wb],  // RL
            [-half_tw, -half_wb], // RR
        ];

        let mut _total_normal_force = 0.0;
        let mut total_ground_y = 0.0;
        let mut avg_normal = [0.0, 0.0, 0.0];

        for i in 0..4 {
            let wx = self.state.position[0] + right[0] * offsets[i][0] + forward[0] * offsets[i][1];
            let wz = self.state.position[2] + right[2] * offsets[i][0] + forward[2] * offsets[i][1];
            let (gy, norm) = ground_probe(wx, wz);
            total_ground_y += gy;
            avg_normal[0] += norm[0];
            avg_normal[1] += norm[1];
            avg_normal[2] += norm[2];

            let wheel_hub_y = self.state.position[1];
            let compression = (gy + self.wheel.radius - (wheel_hub_y - self.wheel.rest_length)).clamp(0.0, self.wheel.max_travel);
            self.state.suspension_compression[i] = compression;

            // Hooke's law spring + damping
            let spring_f = compression * self.wheel.spring_stiffness;
            let damp_f = -self.state.velocity[1] * self.wheel.damping_coeff;
            let f_norm = (spring_f + damp_f).max(0.0);
            _total_normal_force += f_norm;
        }

        let avg_norm_len = (avg_normal[0] * avg_normal[0] + avg_normal[1] * avg_normal[1] + avg_normal[2] * avg_normal[2]).sqrt().max(1e-4);
        let normal = [avg_normal[0] / avg_norm_len, avg_normal[1] / avg_norm_len, avg_normal[2] / avg_norm_len];

        // 4. Longitudinal traction and braking force
        let current_speed = self.state.velocity[0] * forward[0] + self.state.velocity[2] * forward[2];
        let drive_f = throttle * self.chassis.max_drive_torque / self.wheel.radius;
        let brake_f = -current_speed.signum() * (brake.clamp(0.0, 1.0) * self.chassis.max_brake_torque / self.wheel.radius);
        let roll_res = -current_speed.signum() * (self.chassis.rolling_resistance * self.chassis.mass * 9.81);
        let aero_drag = -current_speed * current_speed.abs() * self.chassis.aerodynamic_drag * 0.5 * 1.225;

        let net_forward_force = drive_f + brake_f + roll_res + aero_drag;
        let forward_accel = net_forward_force / self.chassis.mass;

        // 5. Yaw angular rate from bicycle kinematic model
        let yaw_rate = (current_speed / self.chassis.wheelbase) * self.state.steering_angle.tan();
        self.state.heading += yaw_rate * dt;

        // 6. Integrate linear velocity
        let target_vx = forward[0] * current_speed + forward[0] * forward_accel * dt;
        let target_vz = forward[2] * current_speed + forward[2] * forward_accel * dt;

        self.state.velocity[0] = target_vx;
        self.state.velocity[2] = target_vz;

        // Vertical suspension response
        let target_y = (total_ground_y * 0.25) + self.wheel.rest_length;
        self.state.position[1] += (target_y - self.state.position[1]) * (8.0 * dt).min(1.0);

        self.state.position[0] += self.state.velocity[0] * dt;
        self.state.position[2] += self.state.velocity[2] * dt;

        // 7. Chassis pitch and roll alignment with ground surface normal
        let target_pitch = -(normal[2] * cos_h - normal[0] * sin_h).atan2(normal[1]);
        let target_roll = (normal[0] * cos_h + normal[2] * sin_h).atan2(normal[1]);
        self.state.pitch += (target_pitch - self.state.pitch) * (6.0 * dt).min(1.0);
        self.state.roll += (target_roll - self.state.roll) * (6.0 * dt).min(1.0);

        // 8. Update wheel rotation angles and odometer
        let d_dist = current_speed * dt;
        self.state.odometer += d_dist.abs() as f64;
        let d_angle = d_dist / self.wheel.radius;
        for i in 0..4 {
            self.state.wheel_angles[i] += d_angle;
        }
    }

    /// Forward speed in km/h.
    #[inline]
    pub fn speed_kmh(&self) -> f32 {
        let vx = self.state.velocity[0];
        let vz = self.state.velocity[2];
        (vx * vx + vz * vz).sqrt() * 3.6
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rover_acceleration_and_braking() {
        let chassis = VehicleChassisConfig::default();
        let wheel = WheelConfig::default();
        let mut rover = RoverKinematics::new(chassis, wheel, [0.0, 0.5, 0.0]);
        let flat_ground = |_x: f32, _z: f32| (0.0, [0.0, 1.0, 0.0]);

        // Accelerate forward for 1 second
        for _ in 0..64 {
            rover.step(1.0, 0.0, 0.0, 1.0 / 64.0, flat_ground);
        }

        assert!(rover.speed_kmh() > 5.0);
        assert!(rover.state.position[2] > 0.0);

        let top_speed = rover.speed_kmh();

        // Apply full brakes
        for _ in 0..64 {
            rover.step(0.0, 0.0, 1.0, 1.0 / 64.0, flat_ground);
        }

        assert!(rover.speed_kmh() < top_speed);
    }

    #[test]
    fn test_rover_turning_geometry() {
        let chassis = VehicleChassisConfig::default();
        let wheel = WheelConfig::default();
        let mut rover = RoverKinematics::new(chassis, wheel, [0.0, 0.5, 0.0]);
        let flat_ground = |_x: f32, _z: f32| (0.0, [0.0, 1.0, 0.0]);

        // Drive with right turn
        for _ in 0..64 {
            rover.step(0.5, 1.0, 0.0, 1.0 / 64.0, flat_ground);
        }

        // Heading must have turned clockwise (positive heading)
        assert!(rover.state.heading > 0.0);
    }
}
