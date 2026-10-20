//! Two-Way Rigid-Body ↔ Continuum Physics Coupling (`continuum_bridge.rs`).
//!
//! Couples 3D XPBD [`RigidBody3D`] simulation directly with `QualiaDB`'s continuum engines:
//! 1. **Geomechanics (`geomechanics.rs`)**: Normal contact force on soil strata $\to$ Bekker /
//!    Mohr–Coulomb plastic sinkage depth, Kozeny–Carman compaction (`evaluate_soil_compaction`),
//!    and rolling/bulldozing resistance impulse applied back to the [`RigidBody3D`].
//! 2. **Hydrodynamics (`hydro.rs`)**: Submerged 3D collider volume against [`SimdGerstnerSpectrum`]
//!    $\to$ 3D Archimedes upward buoyancy force, wave-normal righting torque, and quadratic fluid
//!    drag/lift applied to the [`RigidBody3D`].
//! 3. **Botanical Biomechanics (`metamer_fspm.rs`)**: Rigid-body collision impulse against a
//!    botanical stem in [`MetamerPlantGraph`] $\to$ elastic cantilever deflection (`pitch_rad` /
//!    `bending_moment_nm`) or modulus-of-rupture branch fracture when bending stress exceeds
//!    $\sigma_{\text{MOR}}$.

use crate::geomechanics::{evaluate_soil_compaction, SoilMechanicsProfile};
use crate::hydro::{SimdGerstnerSpectrum, GRAVITY_M_S2, WATER_DENSITY_KG_M3};
use crate::metamer_fspm::MetamerPlantGraph;
use crate::physics3d::xpbd_solver::RigidBody3D;

/// Telemetry returned by two-way soil-rigid-body coupling.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SoilCouplingOutcome {
    /// Contact ground pressure in kPa.
    pub contact_pressure_kpa: f32,
    /// Bekker / Mohr–Coulomb plastic sinkage depth in meters.
    pub sinkage_depth_m: f32,
    /// Updated soil porosity after compaction.
    pub compacted_porosity: f32,
    /// Kozeny–Carman hydraulic conductivity multiplier after compaction.
    pub permeability_multiplier: f32,
    /// Horizontal compaction & bulldozing drag impulse (N·s) applied to the body.
    pub resistance_impulse_ns: f32,
}

/// Applies two-way geotechnical force coupling between a [`RigidBody3D`] (wheel, foot, boulder, or
/// chassis) and a [`SoilMechanicsProfile`].
pub fn couple_rigid_body_with_soil(
    body: &mut RigidBody3D,
    normal_force_n: f32,
    contact_area_m2: f32,
    current_porosity: f32,
    soil: &SoilMechanicsProfile,
    dt: f32,
) -> SoilCouplingOutcome {
    let area = contact_area_m2.max(1e-3);
    let pressure_kpa = (normal_force_n.max(0.0) * 1e-3) / area;
    let (compacted_porosity, permeability_multiplier) =
        evaluate_soil_compaction(pressure_kpa, current_porosity, soil);

    // Bekker / Mohr-Coulomb bearing capacity & plastic sinkage
    let bearing_modulus_kpa =
        (soil.cohesion_kpa * 12.0 + soil.unconfined_compressive_strength_kpa * 1.5).max(25.0);
    let sinkage_depth_m = ((pressure_kpa / bearing_modulus_kpa).powf(1.2) * 0.15).clamp(0.0, 0.45);

    // Compaction bulldozing resistance force: F_res ~ gamma * z^2 * N_phi + c * z
    let horiz_speed =
        (body.linear_velocity[0] * body.linear_velocity[0] + body.linear_velocity[2] * body.linear_velocity[2])
            .sqrt();
    let resistance_impulse_ns = if horiz_speed > 1e-4 && sinkage_depth_m > 1e-4 {
        let drag_force_n =
            normal_force_n * (sinkage_depth_m * 1.8 + 0.02) + soil.soil_tool_adhesion_kpa * area * 200.0;
        let impulse = drag_force_n * dt;
        let dir_x = -body.linear_velocity[0] / horiz_speed;
        let dir_z = -body.linear_velocity[2] / horiz_speed;
        body.apply_linear_impulse([dir_x * impulse, 0.0, dir_z * impulse]);
        impulse
    } else {
        0.0
    };

    SoilCouplingOutcome {
        contact_pressure_kpa: pressure_kpa,
        sinkage_depth_m,
        compacted_porosity,
        permeability_multiplier,
        resistance_impulse_ns,
    }
}

/// Telemetry returned by two-way hydrodynamic coupling.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HydroCouplingOutcome {
    /// Submerged volume of the 3D collider in $m^3$.
    pub submerged_volume_m3: f32,
    /// Upward Archimedes buoyant force in Newtons.
    pub buoyant_force_n: f32,
    /// Quadratic hydrodynamic drag force vector `[fx, fy, fz]` in Newtons.
    pub drag_force_n: [f32; 3],
}

/// Applies two-way 3D Archimedes buoyancy, wave-normal righting torque, and quadratic fluid drag
/// between a [`RigidBody3D`] and a [`SimdGerstnerSpectrum`] wavefield.
pub fn couple_rigid_body_with_hydro(
    body: &mut RigidBody3D,
    spectrum: &SimdGerstnerSpectrum,
    time_sec: f32,
    dt: f32,
) -> HydroCouplingOutcome {
    let pos = body.pose.translation();
    let aabb = body.collider_shape.world_aabb(&body.pose);
    let bottom_y = aabb.min[1];
    let top_y = aabb.max[1];
    let height = (top_y - bottom_y).max(1e-3);

    let (water_y, wave_normal) = spectrum.sample_surface(pos[0], pos[2], time_sec);
    if water_y <= bottom_y {
        return HydroCouplingOutcome {
            submerged_volume_m3: 0.0,
            buoyant_force_n: 0.0,
            drag_force_n: [0.0; 3],
        };
    }

    let submerged_fraction = ((water_y - bottom_y) / height).clamp(0.0, 1.0);
    let total_vol = body.collider_shape.volume();
    let submerged_volume_m3 = total_vol * submerged_fraction;
    let buoyant_force_n = WATER_DENSITY_KG_M3 * submerged_volume_m3 * GRAVITY_M_S2;

    // 1. Apply upward Archimedes buoyancy + slight wave-normal horizontal surge
    let f_buoy = [
        wave_normal[0] * buoyant_force_n * 0.15,
        buoyant_force_n,
        wave_normal[2] * buoyant_force_n * 0.15,
    ];

    // 2. Quadratic + linear hydrodynamic drag opposing velocity
    let v = body.linear_velocity;
    let speed = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    let drag_coeff = 0.5 * WATER_DENSITY_KG_M3 * submerged_fraction * 0.45 * (total_vol.powf(0.6667));
    let drag_force_n = [
        -v[0] * (speed * drag_coeff + 15.0 * submerged_fraction),
        -v[1] * (speed * drag_coeff + 20.0 * submerged_fraction),
        -v[2] * (speed * drag_coeff + 15.0 * submerged_fraction),
    ];

    body.apply_linear_impulse([
        (f_buoy[0] + drag_force_n[0]) * dt,
        (f_buoy[1] + drag_force_n[1]) * dt,
        (f_buoy[2] + drag_force_n[2]) * dt,
    ]);

    // 3. Angular damping & wave-normal righting torque
    let body_up = body.pose.transform_vector([0.0, 1.0, 0.0]);
    let righting_torque = [
        (body_up[1] * wave_normal[2] - body_up[2] * wave_normal[1]) * buoyant_force_n * 0.08,
        (body_up[2] * wave_normal[0] - body_up[0] * wave_normal[2]) * buoyant_force_n * 0.08,
        (body_up[0] * wave_normal[1] - body_up[1] * wave_normal[0]) * buoyant_force_n * 0.08,
    ];
    body.angular_velocity[0] = (body.angular_velocity[0]
        + righting_torque[0] * body.inv_inertia_local[0] * dt)
        * (1.0 - 2.5 * submerged_fraction * dt).max(0.0);
    body.angular_velocity[1] = (body.angular_velocity[1]
        + righting_torque[1] * body.inv_inertia_local[1] * dt)
        * (1.0 - 2.5 * submerged_fraction * dt).max(0.0);
    body.angular_velocity[2] = (body.angular_velocity[2]
        + righting_torque[2] * body.inv_inertia_local[2] * dt)
        * (1.0 - 2.5 * submerged_fraction * dt).max(0.0);

    HydroCouplingOutcome {
        submerged_volume_m3,
        buoyant_force_n,
        drag_force_n,
    }
}

/// Telemetry returned by rigid-body ↔ botanical metamer impact coupling.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BotanicalImpactOutcome {
    /// Peak bending moment $M$ (N·m) induced on the impacted metamer.
    pub induced_moment_nm: f32,
    /// Peak outer-fiber bending stress $\sigma = \frac{M \cdot r}{I}$ in MPa.
    pub fiber_stress_mpa: f32,
    /// Elastic angular deflection $\Delta\theta$ (radians) applied to the branch.
    pub elastic_deflection_rad: f32,
    /// True if fiber stress exceeded the wood Modulus of Rupture ($\sigma_{\text{MOR}}$), snapping the branch!
    pub branch_fractured: bool,
}

/// Couples a 3D rigid-body impact against `metamer_idx` in a [`MetamerPlantGraph`].
///
/// Computes outer-fiber bending stress $\sigma = \frac{M \cdot r}{I}$ from Shinozaki pipe-model
/// geometry (`radius_m` and `second_moment_i`). If $\sigma \ge \sigma_{\text{MOR}}$, fractures the
/// branch; otherwise applies elastic cantilever deflection and reflects a restitution impulse back
/// onto `body`.
pub fn couple_rigid_body_with_botanical_stem(
    body: &mut RigidBody3D,
    plant: &mut MetamerPlantGraph,
    metamer_idx: usize,
    impact_impulse_ns: f32,
    contact_duration_s: f32,
    modulus_of_rupture_mpa: f32,
) -> BotanicalImpactOutcome {
    if metamer_idx >= plant.count {
        return BotanicalImpactOutcome {
            induced_moment_nm: 0.0,
            fiber_stress_mpa: 0.0,
            elastic_deflection_rad: 0.0,
            branch_fractured: false,
        };
    }

    let r = plant.radius_m(metamer_idx);
    let i_moment = plant.second_moment_i(metamer_idx).max(1e-12);
    let length = plant.metamers[metamer_idx].length_m.max(0.05);
    let impact_force_n = impact_impulse_ns.abs() / contact_duration_s.max(0.005);
    let induced_moment_nm = impact_force_n * (length * 0.5);

    // Outer-fiber bending stress in MPa: sigma = (M * r / I) * 1e-6
    let fiber_stress_mpa = ((induced_moment_nm * r) / i_moment) * 1e-6;
    let e_pa = (plant.wood_elastic_modulus_mpa.max(100.0)) * 1e6;
    let elastic_deflection_rad =
        ((induced_moment_nm * length) / (e_pa * i_moment)).clamp(0.0, 1.2);

    plant.metamers[metamer_idx].bending_moment_nm += induced_moment_nm;

    if fiber_stress_mpa >= modulus_of_rupture_mpa.max(5.0) {
        // Branch fractures: detach from parent metamer and absorb part of the rigid body's momentum
        plant.metamers[metamer_idx].parent_idx = crate::metamer_fspm::NO_PARENT_METAMER;
        plant.metamers[metamer_idx].active_pipes = 0;
        let absorbed = (impact_impulse_ns * 0.35).min(50.0);
        body.apply_linear_impulse([-absorbed, 0.0, 0.0]);
        BotanicalImpactOutcome {
            induced_moment_nm,
            fiber_stress_mpa,
            elastic_deflection_rad,
            branch_fractured: true,
        }
    } else {
        // Elastic deflection + springback impulse on rigid body
        plant.metamers[metamer_idx].pitch_rad += elastic_deflection_rad;
        let rebound = impact_impulse_ns * 0.65;
        body.apply_linear_impulse([-rebound, 0.0, 0.0]);
        BotanicalImpactOutcome {
            induced_moment_nm,
            fiber_stress_mpa,
            elastic_deflection_rad,
            branch_fractured: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metamer_fspm::ThermalPhenology;
    use crate::physics3d::gjk_epa::ColliderShape3D;
    use qualia_core_db::render::pga::motor_translate;

    #[test]
    fn continuum_bridge_couples_soil_hydro_and_botanical_fspm() {
        // 1. Soil coupling: heavy wheel compacts loam and slows down
        let mut wheel_body = RigidBody3D::new_dynamic(
            motor_translate([0.0, 0.4, 0.0]),
            40.0,
            ColliderShape3D::Sphere { radius: 0.4 },
        );
        wheel_body.linear_velocity = [5.0, 0.0, 0.0];
        let soil = SoilMechanicsProfile::loam();
        let soil_out =
            couple_rigid_body_with_soil(&mut wheel_body, 4_500.0, 0.04, 0.45, &soil, 1.0 / 64.0);
        assert!(soil_out.sinkage_depth_m > 0.0);
        assert!(soil_out.compacted_porosity < 0.45);
        assert!(wheel_body.linear_velocity[0] < 5.0);

        // 2. Hydro coupling: submerged crate receives upward Archimedes buoyancy
        let mut crate_body = RigidBody3D::new_dynamic(
            motor_translate([0.0, -1.0, 0.0]),
            50.0,
            ColliderShape3D::Cuboid {
                half_extents: [0.5, 0.5, 0.5],
            },
        );
        let spectrum = SimdGerstnerSpectrum::default();
        let hydro_out =
            couple_rigid_body_with_hydro(&mut crate_body, &spectrum, 0.0, 1.0 / 64.0);
        assert!((hydro_out.submerged_volume_m3 - 1.0).abs() < 1e-4);
        assert!(hydro_out.buoyant_force_n > 9_000.0);
        assert!(crate_body.linear_velocity[1] > 0.0);

        // 3. Botanical coupling: moderate impact bends branch elastically; heavy impact fractures it
        let mut tree = MetamerPlantGraph::new(ThermalPhenology::default());
        tree.solve_growth_cycle(1.0, 0.8);
        let mut boulder = RigidBody3D::new_dynamic(
            motor_translate([0.0, 0.2, 0.0]),
            25.0,
            ColliderShape3D::Sphere { radius: 0.3 },
        );
        boulder.linear_velocity = [3.0, 0.0, 0.0];

        let bend = couple_rigid_body_with_botanical_stem(
            &mut boulder,
            &mut tree,
            1,
            0.02,
            0.05,
            65.0,
        );
        assert!(!bend.branch_fractured);
        assert!(bend.elastic_deflection_rad > 0.0);

        let snap = couple_rigid_body_with_botanical_stem(
            &mut boulder,
            &mut tree,
            1,
            25.0,
            0.01,
            65.0,
        );
        assert!(snap.branch_fractured);
        assert_eq!(tree.metamers[1].parent_idx, crate::metamer_fspm::NO_PARENT_METAMER);
    }
}
