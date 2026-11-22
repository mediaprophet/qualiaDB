//! Semantically admitted serial hinge IK and DCA forward dynamics.

use crate::{
    etalon_render::skeletal::{
        BodyDensitometryInertia, DcaBodyNode, solve_dca_parallel_chain_zero_heap,
    },
    q_hash,
    q42::m64_matter::{M64DecodedRecord, M64Reader},
};

use super::{
    FittedRotation, MotionGraph, Motor, SemanticAction, SemanticMotionError as Error,
    SemanticRotation, fitted_rotation,
    graph::index,
};

const MAX_CHAIN: usize = 16;
const MATERIAL_RESOURCE: u64 = q_hash("https://qualiadb.org/ns/kinematics#materialResource");
const CHAIN: u64 = q_hash("https://qualiadb.org/ns/kinematics#chainAction");
const SLOT: u64 = q_hash("https://qualiadb.org/ns/kinematics#chainSlot");
const CHAIN_LENGTH: u64 = q_hash("https://qualiadb.org/ns/kinematics#chainLength");
const SECONDARY_TIP: u64 = q_hash("https://qualiadb.org/ns/kinematics#secondaryTip");

#[derive(Clone, Copy)]
struct Link {
    body: DcaBodyNode,
    fit: FittedRotation,
    kp: f64,
    kd: f64,
    torque_limit: f64,
}

pub struct PreparedDcaChain {
    links: [Option<Link>; MAX_CHAIN],
    length: usize,
    gravity: [f64; 3],
    local_tip: [f64; 3],
    local_secondary_tip: Option<[f64; 3]>,
    tolerance: f64,
    damping: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct DcaChainState {
    pub angle_radians: f64,
    pub velocity_radians_s: f64,
}

pub struct DcaChainStep {
    pub tracking_residual_m: f64,
    pub ik_residual_m: f64,
}

macro_rules! k {
    ($name:literal) => {
        q_hash(concat!("https://qualiadb.org/ns/kinematics#", $name))
    };
}

/// Admit a serial revolute chain in stable, declared SKL2 joint order. Every
/// link binds its Q42 structural scalars and M64 tissue before the frame loop.
pub fn prepare_dca_motor_chain(
    graph: &mut MotionGraph<'_, '_, '_>,
    action: &SemanticAction<'_>,
    joint_iris: &[&str],
    material_iri: &str,
    material: &M64Reader<'_>,
) -> Result<PreparedDcaChain, Error> {
    graph.check_budget(core::mem::size_of::<PreparedDcaChain>())?;
    let controller = action.authorize(graph)?;
    let source = graph.one(controller, MATERIAL_RESOURCE)?;
    if graph.iri(source)? != material_iri {
        return Err(Error::InvalidMaterial);
    }
    let length = index(graph.one(controller, CHAIN_LENGTH)?)?;
    if !(2..=MAX_CHAIN).contains(&length) || joint_iris.len() != length {
        return Err(Error::InvalidCommand);
    }
    let gravity =
        graph.numeric_lanes(controller, [k!("gravityX"), k!("gravityY"), k!("gravityZ")])?;
    let local_tip = graph.numeric_lanes(
        controller,
        [k!("tipXMetres"), k!("tipYMetres"), k!("tipZMetres")],
    )?;
    let [tolerance, damping] =
        graph.numeric_lanes(controller, [k!("ikToleranceMetres"), k!("ikDamping")])?;
    if !(1e-7..=1e-2).contains(&tolerance) || !(1e-6..=1.0).contains(&damping) {
        return Err(Error::InvalidFact);
    }
    let local_secondary_tip = match graph.one(controller, SECONDARY_TIP) {
        Ok(marker) => {
            graph.iri(marker)?;
            let point = graph.numeric_lanes(
                marker,
                [k!("tipXMetres"), k!("tipYMetres"), k!("tipZMetres")],
            )?;
            let separation = norm(sub(point, local_tip));
            if !separation.is_finite() || separation <= 2.0 * tolerance {
                return Err(Error::InvalidFact);
            }
            Some(point)
        }
        Err(Error::MissingFact) => None,
        Err(error) => return Err(error),
    };
    let mut chain = PreparedDcaChain {
        links: [None; MAX_CHAIN],
        length,
        gravity,
        local_tip,
        local_secondary_tip,
        tolerance,
        damping,
    };
    for (i, iri) in joint_iris.iter().enumerate() {
        let id = q_hash(iri);
        if graph.iri(id)? != *iri
            || graph.one(id, CHAIN)? != controller
            || index(graph.one(id, SLOT)?)? != i
        {
            return Err(Error::InvalidFact);
        }
        let fit = graph.fit(id)?;
        if fit.local_pivot_m.iter().any(|v| v.abs() > 1e-9) {
            return Err(Error::InvalidFit);
        }
        let origin = graph.numeric_lanes(
            id,
            [
                k!("originXMetres"),
                k!("originYMetres"),
                k!("originZMetres"),
            ],
        )?;
        if i != 0 && norm(origin) <= 1e-9 {
            return Err(Error::InvalidFact);
        }
        let rotation: [f64; 4] =
            graph.numeric_lanes(id, [k!("bindW"), k!("bindX"), k!("bindY"), k!("bindZ")])?;
        if (rotation.iter().map(|v| v * v).sum::<f64>() - 1.0).abs() > 1e-6 {
            return Err(Error::InvalidFit);
        }
        let chunk = index(graph.one(id, k!("materialChunk"))?)?;
        let record = index(graph.one(id, k!("materialRecord"))?)?;
        let Some(M64DecodedRecord::BioTissue(tissue)) = material
            .chunk(chunk)
            .ok_or(Error::InvalidMaterial)?
            .decode_record(record)
            .map_err(|_| Error::InvalidMaterial)?
        else {
            return Err(Error::InvalidMaterial);
        };
        let com =
            graph.numeric_lanes(id, [k!("comXMetres"), k!("comYMetres"), k!("comZMetres")])?;
        let radii: [f64; 3] = graph.numeric_lanes(
            id,
            [
                k!("gyrationXMetres"),
                k!("gyrationYMetres"),
                k!("gyrationZMetres"),
            ],
        )?;
        let [
            bone,
            bmd,
            trabecular,
            thickness,
            muscle,
            adipose,
            adipose_density,
            kp,
            kd,
            torque,
        ] = graph.numeric_lanes(
            id,
            [
                k!("boneVolumeM3"),
                k!("corticalBmdGramsPerCubicCentimetre"),
                k!("trabecularBoneVolumeFraction"),
                k!("corticalThicknessMillimetres"),
                k!("muscleVolumeM3"),
                k!("adiposeVolumeM3"),
                k!("adiposeDensityKgPerCubicMetre"),
                k!("proportionalGainNm"),
                k!("derivativeGainNmSeconds"),
                k!("maxTorqueNm"),
            ],
        )?;
        let density = f64::from(tissue.apparent_density_kg_m3);
        if ![
            bmd,
            trabecular,
            thickness,
            density,
            adipose_density,
            kp,
            torque,
        ]
        .iter()
        .all(|v| *v > 0.0)
            || trabecular > 1.0
            || [bone, muscle, adipose, kd].iter().any(|v| *v < 0.0)
            || radii.iter().any(|v| *v <= 0.0)
        {
            return Err(Error::InvalidMaterial);
        }
        let inertia = BodyDensitometryInertia {
            bone_volume_m3: bone,
            cortical_bmd_g_cm3: bmd,
            trabecular_bv_tv: trabecular,
            cortical_thickness_mm: thickness,
            muscle_volume_m3: muscle,
            muscle_density_kg_m3: density,
            adipose_volume_m3: adipose,
            adipose_density_kg_m3: adipose_density,
            com_local_m: com,
            unit_gyration_radii_sq_m2: radii.map(|r| r * r),
        };
        inertia
            .compute_spatial_inertia()
            .map_err(|_| Error::InvalidMaterial)?;
        let axis = fit.local_axis.map(|v| v / norm(fit.local_axis));
        chain.links[i] = Some(Link {
            body: DcaBodyNode {
                parent: i.checked_sub(1).map(|id| id as u16),
                joint_axis: [axis[0], axis[1], axis[2], 0.0, 0.0, 0.0],
                parent_to_joint_origin_m: origin,
                parent_to_joint_rot_wxyz: rotation,
                q: 0.0,
                qd: 0.0,
                tau: 0.0,
                external_wrench: [0.0; 6],
                synovial_stiffness: 0.0,
                synovial_damping: 0.0,
                inertia,
            },
            fit,
            kp,
            kd,
            torque_limit: torque,
        });
    }
    Ok(chain)
}

impl PreparedDcaChain {
    pub fn len(&self) -> usize {
        self.length
    }

    /// Authored parent-relative bind pose for the joint in this Q42 slot.
    pub fn rest_motor(&self, slot: usize) -> Option<Motor> {
        let link = self.links.get(slot)?.as_ref()?;
        Some(Motor::from_rotation_translation(
            link.body.parent_to_joint_rot_wxyz,
            link.body.parent_to_joint_origin_m,
        ))
    }

    pub fn initialize_rest_state(&self, state: &mut [DcaChainState]) -> Result<(), Error> {
        if state.len() < self.length {
            return Err(Error::OutputTooSmall);
        }
        let mut staged = [DcaChainState::default(); MAX_CHAIN];
        for (i, link) in self.links[..self.length].iter().flatten().enumerate() {
            staged[i].angle_radians = fitted_rotation(&link.fit, 0.0)?.angle_radians;
        }
        state[..self.length].copy_from_slice(&staged[..self.length]);
        Ok(())
    }

    /// Solve a damped least-squares IK target and use the Etalon DCA solver to
    /// integrate real torque-limited hinge accelerations. `target_m` is in the
    /// chain's base frame. Frames cannot exceed the admitted interval or
    /// publish a partial update to caller buffers.
    pub fn advance(
        &self,
        target_m: [f64; 3],
        dt_s: f64,
        state: &mut [DcaChainState],
        motors: &mut [Option<SemanticRotation>],
        torques: &mut [f64],
    ) -> Result<DcaChainStep, Error> {
        let [primary, _] =
            self.advance_constraints(target_m, None, dt_s, state, motors, torques)?;
        Ok(primary)
    }

    /// Track the primary tip and a second authored point on the same terminal
    /// link. Both base-frame points must be reachable simultaneously. The pair
    /// fixes the direction of the segment between them and allows roll about it.
    /// Returns the primary and secondary residuals in that order.
    pub fn advance_two_points(
        &self,
        targets_m: [[f64; 3]; 2],
        dt_s: f64,
        state: &mut [DcaChainState],
        motors: &mut [Option<SemanticRotation>],
        torques: &mut [f64],
    ) -> Result<[DcaChainStep; 2], Error> {
        self.advance_constraints(
            targets_m[0],
            Some(targets_m[1]),
            dt_s,
            state,
            motors,
            torques,
        )
    }

    pub fn validate_target_pair(&self, targets: [[f64; 3]; 2]) -> Result<(), Error> {
        self.paired_local_tip(targets).map(|_| ())
    }

    fn paired_local_tip(&self, targets: [[f64; 3]; 2]) -> Result<[f64; 3], Error> {
        if !targets.iter().flatten().all(|value| value.is_finite()) {
            return Err(Error::InvalidCommand);
        }
        let point = self.local_secondary_tip.ok_or(Error::MissingFact)?;
        let target_distance = norm(sub(targets[1], targets[0]));
        let separation = norm(sub(point, self.local_tip));
        if !target_distance.is_finite()
            || (target_distance - separation).abs() > 2.0 * self.tolerance
        {
            return Err(Error::UnreachableTarget);
        }
        Ok(point)
    }

    fn advance_constraints(
        &self,
        target_m: [f64; 3],
        secondary_m: Option<[f64; 3]>,
        dt_s: f64,
        state: &mut [DcaChainState],
        motors: &mut [Option<SemanticRotation>],
        torques: &mut [f64],
    ) -> Result<[DcaChainStep; 2], Error> {
        let n = self.length;
        if state.len() < n || motors.len() < n || torques.len() < n {
            return Err(Error::OutputTooSmall);
        }
        if !dt_s.is_finite()
            || dt_s <= 0.0
            || dt_s > 0.05
            || !target_m.iter().all(|v| v.is_finite())
        {
            return Err(Error::InvalidCommand);
        }
        let (secondary_tip, lanes) = if let Some(target) = secondary_m {
            (Some(self.paired_local_tip([target_m, target])?), 6)
        } else {
            (None, 3)
        };
        let targets = [target_m, secondary_m.unwrap_or(target_m)];
        let mut goals = [0.0; MAX_CHAIN];
        let mut bodies = [DcaBodyNode::default(); MAX_CHAIN];
        for (i, link) in self.links[..n].iter().flatten().enumerate() {
            let current = state[i];
            if !current.angle_radians.is_finite()
                || !current.velocity_radians_s.is_finite()
                || fitted_rotation(&link.fit, current.angle_radians)?.angle_radians
                    != current.angle_radians
            {
                return Err(Error::InvalidCommand);
            }
            goals[i] = current.angle_radians;
        }
        let mut jacobian = [[0.0; 6]; MAX_CHAIN];
        let residual_at = |angles: &[f64], jacobian: &mut [[f64; 6]; MAX_CHAIN]| {
            let tips = self.forward(angles, secondary_tip, jacobian)?;
            Ok::<_, Error>([sub(targets[0], tips[0]), sub(targets[1], tips[1])])
        };
        for _ in 0..64 {
            let residual = residual_at(&goals[..n], &mut jacobian)?;
            if residual[..lanes / 3]
                .iter()
                .all(|&point| norm(point) <= self.tolerance)
            {
                break;
            }
            let mut system = [[0.0; 6]; 6];
            for (r, row) in system[..lanes].iter_mut().enumerate() {
                for (c, dst) in row[..lanes].iter_mut().enumerate() {
                    *dst = jacobian[..n]
                        .iter()
                        .map(|axis| axis[r] * axis[c])
                        .sum::<f64>();
                    if r == c {
                        *dst += self.damping * self.damping;
                    }
                }
            }
            let correction = if lanes == 3 {
                let system = std::array::from_fn(|r| std::array::from_fn(|c| system[r][c]));
                linear3(system, residual[0]).map(|[x, y, z]| [x, y, z, 0.0, 0.0, 0.0])
            } else {
                linear6_positive_definite(system, std::array::from_fn(|i| residual[i / 3][i % 3]))
            }
            .ok_or(Error::UnreachableTarget)?;
            for (i, link) in self.links[..n].iter().flatten().enumerate() {
                let step = dot(&jacobian[i][..lanes], &correction[..lanes]).clamp(-0.25, 0.25);
                goals[i] = fitted_rotation(&link.fit, goals[i] + step)?.angle_radians;
            }
        }
        let ik_residual_m = residual_at(&goals[..n], &mut jacobian)?.map(norm);
        if ik_residual_m[..lanes / 3]
            .iter()
            .any(|&error| !error.is_finite() || error > self.tolerance)
        {
            return Err(Error::UnreachableTarget);
        }
        let mut staged_torques = [0.0; MAX_CHAIN];
        for (i, link) in self.links[..n].iter().flatten().enumerate() {
            let current = state[i];
            let tau =
                link.kp * (goals[i] - current.angle_radians) - link.kd * current.velocity_radians_s;
            if !tau.is_finite() {
                return Err(Error::InvalidCommand);
            }
            staged_torques[i] = tau.clamp(-link.torque_limit, link.torque_limit);
            bodies[i] = DcaBodyNode {
                q: current.angle_radians,
                qd: current.velocity_radians_s,
                tau: staged_torques[i],
                ..link.body
            };
        }
        let mut acceleration = [0.0; MAX_CHAIN];
        solve_dca_parallel_chain_zero_heap(&bodies[..n], self.gravity, &mut acceleration[..n])
            .map_err(|_| Error::InvalidCommand)?;
        let mut staged_state = [DcaChainState::default(); MAX_CHAIN];
        let mut staged_motors = [None; MAX_CHAIN];
        for (i, link) in self.links[..n].iter().flatten().enumerate() {
            let velocity = state[i].velocity_radians_s + acceleration[i] * dt_s;
            let desired = state[i].angle_radians + velocity * dt_s;
            if !desired.is_finite() {
                return Err(Error::InvalidCommand);
            }
            let rotation = fitted_rotation(&link.fit, desired)?;
            staged_state[i] = DcaChainState {
                angle_radians: rotation.angle_radians,
                velocity_radians_s: if rotation.angle_radians == desired {
                    velocity
                } else {
                    0.0
                },
            };
            goals[i] = rotation.angle_radians;
            staged_motors[i] = Some(rotation);
        }
        let tracking_residual_m = residual_at(&goals[..n], &mut jacobian)?.map(norm);
        if tracking_residual_m[..lanes / 3]
            .iter()
            .any(|error| !error.is_finite())
        {
            return Err(Error::InvalidCommand);
        }
        state[..n].copy_from_slice(&staged_state[..n]);
        motors[..n].copy_from_slice(&staged_motors[..n]);
        torques[..n].copy_from_slice(&staged_torques[..n]);
        Ok(std::array::from_fn(|i| DcaChainStep {
            tracking_residual_m: tracking_residual_m[i],
            ik_residual_m: ik_residual_m[i],
        }))
    }

    fn forward(
        &self,
        angles: &[f64],
        secondary_tip: Option<[f64; 3]>,
        jacobian: &mut [[f64; 6]; MAX_CHAIN],
    ) -> Result<[[f64; 3]; 2], Error> {
        let mut frame = Motor::identity();
        let mut origins = [[0.0; 3]; MAX_CHAIN];
        let mut axes = [[0.0; 3]; MAX_CHAIN];
        for (i, link) in self.links[..self.length].iter().flatten().enumerate() {
            let bind = Motor::from_rotation_translation(
                link.body.parent_to_joint_rot_wxyz,
                link.body.parent_to_joint_origin_m,
            );
            frame = frame.mul(&bind);
            origins[i] = frame.transform_point([0.0; 3]);
            axes[i] = sub(
                frame.transform_point([
                    link.body.joint_axis[0],
                    link.body.joint_axis[1],
                    link.body.joint_axis[2],
                ]),
                origins[i],
            );
            frame = frame.mul(&fitted_rotation(&link.fit, angles[i])?.motor);
        }
        let tip = frame.transform_point(self.local_tip);
        let marker = secondary_tip.map(|point| frame.transform_point(point));
        for i in 0..self.length {
            jacobian[i][..3].copy_from_slice(&cross(axes[i], sub(tip, origins[i])));
            if let Some(marker) = marker {
                jacobian[i][3..].copy_from_slice(&cross(axes[i], sub(marker, origins[i])));
            }
        }
        Ok([tip, marker.unwrap_or(tip)])
    }
}

fn linear3(matrix: [[f64; 3]; 3], rhs: [f64; 3]) -> Option<[f64; 3]> {
    let columns: [[f64; 3]; 3] = std::array::from_fn(|c| std::array::from_fn(|r| matrix[r][c]));
    let [a, b, c] = columns;
    let determinant = dot(&a, &cross(b, c));
    if !determinant.is_finite() || determinant <= 0.0 {
        return None;
    }
    let result = [
        dot(&rhs, &cross(b, c)),
        dot(&a, &cross(rhs, c)),
        dot(&a, &cross(b, rhs)),
    ]
    .map(|x| x / determinant);
    result.iter().all(|x| x.is_finite()).then_some(result)
}

fn linear6_positive_definite(matrix: [[f64; 6]; 6], rhs: [f64; 6]) -> Option<[f64; 6]> {
    // Factor the damped J·Jᵀ as L·Lᵀ, then solve the two triangular systems.
    let mut lower = [[0.0; 6]; 6];
    let mut forward = [0.0; 6];
    for row in 0..6 {
        for col in 0..=row {
            let product = dot(&lower[row][..col], &lower[col][..col]);
            let value = matrix[row][col] - product;
            if row == col {
                if !value.is_finite() || value <= 0.0 {
                    return None;
                }
                lower[row][col] = value.sqrt();
            } else {
                lower[row][col] = value / lower[col][col];
            }
        }
        forward[row] = (rhs[row] - dot(&lower[row][..row], &forward[..row])) / lower[row][row];
    }
    let mut solution = [0.0; 6];
    for row in (0..6).rev() {
        let product = ((row + 1)..6)
            .map(|col| lower[col][row] * solution[col])
            .sum::<f64>();
        solution[row] = (forward[row] - product) / lower[row][row];
    }
    solution
        .iter()
        .all(|value| value.is_finite())
        .then_some(solution)
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn norm(a: [f64; 3]) -> f64 {
    dot(&a, &a).sqrt()
}
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serial_dca_holds_a_rest_target_and_refuses_an_unreachable_target_atomically() {
        let mut chain = PreparedDcaChain {
            links: [None; MAX_CHAIN],
            length: 2,
            gravity: [0.0; 3],
            local_tip: [1.0, 0.0, 0.0],
            local_secondary_tip: None,
            tolerance: 1e-5,
            damping: 1e-3,
        };
        for i in 0..2 {
            let body = DcaBodyNode {
                parent: i.checked_sub(1).map(|v| v as u16),
                parent_to_joint_origin_m: [i as f64, 0.0, 0.0],
                ..Default::default()
            };
            chain.links[i] = Some(Link {
                body,
                fit: FittedRotation {
                    semantic_id: i as u64 + 1,
                    local_axis: [0.0, 0.0, 1.0],
                    local_pivot_m: [0.0; 3],
                    rom_radians: [-1.0, 1.0],
                    contact_safe_radians: [-0.8, 0.8],
                },
                kp: 4.0,
                kd: 0.5,
                torque_limit: 10.0,
            });
        }
        let mut state = [DcaChainState::default(); 2];
        let mut motors = [None; 2];
        let mut torque = [0.0; 2];
        chain.initialize_rest_state(&mut state).unwrap();
        let step = chain
            .advance([2.0, 0.0, 0.0], 0.01, &mut state, &mut motors, &mut torque)
            .unwrap();
        assert!(step.tracking_residual_m < 1e-10);
        assert_eq!(torque, [0.0; 2]);
        assert!(motors.iter().all(Option::is_some));
        assert!(matches!(
            chain.advance([4.0, 0.0, 0.0], 0.01, &mut state, &mut motors, &mut torque),
            Err(Error::UnreachableTarget)
        ));
        assert_eq!(motors[0].unwrap().angle_radians, 0.0);
    }
}
