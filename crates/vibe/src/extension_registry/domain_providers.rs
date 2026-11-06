//! Reference deterministic domain provider (`DeterministicMechanicsProvider`, `VS03-08`)
//! with compile-time sealed `NQuin48` ontology records and direct-call oracle helpers.

use vibescript_abi::{
    q_hash_str, BorrowedSliceView, CallerOutputSlice, CanonicalIri, ContractVersionAxes,
    DeterminismGuarantee, EffectSet, FuncDescriptor, NQuin48, OntologicalContract, ParamSpec,
    PrimitiveKind, ProviderDescriptor, ResourceEnvelope, ReversibilityKind, SemVer64, StableFuncId,
    SubNamespaceDescriptor, TargetProfileMask, VibeNamespaceProvider, VibeTrap,
};

const VERLET_PARAMS: &[ParamSpec] = &[
    ParamSpec::slice("pos_xyz", PrimitiveKind::F64, 3, 3),
    ParamSpec::slice("vel_xyz", PrimitiveKind::F64, 3, 3),
    ParamSpec::slice("acc_xyz", PrimitiveKind::F64, 3, 3),
    ParamSpec::scalar("dt", PrimitiveKind::F64),
];

const KE_PARAMS: &[ParamSpec] = &[
    ParamSpec::scalar("mass", PrimitiveKind::F64),
    ParamSpec::slice("vel_xyz", PrimitiveKind::F64, 3, 3),
];

const AABB_PARAMS: &[ParamSpec] = &[
    ParamSpec::slice("min_xyz", PrimitiveKind::F64, 3, 3),
    ParamSpec::slice("max_xyz", PrimitiveKind::F64, 3, 3),
];

const KINEMATICS_FUNCS: &[FuncDescriptor] = &[
    FuncDescriptor {
        name: "verlet_step",
        canonical_iri: CanonicalIri::new(
            "https://qualiadb.org/ext/etalon/mechanics/kinematics/verlet_step",
        ),
        stable_id: StableFuncId::new(0, 0),
        params: VERLET_PARAMS,
        output_kind: PrimitiveKind::F64,
        required_output_elems: 6,
        effects: EffectSet::PURE_COMPUTE,
        determinism: DeterminismGuarantee::DeterministicTolerance,
        reversibility: ReversibilityKind::PureNoState,
        envelope: ResourceEnvelope::hot_zero_heap(32, 48),
        contract: OntologicalContract {
            contract_digest: [0x11; 32],
            schema_hash: q_hash_str("(f64[3],f64[3],f64[3],f64)->f64[6]"),
            precondition_quin_span: (0, 1),
            postcondition_quin_span: (1, 1),
            invariant_quin_span: (2, 1),
        },
    },
    FuncDescriptor {
        name: "kinetic_energy",
        canonical_iri: CanonicalIri::new(
            "https://qualiadb.org/ext/etalon/mechanics/kinematics/kinetic_energy",
        ),
        stable_id: StableFuncId::new(0, 1),
        params: KE_PARAMS,
        output_kind: PrimitiveKind::F64,
        required_output_elems: 1,
        effects: EffectSet::PURE_COMPUTE,
        determinism: DeterminismGuarantee::DeterministicTolerance,
        reversibility: ReversibilityKind::PureNoState,
        envelope: ResourceEnvelope::hot_zero_heap(16, 8),
        contract: OntologicalContract {
            contract_digest: [0x22; 32],
            schema_hash: q_hash_str("(f64,f64[3])->f64"),
            precondition_quin_span: (0, 1),
            postcondition_quin_span: (1, 1),
            invariant_quin_span: (2, 1),
        },
    },
];

const GEOMETRY_FUNCS: &[FuncDescriptor] = &[FuncDescriptor {
    name: "aabb_volume",
    canonical_iri: CanonicalIri::new(
        "https://qualiadb.org/ext/etalon/mechanics/geometry/aabb_volume",
    ),
    stable_id: StableFuncId::new(1, 0),
    params: AABB_PARAMS,
    output_kind: PrimitiveKind::F64,
    required_output_elems: 2,
    effects: EffectSet::PURE_COMPUTE,
    determinism: DeterminismGuarantee::DeterministicTolerance,
    reversibility: ReversibilityKind::PureNoState,
    envelope: ResourceEnvelope::hot_zero_heap(16, 16),
    contract: OntologicalContract {
        contract_digest: [0x33; 32],
        schema_hash: q_hash_str("(f64[3],f64[3])->f64[2]"),
        precondition_quin_span: (0, 1),
        postcondition_quin_span: (1, 1),
        invariant_quin_span: (2, 1),
    },
}];

const MECH_SUB_NAMESPACES: &[SubNamespaceDescriptor] = &[
    SubNamespaceDescriptor {
        name: "kinematics",
        sub_id: 0,
        functions: KINEMATICS_FUNCS,
    },
    SubNamespaceDescriptor {
        name: "geometry",
        sub_id: 1,
        functions: GEOMETRY_FUNCS,
    },
];

const MECH_DESCRIPTOR: ProviderDescriptor = ProviderDescriptor {
    canonical_iri: CanonicalIri::new("https://qualiadb.org/ext/etalon/mechanics"),
    short_alias: "mech",
    versions: ContractVersionAxes::v0_3_default(
        SemVer64::release(0, 4, 2),
        SemVer64::release(0, 1, 0),
    ),
    supported_targets: TargetProfileMask::ALL_CPU,
    sub_namespaces: MECH_SUB_NAMESPACES,
    max_ontology_quins: 16,
};

const MECH_ONTOLOGY_QUINS: &[NQuin48] = &[
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/etalon/mechanics"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPrecondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#FiniteRealInputs"),
        q_hash_str("https://qualiadb.org/context/public"),
        1,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/etalon/mechanics"),
        q_hash_str("https://qualiadb.org/schema/vibe#hasPostcondition"),
        q_hash_str("https://qualiadb.org/schema/vibe#BoundedPhaseSpaceStep"),
        q_hash_str("https://qualiadb.org/context/public"),
        2,
    ),
    NQuin48::from_parts(
        q_hash_str("https://qualiadb.org/ext/etalon/mechanics"),
        q_hash_str("https://qualiadb.org/schema/vibe#preservesInvariant"),
        q_hash_str("https://qualiadb.org/schema/vibe#SymplecticEnergyBound"),
        q_hash_str("https://qualiadb.org/context/public"),
        3,
    ),
];

/// Deterministic mechanics and geometry domain provider (`VS03-08`).
#[derive(Debug, Clone, Copy, Default)]
pub struct DeterministicMechanicsProvider;

impl VibeNamespaceProvider for DeterministicMechanicsProvider {
    fn descriptor(&self) -> &'static ProviderDescriptor {
        &MECH_DESCRIPTOR
    }

    fn static_ontology_nquins(&self) -> &'static [NQuin48] {
        MECH_ONTOLOGY_QUINS
    }

    fn call(
        &self,
        stable_id: StableFuncId,
        inputs: &[BorrowedSliceView<'_>],
        output: &mut CallerOutputSlice<'_>,
    ) -> Result<u64, VibeTrap> {
        match (stable_id.sub_namespace_id, stable_id.func_id) {
            (0, 0) => {
                // kinematics::verlet_step
                let pos = [
                    inputs[0].read_f64_le(0)?,
                    inputs[0].read_f64_le(1)?,
                    inputs[0].read_f64_le(2)?,
                ];
                let vel = [
                    inputs[1].read_f64_le(0)?,
                    inputs[1].read_f64_le(1)?,
                    inputs[1].read_f64_le(2)?,
                ];
                let acc = [
                    inputs[2].read_f64_le(0)?,
                    inputs[2].read_f64_le(1)?,
                    inputs[2].read_f64_le(2)?,
                ];
                let dt = inputs[3].read_f64_le(0)?;
                if !dt.is_finite() || dt < 0.0 {
                    return Err(VibeTrap::NumericalDomainError);
                }
                let (new_pos, new_vel) = oracle_verlet_step(pos, vel, acc, dt);
                for &p in &new_pos {
                    output.push_f64_le(p)?;
                }
                for &v in &new_vel {
                    output.push_f64_le(v)?;
                }
                Ok(18)
            }
            (0, 1) => {
                // kinematics::kinetic_energy
                let mass = inputs[0].read_f64_le(0)?;
                if !mass.is_finite() || mass < 0.0 {
                    return Err(VibeTrap::NumericalDomainError);
                }
                let vel = [
                    inputs[1].read_f64_le(0)?,
                    inputs[1].read_f64_le(1)?,
                    inputs[1].read_f64_le(2)?,
                ];
                let ke = oracle_kinetic_energy(mass, vel);
                output.push_f64_le(ke)?;
                Ok(8)
            }
            (1, 0) => {
                // geometry::aabb_volume
                let min_xyz = [
                    inputs[0].read_f64_le(0)?,
                    inputs[0].read_f64_le(1)?,
                    inputs[0].read_f64_le(2)?,
                ];
                let max_xyz = [
                    inputs[1].read_f64_le(0)?,
                    inputs[1].read_f64_le(1)?,
                    inputs[1].read_f64_le(2)?,
                ];
                let (vol, area) = oracle_aabb_volume(min_xyz, max_xyz)?;
                output.push_f64_le(vol)?;
                output.push_f64_le(area)?;
                Ok(10)
            }
            _ => Err(VibeTrap::FunctionNotFound),
        }
    }
}

/// Direct-call reference oracle for `kinematics::verlet_step`.
#[inline]
pub fn oracle_verlet_step(
    pos: [f64; 3],
    vel: [f64; 3],
    acc: [f64; 3],
    dt: f64,
) -> ([f64; 3], [f64; 3]) {
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

/// Direct-call reference oracle for `kinematics::kinetic_energy`.
#[inline]
pub fn oracle_kinetic_energy(mass: f64, vel: [f64; 3]) -> f64 {
    0.5 * mass * (vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2])
}

/// Direct-call reference oracle for `geometry::aabb_volume`.
#[inline]
pub fn oracle_aabb_volume(min_xyz: [f64; 3], max_xyz: [f64; 3]) -> Result<(f64, f64), VibeTrap> {
    let dx = max_xyz[0] - min_xyz[0];
    let dy = max_xyz[1] - min_xyz[1];
    let dz = max_xyz[2] - min_xyz[2];
    if dx < 0.0 || dy < 0.0 || dz < 0.0 {
        return Err(VibeTrap::NumericalDomainError);
    }
    let vol = dx * dy * dz;
    let area = 2.0 * (dx * dy + dy * dz + dz * dx);
    Ok((vol, area))
}
