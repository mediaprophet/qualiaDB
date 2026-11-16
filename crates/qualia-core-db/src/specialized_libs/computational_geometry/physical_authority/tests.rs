use super::*;
use crate::container_10d::correspondence_section::VertexTetBinding;
use crate::specialized_libs::computational_geometry::geometry_workspace::production_budget::{
    BudgetError, BufferClass, CapacityPlan, ProductionBudget,
};
use crate::specialized_libs::computational_geometry::geometry_workspace::Cancellation;
use crate::specialized_libs::computational_geometry::pbd_softbody::PbdConstraint;
use crate::specialized_libs::computational_geometry::primitives::Point3;
use crate::specialized_libs::computational_geometry::production_contracts::{
    ContentId, EntityDomain, FieldSemantics, MeasurementClass, MetricValue, NamedMetric,
    ReasonCode, RepresentationRole, ResourceEncoding, ResourceKind, ResourceRef, ResultStatus,
    TimingSource, TransferContract, TransferDirection, TransferRule, ValidationReceipt,
    ValidationStage,
};

fn id(bytes: &[u8]) -> ContentId {
    ContentId::sha256(bytes)
}

fn budget<'a>(
    cancel: &'a Cancellation,
    plan: CapacityPlan,
    work: u64,
) -> ProductionBudget<'a> {
    ProductionBudget::admit(plan, usize::MAX, work, cancel).unwrap()
}

fn fixture_plan(solver_n: usize, visual_n: usize, tets: usize) -> CapacityPlan {
    let mut plan = CapacityPlan::default();
    plan.add_slice::<Point3>(BufferClass::Input, solver_n).unwrap();
    plan.add_slice::<Point3>(BufferClass::Scratch, solver_n).unwrap();
    plan.add_slice::<[f64; 3]>(BufferClass::Output, visual_n.max(solver_n))
        .unwrap();
    plan.add_slice::<[f64; 3]>(BufferClass::Scratch, visual_n.max(solver_n))
        .unwrap();
    plan.add_slice::<[u32; 4]>(BufferClass::Input, tets).unwrap();
    plan.add_slice::<VertexTetBinding>(BufferClass::Input, visual_n)
        .unwrap();
    plan.add_slice::<PbdConstraint>(BufferClass::Scratch, tets.max(1))
        .unwrap();
    plan
}

/// Connected two-tet bipyramid. Visual includes identity corners plus one face centroid.
struct ConnectedFixture {
    solver: MeshAuthority,
    visual: MeshAuthority,
    positions: [Point3; 5],
    tets: [[u32; 4]; 2],
    bindings: [VertexTetBinding; 6],
    visual_rest: [[f64; 3]; 6],
}

fn connected_fixture() -> ConnectedFixture {
    let positions = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.5, 0.8660254037844386, 0.0),
        Point3::new(0.5, 0.2886751345948129, 0.8),
        Point3::new(0.5, 0.2886751345948129, -0.8),
    ];
    let tets = [[0, 1, 2, 3], [0, 2, 1, 4]];
    let n = [0.0, 0.0, 1.0];
    let bindings = [
        VertexTetBinding::from_barycentric(0, [1.0, 0.0, 0.0, 0.0], n),
        VertexTetBinding::from_barycentric(0, [0.0, 1.0, 0.0, 0.0], n),
        VertexTetBinding::from_barycentric(0, [0.0, 0.0, 1.0, 0.0], n),
        VertexTetBinding::from_barycentric(0, [0.0, 0.0, 0.0, 1.0], n),
        VertexTetBinding::from_barycentric(1, [0.0, 0.0, 0.0, 1.0], n),
        VertexTetBinding::from_barycentric(0, [1.0 / 3.0, 1.0 / 3.0, 0.0, 1.0 / 3.0], n),
    ];
    let visual_rest = [
        [positions[0].x, positions[0].y, positions[0].z],
        [positions[1].x, positions[1].y, positions[1].z],
        [positions[2].x, positions[2].y, positions[2].z],
        [positions[3].x, positions[3].y, positions[3].z],
        [positions[4].x, positions[4].y, positions[4].z],
        [
            (positions[0].x + positions[1].x + positions[3].x) / 3.0,
            (positions[0].y + positions[1].y + positions[3].y) / 3.0,
            (positions[0].z + positions[1].z + positions[3].z) / 3.0,
        ],
    ];
    ConnectedFixture {
        solver: MeshAuthority {
            identity: id(b"p10-solver-bipyramid"),
            role: RepresentationRole::VolumetricSolver,
            vertex_count: 5,
            cell_or_face_count: 2,
            topology_generation: 1,
        },
        visual: MeshAuthority {
            identity: id(b"p10-visual-bipyramid-lod0"),
            role: RepresentationRole::VisualSurface,
            vertex_count: 6,
            cell_or_face_count: 6,
            topology_generation: 1,
        },
        positions,
        tets,
        bindings,
        visual_rest,
    }
}

fn authority_of(fx: &ConnectedFixture) -> AuthorityDeclaration {
    AuthorityDeclaration {
        solver_mesh: fx.solver,
        visual_mesh: fx.visual,
        visual_lod_identity: None,
    }
}

fn plan_of<'a>(fx: &'a ConnectedFixture) -> FemcTransferPlan<'a> {
    FemcTransferPlan {
        solver: fx.solver,
        visual: fx.visual,
        bindings: &fx.bindings,
        tets: &fx.tets,
    }
}

fn single_tet() -> ([Point3; 4], [u32; 4]) {
    (
        [
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            Point3::new(0.0, 0.0, 1.0),
        ],
        [0, 1, 2, 3],
    )
}

#[test]
fn units_reject_inconsistent_force_scale() {
    let mut u = UnitsDeclaration::SI;
    u.force_unit_newtons = 2.0;
    assert_eq!(
        u.validate(),
        Err(PhysicalError::InvalidInput("force_unit_inconsistent"))
    );
    assert!(UnitsDeclaration::SI.validate().is_ok());
}

#[test]
fn nonzero_xpbd_compliance_is_unsupported() {
    let c = ConstitutiveDeclaration {
        family: ConstitutiveFamily::PbdDistanceAndVolume,
        density_kg_per_m3: 1000.0,
        volume_compliance: 1.0e-8,
        distance_compliance: 0.0,
    };
    assert_eq!(
        c.validate(),
        Err(PhysicalError::Unsupported("xpbd_compliance"))
    );
}

#[test]
fn visual_lod_cannot_replace_solver_topology() {
    let fx = connected_fixture();
    let before = authority_of(&fx);
    before.validate().unwrap();
    let lod = MeshAuthority {
        identity: id(b"p10-visual-bipyramid-lod1"),
        role: RepresentationRole::VisualSurface,
        vertex_count: 5,
        cell_or_face_count: 5,
        topology_generation: 2,
    };
    let after = before.with_visual_lod(lod).unwrap();
    let ev = lod_topology_evidence(&before, &after);
    assert_eq!(ev.status, ResultStatus::Passed);
    assert_eq!(after.solver_mesh, before.solver_mesh);
    assert_ne!(after.visual_mesh.identity, before.visual_mesh.identity);
    assert_ne!(after.visual_mesh.vertex_count, before.visual_mesh.vertex_count);

    let hijack = MeshAuthority {
        identity: before.solver_mesh.identity,
        role: RepresentationRole::VisualSurface,
        vertex_count: 4,
        cell_or_face_count: 4,
        topology_generation: 9,
    };
    assert_eq!(
        before.with_visual_lod(hijack),
        Err(PhysicalError::InvalidInput("lod_replaced_solver_identity"))
    );

    let mut mutated = after;
    mutated.solver_mesh.vertex_count = 3;
    let failed = lod_topology_evidence(&before, &mutated);
    assert_eq!(failed.status, ResultStatus::Failed);
}

#[test]
fn femc_bindings_sum_to_65535_and_rest_state_transfers() {
    let fx = connected_fixture();
    for b in &fx.bindings {
        let sum: u32 = b.barycentric_unorm16.iter().map(|w| *w as u32).sum();
        assert_eq!(sum, 65_535);
    }
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, fixture_plan(5, 6, 2), 50_000);
    let plan = plan_of(&fx);
    let mut predicted = [[0.0; 3]; 6];
    let mut scratch = [[0.0; 3]; 6];
    interpolate_visual_positions(&plan, &fx.positions, &mut predicted, &mut scratch, &mut budget)
        .unwrap();
    let ev = rest_or_deformed_transfer_evidence(
        ClaimId::RestStateTransfer,
        &predicted,
        &fx.visual_rest,
        femc_position_tolerance(UnitsDeclaration::SI),
    )
    .unwrap();
    assert_eq!(ev.status, ResultStatus::Passed, "rest error {:?}", ev.value);
}

#[test]
fn deformed_state_transfers_after_connected_volume_step() {
    let fx = connected_fixture();
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, fixture_plan(5, 6, 2), 200_000);
    let mut constraints = [PbdConstraint::Distance {
        a: 0,
        b: 1,
        rest: 1.0,
    }; 2];
    fill_connected_volume_constraints(&fx.tets, &fx.positions, &mut constraints, &mut budget)
        .unwrap();
    let mut positions = fx.positions;
    positions[3].z = 0.4;
    positions[4].z = -0.4;
    let mut previous = positions;
    let inv = [0.0, 0.0, 0.0, 1.0, 1.0];
    let constitutive = ConstitutiveDeclaration {
        family: ConstitutiveFamily::PbdDistanceAndVolume,
        density_kg_per_m3: 1000.0,
        volume_compliance: 0.0,
        distance_compliance: 0.0,
    };
    let time = TimeDeclaration {
        integrator: IntegratorKind::ConnectedGaussSeidelPbd,
        timestep_seconds: 1.0 / 64.0,
        substeps: 1,
        iterations_per_substep: 24,
        solve_tolerance: 1.0e-6,
        damping: 1.0,
    };
    let params = StepParams::from_time(time, Point3::new(0.0, 0.0, 0.0)).unwrap();
    step_scoped_kernel(
        constitutive,
        time,
        params,
        &mut positions,
        &mut previous,
        &inv,
        &constraints,
        &mut budget,
    )
    .unwrap();

    let plan = plan_of(&fx);
    let mut predicted = [[0.0; 3]; 6];
    let mut scratch = [[0.0; 3]; 6];
    interpolate_visual_positions(&plan, &positions, &mut predicted, &mut scratch, &mut budget)
        .unwrap();
    for i in 0..5 {
        let p = positions[i];
        assert!((predicted[i][0] - p.x).abs() < 1.0e-12);
        assert!((predicted[i][1] - p.y).abs() < 1.0e-12);
        assert!((predicted[i][2] - p.z).abs() < 1.0e-12);
    }
    let mut observed = [[0.0; 3]; 6];
    for i in 0..5 {
        observed[i] = [positions[i].x, positions[i].y, positions[i].z];
    }
    observed[5] = [
        (positions[0].x + positions[1].x + positions[3].x) / 3.0,
        (positions[0].y + positions[1].y + positions[3].y) / 3.0,
        (positions[0].z + positions[1].z + positions[3].z) / 3.0,
    ];
    let ev = rest_or_deformed_transfer_evidence(
        ClaimId::DeformedStateTransfer,
        &predicted,
        &observed,
        femc_position_tolerance(UnitsDeclaration::SI),
    )
    .unwrap();
    assert_eq!(ev.status, ResultStatus::Passed, "deformed {:?}", ev.value);

    let mut tet_f32 = [[0.0f32; 3]; 5];
    let mut vis_f32 = [[0.0f32; 3]; 6];
    let mut scratch_f32 = [[0.0f32; 3]; 6];
    copy_points_f32(&positions, &mut tet_f32).unwrap();
    interpolate_visual_femc_f32(&plan, &tet_f32, &mut vis_f32, &mut scratch_f32, &mut budget)
        .unwrap();
    for i in 0..6 {
        observed[i] = [vis_f32[i][0] as f64, vis_f32[i][1] as f64, vis_f32[i][2] as f64];
    }
    let codec = rest_or_deformed_transfer_evidence(
        ClaimId::DeformedStateTransfer,
        &predicted,
        &observed,
        femc_position_tolerance(UnitsDeclaration::SI),
    )
    .unwrap();
    assert_eq!(codec.status, ResultStatus::Passed, "codec {:?}", codec.value);

    let sample = post_step_positive_volume_sample(&positions, &fx.tets).unwrap();
    assert_eq!(sample.status, ResultStatus::Passed);
    let inversion = evaluate_claim(
        IntegratorKind::ConnectedGaussSeidelPbd,
        ClaimId::InversionFreeTrajectory,
        ResultStatus::Passed,
    );
    assert_eq!(inversion.status, ResultStatus::Failed);
}

#[test]
fn virtual_work_transpose_identity_on_connected_fixture() {
    let fx = connected_fixture();
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, fixture_plan(5, 6, 2), 50_000);
    let plan = plan_of(&fx);
    let visual_forces = [
        [0.1, 0.0, 0.0],
        [0.0, 0.2, 0.0],
        [0.0, 0.0, 0.3],
        [0.4, 0.1, 0.0],
        [0.0, 0.5, 0.1],
        [0.2, 0.2, 0.2],
    ];
    let mut solver_u = [[0.0; 3]; 5];
    for (i, p) in fx.positions.iter().enumerate() {
        solver_u[i] = [0.01 * p.x, -0.02 * p.y, 0.03 * p.z + 0.001 * i as f64];
    }
    let solver_points: [Point3; 5] = core::array::from_fn(|i| {
        Point3::new(solver_u[i][0], solver_u[i][1], solver_u[i][2])
    });
    let mut visual_u = [[0.0; 3]; 6];
    let mut scratch_u = [[0.0; 3]; 6];
    let mut entries = [FemcMapEntry {
        visual_vertex: 0,
        solver_vertex: 0,
        weight: 0.0,
    }; 24];
    let nnz = femc_to_map_entries(&plan, &mut entries, &mut budget).unwrap();
    assert!(nnz >= 6);
    interpolate_visual_positions(&plan, &solver_points, &mut visual_u, &mut scratch_u, &mut budget)
        .unwrap();
    let mut solver_f = [[0.0; 3]; 5];
    let mut scratch_f = [[0.0; 3]; 5];
    scatter_visual_forces_to_solver(&plan, &visual_forces, &mut solver_f, &mut scratch_f, &mut budget)
        .unwrap();
    let residual = virtual_work_residual(&visual_forces, &visual_u, &solver_f, &solver_u).unwrap();
    let mut scale = 0.0;
    for i in 0..6 {
        scale += visual_forces[i][0] * visual_u[i][0]
            + visual_forces[i][1] * visual_u[i][1]
            + visual_forces[i][2] * visual_u[i][2];
    }
    let ev = virtual_work_evidence(residual, scale.abs().max(1.0e-18), 1.0e-12, 1.0e-12).unwrap();
    assert_eq!(ev.status, ResultStatus::Passed, "W residual={residual}");
}

#[test]
fn prescribed_response_restores_single_tet_height() {
    let (rest, tet) = single_tet();
    let rest_volume = tet_signed_volume(rest[0], rest[1], rest[2], rest[3]);
    let rest_height = rest[3].z;
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, fixture_plan(4, 4, 1), 100_000);
    let mut constraints = [PbdConstraint::Volume {
        a: 0,
        b: 1,
        c: 2,
        d: 3,
        rest: rest_volume,
    }];
    fill_connected_volume_constraints(&[tet], &rest, &mut constraints, &mut budget).unwrap();
    let mut positions = rest;
    positions[3].z = 0.45;
    let mut previous = positions;
    let inv = [0.0, 0.0, 0.0, 1.0];
    let constitutive = ConstitutiveDeclaration {
        family: ConstitutiveFamily::PbdDistanceAndVolume,
        density_kg_per_m3: 1000.0,
        volume_compliance: 0.0,
        distance_compliance: 0.0,
    };
    let time = TimeDeclaration {
        integrator: IntegratorKind::ConnectedGaussSeidelPbd,
        timestep_seconds: 1.0 / 64.0,
        substeps: 1,
        iterations_per_substep: 32,
        solve_tolerance: 1.0e-8,
        damping: 1.0,
    };
    step_scoped_kernel(
        constitutive,
        time,
        StepParams::from_time(time, Point3::new(0.0, 0.0, 0.0)).unwrap(),
        &mut positions,
        &mut previous,
        &inv,
        &constraints,
        &mut budget,
    )
    .unwrap();
    let volume = tet_signed_volume(positions[0], positions[1], positions[2], positions[3]);
    let height = positions[3].z;
    let vol_ev = prescribed_response_evidence(volume, rest_volume, 0.01, 0.05).unwrap();
    let h_ev = prescribed_response_evidence(height, rest_height, 0.01, 0.05).unwrap();
    assert_eq!(vol_ev.status, ResultStatus::Passed, "volume rel {:?}", vol_ev.value);
    assert_eq!(h_ev.status, ResultStatus::Passed, "height rel {:?}", h_ev.value);
    assert_eq!(positions[0], rest[0]);
    assert_eq!(positions[1], rest[1]);
    assert_eq!(positions[2], rest[2]);
}

#[test]
fn independent_tetrahedron_kernel_is_not_a_connected_solver() {
    let fx = connected_fixture();
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, fixture_plan(8, 6, 2), 200_000);
    let mut rest_volumes = [0.0; 2];
    for (i, tet) in fx.tets.iter().enumerate() {
        let pts = tet_points(&fx.positions, *tet).unwrap();
        rest_volumes[i] = tet_signed_volume(pts[0], pts[1], pts[2], pts[3]);
    }
    let mut constraints = [PbdConstraint::Volume {
        a: 0,
        b: 1,
        c: 2,
        d: 3,
        rest: rest_volumes[0],
    }; 2];
    fill_independent_volume_constraints(&rest_volumes, &mut constraints, &mut budget).unwrap();

    let mut positions = [Point3::new(0.0, 0.0, 0.0); 8];
    for t in 0..2 {
        let pts = tet_points(&fx.positions, fx.tets[t]).unwrap();
        positions[t * 4..t * 4 + 4].copy_from_slice(&pts);
    }
    // Asymmetric compression: only the first tet's apex is moved.
    positions[3].z *= 0.4;
    let mut previous = positions;
    let inv = [1.0; 8];
    let constitutive = ConstitutiveDeclaration {
        family: ConstitutiveFamily::IndependentTetrahedronPbdVolume,
        density_kg_per_m3: 1000.0,
        volume_compliance: 0.0,
        distance_compliance: 0.0,
    };
    let time = TimeDeclaration {
        integrator: IntegratorKind::IndependentTetrahedronPbd,
        timestep_seconds: 1.0 / 64.0,
        substeps: 1,
        iterations_per_substep: 16,
        solve_tolerance: 1.0e-6,
        damping: 1.0,
    };
    step_scoped_kernel(
        constitutive,
        time,
        StepParams::from_time(time, Point3::new(0.0, 0.0, 0.0)).unwrap(),
        &mut positions,
        &mut previous,
        &inv,
        &constraints,
        &mut budget,
    )
    .unwrap();

    let dx = positions[0].x - positions[4].x;
    let dy = positions[0].y - positions[4].y;
    let dz = positions[0].z - positions[4].z;
    let split = (dx * dx + dy * dy + dz * dz).sqrt();
    assert!(
        split > 1.0e-8,
        "independent tets must be allowed to split duplicated shared vertices, got {split}"
    );

    let label = kernel_label(IntegratorKind::IndependentTetrahedronPbd);
    assert!(advertised_label_is_lawful(label));
    assert!(!label.advertised_as.to_ascii_lowercase().contains("race-free"));
    assert!(!label.advertised_as.to_ascii_lowercase().contains("connected global"));
    assert!(!label
        .advertised_as
        .to_ascii_lowercase()
        .contains("universal collision"));
    let overclaim = evaluate_claim(
        IntegratorKind::IndependentTetrahedronPbd,
        ClaimId::RaceFreeConnectedGlobalSolve,
        ResultStatus::Passed,
    );
    assert_eq!(overclaim.status, ResultStatus::Failed);
    let collision = evaluate_claim(
        IntegratorKind::IndependentTetrahedronPbd,
        ClaimId::UniversalCollisionGuarantee,
        ResultStatus::Passed,
    );
    assert_eq!(collision.status, ResultStatus::Failed);
}

#[test]
fn contact_friction_and_global_coupling_are_unsupported() {
    for claim in UNSUPPORTED_COUPLING_CLAIMS {
        let ev = evidence_unsupported(claim);
        assert_eq!(ev.status, ResultStatus::Unsupported);
        let over = evaluate_claim(
            IntegratorKind::ConnectedGaussSeidelPbd,
            claim,
            ResultStatus::Passed,
        );
        assert_eq!(over.status, ResultStatus::Failed);
    }
    assert_eq!(
        LoadDeclaration {
            kind: LoadKind::ContactUnsupported,
        }
        .validate(),
        Err(PhysicalError::Unsupported("contact_load"))
    );
    assert_eq!(
        LoadDeclaration {
            kind: LoadKind::FrictionUnsupported,
        }
        .validate(),
        Err(PhysicalError::Unsupported("friction_load"))
    );
    assert_eq!(
        transfer_visual_displacement_to_solver(),
        PhysicalError::Unsupported("femc_is_surface_to_tetra_only")
    );
    match (GroundPlaneTreatment::KinematicZClamp { ground_z: 0.0 }) {
        GroundPlaneTreatment::KinematicZClamp { .. } => {}
        GroundPlaneTreatment::Disabled => panic!("expected kinematic clamp variant"),
    }
}

#[test]
fn femc_contracts_and_receipt_statuses_are_explicit() {
    let solver = id(b"solver");
    let visual = id(b"visual");
    let support = ResourceRef {
        id: id(b"femc-bytes"),
        kind: ResourceKind::FemcBindings,
        encoding: ResourceEncoding::FemcV1,
        byte_len: 16,
        element_count: Some(1),
    };
    let displacement = TransferContract {
        source: solver,
        target: visual,
        direction: TransferDirection::SourceToTarget,
        source_dofs: 3,
        target_dofs: 3,
        construction_method: "femc barycentric interpolation".into(),
        domain: EntityDomain::Vertex,
        semantics: FieldSemantics::Displacement,
        units: "m".into(),
        source_frame: id(b"frame"),
        target_frame: id(b"frame"),
        rule: TransferRule::Interpolation,
        support: support.clone(),
        maximum_nonzeros: 4,
        quantisation: "unorm16-barycentric".into(),
        domain_restrictions: vec![],
        validation: vec![],
        errors: vec![],
    };
    validate_femc_displacement_contract(&displacement, solver, visual).unwrap();
    let force = TransferContract {
        source: visual,
        target: solver,
        semantics: FieldSemantics::Force,
        rule: TransferRule::JacobianTranspose,
        construction_method: "femc jacobian transpose".into(),
        ..displacement.clone()
    };
    validate_femc_force_contract(&force, visual, solver).unwrap();
    assert_eq!(
        validate_femc_displacement_contract(&force, solver, visual),
        Err(PhysicalError::IdentityMismatch)
    );

    let receipt = ValidationReceipt {
        targets: vec![solver, visual],
        stage: ValidationStage::OracleVerified,
        fixtures: vec![],
        seeds: vec![10],
        methods: vec!["virtual_work_transpose".into()],
        metrics: vec![NamedMetric {
            name: "virtual_work_residual".into(),
            value: MetricValue::Available {
                value: 0.0,
                units: "J".into(),
                class: MeasurementClass::ProofUnderAssumptions,
                method: "serial f64 identity".into(),
            },
        }],
        status: ResultStatus::Passed,
        reason: None,
        explanation: Some("scoped connected fixture only".into()),
        timing_source: TimingSource::NotMeasured,
        resources: vec![],
        provenance: vec![],
        etalon_receipt: None,
    };
    assert_eq!(receipt.status, ResultStatus::Passed);
    assert_eq!(
        evaluate_claim(
            IntegratorKind::ConnectedGaussSeidelPbd,
            ClaimId::ContactFreeTrajectory,
            ResultStatus::Unsupported,
        )
        .status,
        ResultStatus::Unsupported
    );
}

#[test]
fn budget_exhaustion_does_not_publish_visual() {
    let fx = connected_fixture();
    let cancel = Cancellation::new();
    let mut budget = ProductionBudget::admit(CapacityPlan::default(), usize::MAX, 0, &cancel).unwrap();
    let plan = plan_of(&fx);
    let mut out = [[0.0; 3]; 6];
    let mut scratch = [[0.0; 3]; 6];
    out[0] = [7.0, 7.0, 7.0];
    let err = interpolate_visual_positions(&plan, &fx.positions, &mut out, &mut scratch, &mut budget)
        .unwrap_err();
    assert!(matches!(
        err,
        PhysicalError::Budget(BudgetError::WorkLimitExceeded { .. })
    ));
    assert_eq!(out[0], [7.0, 7.0, 7.0]);
    let (status, code) = physical_error_status(err);
    assert_eq!(status, ResultStatus::Failed);
    assert_eq!(code, ReasonCode::CapacityExhausted);
}

#[test]
fn independent_layout_rejects_shared_vertex_constraints() {
    let cancel = Cancellation::new();
    let mut budget = budget(&cancel, fixture_plan(4, 4, 1), 10_000);
    let mut positions = [
        Point3::new(0.0, 0.0, 0.0),
        Point3::new(1.0, 0.0, 0.0),
        Point3::new(0.0, 1.0, 0.0),
        Point3::new(0.0, 0.0, 1.0),
    ];
    let mut previous = positions;
    let inv = [1.0; 4];
    let constraints = [PbdConstraint::Volume {
        a: 0,
        b: 1,
        c: 2,
        d: 3,
        rest: 1.0 / 6.0,
    }];
    // Shared-looking but actually the independent layout for tet 0. Distance is rejected.
    let distance = [PbdConstraint::Distance {
        a: 0,
        b: 1,
        rest: 1.0,
    }];
    let constitutive = ConstitutiveDeclaration {
        family: ConstitutiveFamily::IndependentTetrahedronPbdVolume,
        density_kg_per_m3: 1.0,
        volume_compliance: 0.0,
        distance_compliance: 0.0,
    };
    let time = TimeDeclaration {
        integrator: IntegratorKind::IndependentTetrahedronPbd,
        timestep_seconds: 0.01,
        substeps: 1,
        iterations_per_substep: 1,
        solve_tolerance: 1.0,
        damping: 0.0,
    };
    let params = StepParams::from_time(time, Point3::new(0.0, 0.0, 0.0)).unwrap();
    assert_eq!(
        step_scoped_kernel(
            constitutive,
            time,
            params,
            &mut positions,
            &mut previous,
            &inv,
            &distance,
            &mut budget,
        ),
        Err(PhysicalError::Unsupported(
            "independent_tet_distance_coupling"
        ))
    );
    step_scoped_kernel(
        constitutive,
        time,
        params,
        &mut positions,
        &mut previous,
        &inv,
        &constraints,
        &mut budget,
    )
    .unwrap();
}
