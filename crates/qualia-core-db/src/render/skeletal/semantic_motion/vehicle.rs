//! Resident six-DOF marine rigid-body motion over an admitted M64 hull proxy.

use crate::{
    container_10d::InstanceOverrideHeader,
    etalon_render::{
        spatial_math::pga::sandwich_point,
        water_surface::{GerstnerWave, SimdGerstnerSpectrum, GRAVITY_M_S2, MAX_GERSTNER_HARMONICS, WATER_DENSITY_KG_M3},
    },
    q42::m64_matter::{M64DecodedRecord, M64Reader},
    q_hash,
    simulation::physics3d::{ColliderShape3D, RigidBody3D, couple_rigid_body_with_hydro},
};

use super::{MotionGraph, Motor, SemanticAction, SemanticMotionError as Error, graph::index, rest_poses_agree};

const MAX_HULL_POINTS: usize = 4096;
const MAX_STEP_S: f64 = 0.25;
const INTERNAL_STEP_S: f64 = 1.0 / 128.0;
const HULL_COORDINATES: &str = "https://qualiadb.org/ns/kinematics#EtalonLocalMetresYUp";

#[derive(Clone, Copy, Debug, Default)]
pub struct MarineVehicleControl {
    /// Signed fraction of the authored maximum propeller thrust.
    pub throttle: f32,
    /// Signed fraction of the authored rudder deflection limit.
    pub rudder: f32,
}

#[derive(Clone, Copy)]
pub struct MarineVehicleState {
    chassis: RigidBody3D,
    time_s: f64,
    model_id: u64,
}

#[derive(Clone, Copy)]
pub struct MarineVehicleStep {
    pub semantic_id: u64,
    /// Local PGA delta for a resident IOVR leaf. Compose after its authored pose.
    pub local_delta: Motor,
    pub world_pose: Motor,
    pub time_s: f64,
    pub linear_velocity_m_s: [f32; 3],
    /// Body-local angular velocity supplied to Etalon's post-multiplied rotor update.
    pub angular_velocity_rad_s: [f32; 3],
    pub submerged_volume_m3: f32,
    pub buoyancy_n: f32,
    pub hydrodynamic_drag_n: [f32; 3],
    pub aerodynamic_drag_n: [f32; 3],
    /// Propulsive force dotted with water-relative velocity at the thrust point.
    pub propulsive_power_w: f32,
}

pub struct PreparedMarineVehicle {
    controller_id: u64,
    semantic_id: u64,
    initial_chassis: RigidBody3D,
    initial_root: Motor,
    hull_center: [f32; 3],
    waves: SimdGerstnerSpectrum,
    air_density: f32,
    wind_m_s: [f32; 3],
    air_drag_area_m2: [f32; 3],
    air_drag_center: [f32; 3],
    water_current_m_s: [f32; 3],
    thrust_point: [f32; 3],
    thrust_direction: [f32; 3],
    thrust_limit_n: f32,
    rudder_point: [f32; 3],
    rudder_area_m2: f32,
    rudder_lift_slope: f32,
    rudder_limit_rad: f32,
}

macro_rules! k {
    ($name:literal) => { q_hash(concat!("https://qualiadb.org/ns/kinematics#", $name)) };
}

const WAVE_SLOTS: [u64; MAX_GERSTNER_HARMONICS] = [
    k!("wave0"), k!("wave1"), k!("wave2"), k!("wave3"),
    k!("wave4"), k!("wave5"), k!("wave6"), k!("wave7"),
];

/// Admit a marine vehicle and sea state from an authorized Q42 action, with
/// the collider fitted to its verified M64 point hull. The Etalon continuum
/// solver uses a cuboid hull proxy and fixed seawater density.
pub fn prepare_marine_vehicle(
    graph: &mut MotionGraph<'_, '_, '_>, action: &SemanticAction<'_>, material_iri: &str,
    hull_points: &M64Reader<'_>,
) -> Result<PreparedMarineVehicle, Error> {
    graph.check_budget(core::mem::size_of::<PreparedMarineVehicle>())?;
    let controller_id = action.authorize(graph)?;
    let source = graph.one(controller_id, k!("materialResource"))?;
    if graph.iri(source)? != material_iri { return Err(Error::InvalidMaterial); }
    let coordinates = graph.one(controller_id, k!("hullCoordinateConvention"))?;
    if graph.iri(coordinates)? != HULL_COORDINATES { return Err(Error::InvalidMaterial); }
    let semantic_id = graph.one(controller_id, k!("target"))?;
    graph.iri(semantic_id)?;
    let chunk = index(graph.one(controller_id, k!("hullChunk"))?)?;
    let start = index(graph.one(controller_id, k!("hullPointStart"))?)?;
    let count = index(graph.one(controller_id, k!("hullPointCount"))?)?;
    let (hull_center, half_extents) = hull_proxy(hull_points, chunk, start, count)?;
    let [mass, air_density, thrust_limit_n, rudder_area_m2, rudder_lift_slope, rudder_limit_rad] = read_lanes(graph, controller_id, [
        k!("massKg"), k!("airDensityKgPerCubicMetre"), k!("maximumThrustNewtons"),
        k!("rudderAreaSquareMetres"), k!("rudderLiftSlopePerRadian"), k!("rudderMaxRadians"),
    ])?;
    if mass <= 1e-6 || air_density <= 0.0 || thrust_limit_n <= 0.0
        || rudder_area_m2 <= 0.0 || rudder_lift_slope <= 0.0
        || !(0.0..=std::f32::consts::FRAC_PI_3).contains(&rudder_limit_rad) || rudder_limit_rad == 0.0
    {
        return Err(Error::InvalidFact);
    }
    let [ix, iy, iz] = read_lanes(graph, controller_id, [k!("inertiaXKgM2"), k!("inertiaYKgM2"), k!("inertiaZKgM2")])?;
    let inv_inertia = [ix.recip(), iy.recip(), iz.recip()];
    if inv_inertia.iter().any(|v| !v.is_finite() || *v <= 0.0) || ix + iy < iz || iy + iz < ix || iz + ix < iy {
        return Err(Error::InvalidMaterial);
    }
    let origin = graph.numeric_lanes(controller_id, [k!("originXMetres"), k!("originYMetres"), k!("originZMetres")])?;
    let orientation: [f64; 4] = graph.numeric_lanes(controller_id, [k!("bindW"), k!("bindX"), k!("bindY"), k!("bindZ")])?;
    if (orientation.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() > 1e-6 || origin.iter().any(|v| v.abs() > 1e6) {
        return Err(Error::InvalidFit);
    }
    let initial_root = Motor::from_rotation_translation(orientation, origin);
    let centre_pose = initial_root.mul(&Motor::from_rotation_translation(
        [1.0, 0.0, 0.0, 0.0], hull_center.map(f64::from),
    ));
    let mut initial_chassis = RigidBody3D::new_dynamic(centre_pose.to_pga_f32(), mass, ColliderShape3D::Cuboid { half_extents });
    initial_chassis.inv_inertia_local = inv_inertia;
    initial_chassis.linear_velocity = read_lanes(graph, controller_id, [k!("initialVelocityX"), k!("initialVelocityY"), k!("initialVelocityZ")])?;
    initial_chassis.angular_velocity = read_lanes(graph, controller_id, [k!("initialAngularX"), k!("initialAngularY"), k!("initialAngularZ")])?;
    let waves = read_waves(graph, controller_id)?;
    let air_drag_area_m2: [f32; 3] = read_lanes(graph, controller_id, [k!("airDragAreaX"), k!("airDragAreaY"), k!("airDragAreaZ")])?;
    let wind_m_s = read_lanes(graph, controller_id, [k!("windX"), k!("windY"), k!("windZ")])?;
    let water_current_m_s = read_lanes(graph, controller_id, [k!("waterCurrentX"), k!("waterCurrentY"), k!("waterCurrentZ")])?;
    let air_drag_center = read_lanes(graph, controller_id, [k!("airCentreX"), k!("airCentreY"), k!("airCentreZ")])?;
    let thrust_point = read_lanes(graph, controller_id, [k!("thrustPointX"), k!("thrustPointY"), k!("thrustPointZ")])?;
    let rudder_point = read_lanes(graph, controller_id, [k!("rudderPointX"), k!("rudderPointY"), k!("rudderPointZ")])?;
    let [forward_x, forward_z] = read_lanes(graph, controller_id, [k!("propulsorDirectionX"), k!("propulsorDirectionZ")])?;
    let length = forward_x.hypot(forward_z);
    if !length.is_finite() || length <= 1e-6 || air_drag_area_m2.iter().any(|v| *v < 0.0) {
        return Err(Error::InvalidFact);
    }
    let thrust_direction = [forward_x / length, 0.0, forward_z / length];
    Ok(PreparedMarineVehicle {
        controller_id, semantic_id, initial_chassis, initial_root, hull_center, waves, air_density,
        wind_m_s, air_drag_area_m2, air_drag_center, water_current_m_s, thrust_point, thrust_direction,
        thrust_limit_n, rudder_point, rudder_area_m2, rudder_lift_slope, rudder_limit_rad,
    })
}

impl PreparedMarineVehicle {
    pub fn semantic_id(&self) -> u64 { self.semantic_id }

    /// Require the render instance to use the same rigid, unit-scale metres
    /// and root pose as the admitted Q42/M64 physical model.
    pub fn validate_instance_root(&self, instance: &InstanceOverrideHeader) -> Result<(), Error> {
        let [w, x, y, z, e01, e02, e03, dw] = instance.local_pga_motor.map(f64::from);
        let authored = Motor { r_w: w, r_x: x, r_y: y, r_z: z, d_w: dw, d_x: -e01, d_y: -e02, d_z: -e03 };
        if instance.uniform_scale != 1.0 || !rest_poses_agree(self.initial_root, authored) {
            return Err(Error::InvalidFit);
        }
        Ok(())
    }

    pub fn initial_state(&self) -> MarineVehicleState {
        MarineVehicleState { chassis: self.initial_chassis, time_s: 0.0, model_id: self.controller_id }
    }

    /// Integrate an admitted vessel in fixed-size substeps. State publishes
    /// only after all solver outputs and the emitted world/local motors validate.
    pub fn advance(
        &self, state: &mut MarineVehicleState, control: MarineVehicleControl, dt_s: f64,
    ) -> Result<MarineVehicleStep, Error> {
        if state.model_id != self.controller_id || !dt_s.is_finite() || dt_s <= 0.0 || dt_s > MAX_STEP_S
            || !(-1.0..=1.0).contains(&control.throttle) || !(-1.0..=1.0).contains(&control.rudder)
        {
            return Err(Error::InvalidCommand);
        }
        let mut next = *state;
        let steps = (dt_s / INTERNAL_STEP_S).ceil() as u32;
        let dt = (dt_s / f64::from(steps)) as f32;
        if !dt.is_finite() || dt <= 0.0 { return Err(Error::InvalidCommand); }
        let mut last = None;
        for _ in 0..steps {
            last = Some(self.step_subinterval(&mut next.chassis, control, next.time_s as f32, dt)?);
            next.time_s += dt_s / f64::from(steps);
        }
        let (submerged_volume_m3, buoyancy_n, hydrodynamic_drag_n, aerodynamic_drag_n, propulsive_power_w) =
            last.ok_or(Error::InvalidCommand)?;
        let world_pose = Motor::from_pga_f32(next.chassis.pose).mul(&Motor::from_rotation_translation(
            [1.0, 0.0, 0.0, 0.0], self.hull_center.map(|v| -f64::from(v)),
        ));
        let local_delta = self.initial_root.conjugate().mul(&world_pose);
        if !super::super::valid_motor(&world_pose) || !super::super::valid_motor(&local_delta)
            || !next.time_s.is_finite() || next.time_s > 86_400.0
        {
            return Err(Error::InvalidCommand);
        }
        *state = next;
        Ok(MarineVehicleStep {
            semantic_id: self.semantic_id, local_delta, world_pose, time_s: next.time_s,
            linear_velocity_m_s: next.chassis.linear_velocity, angular_velocity_rad_s: next.chassis.angular_velocity,
            submerged_volume_m3, buoyancy_n, hydrodynamic_drag_n, aerodynamic_drag_n, propulsive_power_w,
        })
    }

    fn step_subinterval(
        &self, body: &mut RigidBody3D, control: MarineVehicleControl, time: f32, dt: f32,
    ) -> Result<(f32, f32, [f32; 3], [f32; 3], f32), Error> {
        let pose = body.pose;
        let mut orientation = pose;
        orientation.d = [0.0; 4];
        let right = sandwich_point(orientation, [1.0, 0.0, 0.0]);
        let up = sandwich_point(orientation, [0.0, 1.0, 0.0]);
        let forward = sandwich_point(orientation, [0.0, 0.0, 1.0]);
        let axes = [right, up, forward];
        let angular_world = local_to_world(axes, body.angular_velocity);
        let point = |p| sandwich_point(pose, sub(p, self.hull_center));
        let com = sandwich_point(pose, [0.0; 3]);
        let velocity_at = |position| add(body.linear_velocity, cross(angular_world, sub(position, com)));

        let aero_point = point(self.air_drag_center);
        let air_velocity = sub(velocity_at(aero_point), self.wind_m_s);
        let aero_local = std::array::from_fn(|axis| {
            let speed = dot(air_velocity, axes[axis]);
            -0.5 * self.air_density * self.air_drag_area_m2[axis] * speed.abs() * speed
        });
        let aerodynamic_drag_n = local_to_world(axes, aero_local);
        let thrust_point = point(self.thrust_point);
        let propulsion_axis = local_to_world(axes, self.thrust_direction);
        let water_at_thrust = sub(velocity_at(thrust_point), self.water_current_m_s);
        let (thrust_surface, _) = self.waves.sample_surface(thrust_point[0], thrust_point[2], time);
        let thrust = if thrust_point[1] <= thrust_surface {
            mul(propulsion_axis, self.thrust_limit_n * control.throttle)
        } else { [0.0; 3] };
        let propulsive_power_w = dot(thrust, water_at_thrust);
        let rudder_point = point(self.rudder_point);
        let rudder_forward_speed = dot(sub(velocity_at(rudder_point), self.water_current_m_s), propulsion_axis);
        let rudder_lateral = cross(up, propulsion_axis);
        let (surface, _) = self.waves.sample_surface(rudder_point[0], rudder_point[2], time);
        let rudder_force = if rudder_point[1] <= surface {
            let deflection = control.rudder * self.rudder_limit_rad;
            let lift_coefficient = (self.rudder_lift_slope * deflection.sin()).clamp(-1.2, 1.2);
            mul(rudder_lateral, -0.5 * WATER_DENSITY_KG_M3 * self.rudder_area_m2 * lift_coefficient
                * rudder_forward_speed * rudder_forward_speed.abs())
        } else { [0.0; 3] };

        let mut hydro_body = *body;
        hydro_body.linear_velocity = sub(body.linear_velocity, self.water_current_m_s);
        hydro_body.angular_velocity = [0.0; 3];
        hydro_body.inv_inertia_local = [1.0; 3];
        let hydro = couple_rigid_body_with_hydro(&mut hydro_body, &self.waves, time, dt);
        body.linear_velocity = add(hydro_body.linear_velocity, self.water_current_m_s);
        let wet = (hydro.submerged_volume_m3 / body.collider_shape.volume()).clamp(0.0, 1.0);
        let angular_damping = (1.0 - 2.5 * wet * dt).max(0.0);
        for (axis, basis) in axes.iter().enumerate() {
            // Etalon's coupling emits a world-frame righting impulse with unit inertia.
            body.angular_velocity[axis] = body.angular_velocity[axis] * angular_damping
                + dot(hydro_body.angular_velocity, *basis) * body.inv_inertia_local[axis];
        }
        body.linear_velocity[1] -= GRAVITY_M_S2 * dt;
        for (force, location) in [(aerodynamic_drag_n, aero_point), (thrust, thrust_point), (rudder_force, rudder_point)] {
            let impulse = mul(force, dt);
            let moment = cross(sub(location, com), impulse);
            body.apply_linear_impulse(impulse);
            for (axis, basis) in axes.iter().enumerate() {
                body.angular_velocity[axis] += dot(moment, *basis) * body.inv_inertia_local[axis];
            }
        }
        if body.linear_velocity.iter().chain(&body.angular_velocity).any(|v| !v.is_finite() || v.abs() > 4096.0)
            || aerodynamic_drag_n.iter().chain(&hydro.drag_force_n).any(|v| !v.is_finite())
            || !propulsive_power_w.is_finite() || !hydro.buoyant_force_n.is_finite() || !hydro.submerged_volume_m3.is_finite()
        {
            return Err(Error::InvalidCommand);
        }
        body.integrate_pose(dt);
        let world = Motor::from_pga_f32(body.pose);
        if !super::super::valid_motor(&world) { return Err(Error::InvalidCommand); }
        Ok((hydro.submerged_volume_m3, hydro.buoyant_force_n, hydro.drag_force_n, aerodynamic_drag_n, propulsive_power_w))
    }
}

fn hull_proxy(reader: &M64Reader<'_>, chunk: usize, start: usize, count: usize) -> Result<([f32; 3], [f32; 3]), Error> {
    if !(4..=MAX_HULL_POINTS).contains(&count) || reader.georaster().is_some() || reader.lrm3().is_some()
        || reader.header().spatial_origin != [0.0; 2] || reader.header().elevation_datum != [0.0, 1.0]
    {
        return Err(Error::InvalidMaterial);
    }
    let chunk = reader.chunk(chunk).ok_or(Error::InvalidMaterial)?;
    if chunk.directory_entry().srid_geodetic_code != 0 { return Err(Error::InvalidMaterial); }
    let end = start.checked_add(count).ok_or(Error::InvalidMaterial)?;
    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for i in start..end {
        let Some(M64DecodedRecord::Point(point)) = chunk.decode_record(i).map_err(|_| Error::InvalidMaterial)?
        else { return Err(Error::InvalidMaterial); };
        let coordinates = [point.x, point.y, point.z];
        if coordinates.iter().any(|v| !v.is_finite() || v.abs() > 1e4) { return Err(Error::InvalidMaterial); }
        for axis in 0..3 {
            min[axis] = min[axis].min(coordinates[axis]);
            max[axis] = max[axis].max(coordinates[axis]);
        }
    }
    let center = std::array::from_fn(|axis| 0.5 * (min[axis] + max[axis]));
    let half: [f32; 3] = std::array::from_fn(|axis| 0.5 * (max[axis] - min[axis]));
    if half.iter().any(|v| *v <= 1e-4) { return Err(Error::InvalidMaterial); }
    Ok((center, half))
}

fn read_waves(graph: &mut MotionGraph<'_, '_, '_>, controller: u64) -> Result<SimdGerstnerSpectrum, Error> {
    let [sea_level] = read_lanes(graph, controller, [k!("seaLevelMetres")])?;
    let active_waves = index(graph.one(controller, k!("waveCount"))?)?;
    if active_waves > MAX_GERSTNER_HARMONICS { return Err(Error::InvalidFact); }
    let mut spectrum = SimdGerstnerSpectrum { sea_level, active_waves, ..Default::default() };
    for (ordinal, slot) in WAVE_SLOTS[..active_waves].iter().enumerate() {
        let wave = graph.one(controller, *slot)?;
        graph.iri(wave)?;
        let [x, z, amplitude, wavelength, steepness, phase] = read_lanes(graph, wave, [
            k!("waveDirectionX"), k!("waveDirectionZ"), k!("waveAmplitudeMetres"),
            k!("waveLengthMetres"), k!("waveSteepness"), k!("wavePhaseRadians"),
        ])?;
        let length = x.hypot(z);
        if length <= 1e-5 || !length.is_finite() || amplitude < 0.0 || wavelength < 0.01
            || amplitude > wavelength || !(0.0..=1.0).contains(&steepness)
        {
            return Err(Error::InvalidFact);
        }
        spectrum.waves[ordinal] = GerstnerWave { direction: [x / length, z / length], amplitude, wavelength, steepness, phase };
    }
    Ok(spectrum)
}

fn read_lanes<const N: usize>(graph: &mut MotionGraph<'_, '_, '_>, id: u64, predicates: [u64; N]) -> Result<[f32; N], Error> {
    let values = graph.numeric_lanes(id, predicates)?.map(|v| v as f32);
    if values.iter().any(|v| !v.is_finite()) { return Err(Error::InvalidFact); }
    Ok(values)
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 { a.into_iter().zip(b).map(|(a, b)| a * b).sum() }
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { std::array::from_fn(|i| a[i] + b[i]) }
fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] { std::array::from_fn(|i| a[i] - b[i]) }
fn mul(a: [f32; 3], s: f32) -> [f32; 3] { a.map(|n| n * s) }
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn local_to_world(axes: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| (0..3).map(|axis| axes[axis][i] * v[axis]).sum())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balanced_box() -> PreparedMarineVehicle {
        let initial_root = Motor::identity();
        let mass = 4.0 * WATER_DENSITY_KG_M3;
        let initial_chassis = RigidBody3D::new_dynamic(initial_root.to_pga_f32(), mass,
            ColliderShape3D::Cuboid { half_extents: [1.0; 3] });
        PreparedMarineVehicle {
            controller_id: 1, semantic_id: 2, initial_chassis, initial_root, hull_center: [0.0; 3],
            waves: SimdGerstnerSpectrum::default(), air_density: 1.2, wind_m_s: [0.0; 3],
            air_drag_area_m2: [1.0; 3], air_drag_center: [0.0; 3], water_current_m_s: [0.0; 3],
            thrust_point: [0.0, -0.5, 0.0], thrust_direction: [0.0, 0.0, 1.0], thrust_limit_n: 2500.0,
            rudder_point: [0.0, -0.5, -0.9], rudder_area_m2: 0.25, rudder_lift_slope: 4.0, rudder_limit_rad: 0.3,
        }
    }

    #[test]
    fn native_buoyancy_balances_calm_gravity_and_offset_thrust_generates_pitch() {
        let vehicle = balanced_box();
        let mut state = vehicle.initial_state();
        let calm = vehicle.advance(&mut state, MarineVehicleControl::default(), 0.02).unwrap();
        assert_eq!(calm.semantic_id, 2);
        assert!((calm.submerged_volume_m3 - 4.0).abs() < 1e-6);
        assert!(calm.linear_velocity_m_s.iter().all(|v| v.abs() < 1e-6));
        let powered = vehicle.advance(&mut state, MarineVehicleControl { throttle: 1.0, rudder: 0.0 }, 0.03).unwrap();
        assert!(powered.linear_velocity_m_s[2] > 0.0);
        assert!(powered.angular_velocity_rad_s[0] < 0.0);
        assert!(powered.propulsive_power_w > 0.0);
        assert!(powered.local_delta.transform_point([0.0; 3])[2] > 0.0);
    }

    #[test]
    fn invalid_control_leaves_the_resident_body_untouched() {
        let vehicle = balanced_box();
        let mut state = vehicle.initial_state();
        let before = state;
        assert!(matches!(vehicle.advance(&mut state, MarineVehicleControl { throttle: 2.0, rudder: 0.0 }, 0.01), Err(Error::InvalidCommand)));
        assert_eq!(state.chassis, before.chassis);
        assert_eq!(state.time_s, before.time_s);
    }
}
